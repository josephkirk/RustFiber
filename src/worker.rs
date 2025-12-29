//! Worker thread implementation.
//!
//! Worker threads continuously pull jobs from the queue and execute them.
//! They form the foundation of the M:N threading model, where multiple
//! fibers (jobs) are multiplexed onto a fixed number of worker threads.

use crate::PinningStrategy;
use crate::allocator::paged::PagedFrameAllocator;
use crate::fiber::{AllocatorPtr, FiberInput, FiberState, QueuePtr};
use crate::fiber_pool::FiberPool;
use crate::job::Job;
use core_affinity::CoreId;
use crossbeam::deque::{Injector, Stealer, Worker as Deque};
use crossbeam::utils::{Backoff, CachePadded};
use rand::SeedableRng;
use std::cell::Cell;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::thread::{self, JoinHandle};

/// Error type for worker pool operations.
#[derive(Debug, thiserror::Error)]
pub enum WorkerPoolError {
    /// One or more worker threads failed to join during shutdown.
    #[error("{failed_count} worker thread(s) panicked")]
    WorkerPanic { failed_count: usize },
}

/// Global parking mechanism for worker threads.
/// Uses 1ms timeout polling for efficient work discovery without explicit signaling overhead.
pub(crate) struct GlobalParker {
    lock: std::sync::Mutex<()>,
    cvar: std::sync::Condvar,
    sleepers: AtomicUsize,
}

impl GlobalParker {
    pub(crate) fn new() -> Self {
        Self {
            lock: std::sync::Mutex::new(()),
            cvar: std::sync::Condvar::new(),
            sleepers: AtomicUsize::new(0),
        }
    }

    /// Parks the current thread with a 1ms timeout.
    /// This allows workers to self-discover work without explicit signaling.
    pub(crate) fn park(&self) {
        let guard = self.lock.lock().unwrap();
        self.sleepers.fetch_add(1, Ordering::SeqCst);
        let _result = self.cvar.wait_timeout(guard, std::time::Duration::from_millis(1)).unwrap();
        self.sleepers.fetch_sub(1, Ordering::SeqCst);
    }

    /// Wakes up *one* sleeping thread (if any).
    pub fn wake_one(&self) {
        if self.sleepers.load(Ordering::SeqCst) > 0 {
            self.cvar.notify_one();
        }
    }

    /// Wakes up *all* sleeping threads.
    /// Used during shutdown.
    pub(crate) fn wake_all(&self) {
        self.cvar.notify_all();
    }
}

// Thread-Local Storage for the worker's FrameAllocator
thread_local! {
    static ACTIVE_ALLOCATOR: Cell<Option<*mut PagedFrameAllocator>> = const { Cell::new(None) };
}

/// Helper to temporarily expose the allocator via TLS
struct ScopedAllocator;

impl ScopedAllocator {
    /// Registers the allocator in TLS.
    ///
    /// # Safety
    /// Caller must ensure the allocator outlives this struct and is not accessed uniquely
    /// while accessed via TLS.
    unsafe fn new(ptr: *mut PagedFrameAllocator) -> Self {
        ACTIVE_ALLOCATOR.with(|tls| tls.set(Some(ptr)));
        Self
    }
}

impl Drop for ScopedAllocator {
    fn drop(&mut self) {
        ACTIVE_ALLOCATOR.with(|tls| tls.set(None));
    }
}

/// Executes a closure wih the current thread's frame allocator, if available.
pub fn with_current_allocator<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&mut PagedFrameAllocator) -> R,
{
    ACTIVE_ALLOCATOR.with(|tls| tls.get().map(|ptr| unsafe { f(&mut *ptr) }))
}

/// Returns a raw pointer to the current thread's frame allocator, if active.
pub fn get_current_allocator() -> Option<*mut PagedFrameAllocator> {
    ACTIVE_ALLOCATOR.with(|tls| tls.get())
}

/// A worker thread that executes jobs from a queue.
pub struct Worker {
    id: usize,
    handle: Option<JoinHandle<()>>,
}

/// Parameters for creating a new worker thread.
pub(crate) struct WorkerParams {
    pub(crate) id: usize,
    pub(crate) local_queue: Deque<Job>,
    pub(crate) stealers: Arc<Vec<Stealer<Job>>>,
    pub(crate) injector: Arc<Injector<Job>>,
    pub(crate) high_priority_injector: Arc<Injector<Job>>,
    pub(crate) shutdown: Arc<CachePadded<AtomicBool>>,
    pub(crate) active_workers: Arc<CachePadded<AtomicUsize>>,
    pub(crate) tier: u32,
    pub(crate) threshold: usize,
    pub(crate) core_id: Option<CoreId>,
    pub(crate) strategy: PinningStrategy,
    pub(crate) fiber_config: crate::job_system::FiberConfig,
    pub(crate) frame_index: Arc<CachePadded<AtomicU64>>,
    pub(crate) topology: Arc<crate::topology::Topology>,
    pub(crate) parker: Arc<GlobalParker>,
    pub(crate) has_work: Arc<Vec<CachePadded<AtomicBool>>>,
    #[cfg(feature = "metrics")]
    pub(crate) metrics: Option<Arc<crate::metrics::Metrics>>,
}

thread_local! {
    pub static WORKER_ID: std::cell::Cell<Option<usize>> = const { std::cell::Cell::new(None) };
    /// Thread-local raw pointer to the worker's local queue (Deque).
    /// Accessed unsafely to bypass ownership rules for optimized submission.
    static LOCAL_DEQUE: Cell<Option<*const Deque<Job>>> = const { Cell::new(None) };
}

/// Helper to expose the local deque via TLS
struct ScopedDeque;

impl ScopedDeque {
    /// Registers the deque in TLS.
    ///
    /// # Safety
    /// Caller must ensure the deque outlives this struct and is effectively owned by this thread.
    unsafe fn new(ptr: *const Deque<Job>) -> Self {
        LOCAL_DEQUE.with(|tls| tls.set(Some(ptr)));
        Self
    }
}

impl Drop for ScopedDeque {
    fn drop(&mut self) {
        LOCAL_DEQUE.with(|tls| tls.set(None));
    }
}

impl Worker {
    /// Creates and starts a new worker thread with work-stealing support.
    ///
    /// The worker will continuously pull jobs from its local queue, steal from
    /// other workers when idle, and check the global injector.
    pub(crate) fn new(params: WorkerParams) -> Self {
        let id = params.id;
        let handle = thread::spawn(move || {
            // Pin worker to its core for better cache locality if specified
            if let Some(core_id) = params.core_id {
                core_affinity::set_for_current(core_id);
            }

            Worker::run_loop(params);
        });

        Worker {
            id,
            handle: Some(handle),
        }
    }

    /// Main execution loop for the worker thread with work-stealing.
    fn run_loop(params: WorkerParams) {
        let WorkerParams {
            id: worker_id,
            local_queue,
            stealers,
            injector,
            high_priority_injector,
            shutdown,
            active_workers,
            tier,
            threshold,
            strategy,
            fiber_config,
            frame_index,
            topology,
            parker,
            has_work,
            core_id: _,
            #[cfg(feature = "metrics")]
            metrics,
        } = params;

        WORKER_ID.set(Some(worker_id));

        WORKER_ID.set(Some(worker_id));

        // Initialize FrameAllocator on the thread (First-Touch)
        let mut allocator = PagedFrameAllocator::new(fiber_config.frame_stack_size);

        // SAFETY: Allocator lives as long as the function (and this scope).

        // SAFETY: Allocator lives as long as the function (and this scope).
        // Access via TLS is strictly local to this thread.
        let _scope = unsafe { ScopedAllocator::new(&mut allocator as *mut _) };

        // Register local queue in TLS for optimized submission
        // SAFETY: local_queue lives as long as this function.
        let _deque_scope = unsafe { ScopedDeque::new(&local_queue as *const _) };

        // Initialize FiberPool on the thread (First-Touch)
        // Initialize FiberPool on the thread (First-Touch)
        let mut fiber_pool = FiberPool::new(
            fiber_config.initial_pool_size,
            fiber_config.stack_size,
            fiber_config.prefetch_pages,
        );

        #[cfg(feature = "tracing")]
        let _collector = crate::tracing::CollectorGuard;

        // Compute Steal Order based on Topology
        // 1. Siblings (Same NUMA Node)
        // 2. Others (Remote NUMA Nodes)
        let mut steal_order = Vec::new();

        // Add siblings first
        if let Some(siblings) = topology.get_siblings(worker_id) {
            for &sibling_id in siblings {
                if sibling_id != worker_id && sibling_id < stealers.len() {
                    steal_order.push(sibling_id);
                }
            }
        }

        // Add remaining workers
        for i in 0..stealers.len() {
            if i != worker_id && !steal_order.contains(&i) {
                steal_order.push(i);
            }
        }

        // Rotate steal order to avoid convoy effect (all workers stealing from 0 first)
        // This spreads the stealing load across all workers.
        if !steal_order.is_empty() {
            let rotate_amt = worker_id % steal_order.len();
            steal_order.rotate_left(rotate_amt);
        }

        // Optimization: Flatten the stealing list to avoid double indirection (Vec<usize> -> stealers[idx])
        // and bounds checks during the hot loop. We clone the stealers (lightweight handles) into a local Vec.
        // Load-Aware: We now store (id, stealer) tuple to check status flags.
        let ordered_stealers: Vec<_> = steal_order
            .iter()
            .map(|&idx| (idx, stealers[idx].clone()))
            .collect();

        // Init backoff outside the loop so it persists across iterations
        let backoff = Backoff::new();
        
        // Initialize RNG for randomized back-off
        // We use SmallRng for performance as we might call it frequently in the idle loop
        let mut _rng = rand::rngs::SmallRng::from_rng(&mut rand::rng());


        // Track local frame index to detect changes
        let mut local_frame_index = frame_index.load(Ordering::Relaxed);

        loop {
            // Check for shutdown signal
            if shutdown.load(Ordering::Relaxed) {
                break;
            }

            // Frame Reset Logic
            let global_frame_index = frame_index.load(Ordering::Relaxed);
            if global_frame_index > local_frame_index {
                // New frame detected, reset allocator
                // SAFETY: We assume the user guarantees all jobs from the previous frame are done
                // before incrementing the frame index.
                // SAFETY: We assume the user guarantees all jobs from the previous frame are done
                // before incrementing the frame index.
                /*unsafe {*/
                    allocator.reset();
                /*}*/
                local_frame_index = global_frame_index;
            }

            // Tiered Spillover Logic: Stay dormant if load is below threshold
            if tier > 1 && active_workers.load(Ordering::Relaxed) < threshold && injector.is_empty()
            {
                // park logic handles this naturally now, but explicit check saves mutex 
                parker.park();
                continue;
            }

            // Try to get a job: High Priority -> Local -> Steal (Topology Aware) -> Global
            let job = loop {
                match high_priority_injector.steal() {
                    crossbeam::deque::Steal::Success(job) => {
                        #[cfg(feature = "metrics")]
                        if let Some(metrics) = &metrics {
                            metrics.global_injector_pops.fetch_add(1, Ordering::Relaxed);
                        }
                        break Some(job);
                    }
                    crossbeam::deque::Steal::Retry => continue,
                    crossbeam::deque::Steal::Empty => break None,
                }
            }
            .or_else(|| {
                let j = local_queue.pop();
                #[cfg(feature = "metrics")]
                if let Some(metrics) = &metrics {
                    if j.is_some() {
                        metrics.local_queue_pops.fetch_add(1, Ordering::Relaxed);
                    }
                }
                
                // Load-Aware: If local queue is empty, update flag
                if j.is_none() {
                     has_work[worker_id].store(false, Ordering::Relaxed);
                }
                
                j
            })
            .or_else(|| {
                // Try to steal from other workers using Topology-Aware order
                let mut steal_attempted = false;
                let steal_result = ordered_stealers
                    .iter()
                    .map(|(victim_id, s)| {
                        // Load-Aware: Check if victim has work before attempting steal
                        if !has_work[*victim_id].load(Ordering::Relaxed) {
                            return crossbeam::deque::Steal::Empty;
                        }
                        
                        steal_attempted = true;
                        let result = s.steal_batch_and_pop(&local_queue);
                        #[cfg(feature = "metrics")]
                        if let Some(metrics) = &metrics {
                            match &result {
                                crossbeam::deque::Steal::Success(_) => {
                                    metrics.worker_steals_success.fetch_add(1, Ordering::Relaxed);
                                    metrics.local_queue_pushes.fetch_add(1, Ordering::Relaxed);
                                    // We stole work and populated our queue
                                    has_work[worker_id].store(true, Ordering::Relaxed);
                                    // Note: No wake_one here - 1ms polling handles discovery
                                }
                                crossbeam::deque::Steal::Empty => {
                                    // Stale has_work flag - victim finished before we arrived.
                                    // Not counted as failure since we did check has_work.
                                }
                                crossbeam::deque::Steal::Retry => {
                                    metrics.worker_steals_retry.fetch_add(1, Ordering::Relaxed);
                                }
                            }
                        }
                        result
                    })
                    .find_map(|steal_result| match steal_result {
                        crossbeam::deque::Steal::Success(job) => Some(job),
                        _ => None,
                    });

                // If we attempted steals but got nothing, count as failed attempts
                #[cfg(feature = "metrics")]
                if steal_attempted && steal_result.is_none() && metrics.is_some() {
                    // We already counted individual failures above, so this is redundant
                }

                steal_result
                    .or_else(|| {
                        // If stealing failed, try the global injector
                        let mut retry_count = 0;
                        const MAX_RETRIES: usize = 3;

                        loop {
                            let injector_result = injector.steal_batch_and_pop(&local_queue);
                            #[cfg(feature = "metrics")]
                            if let Some(metrics) = &metrics {
                                match &injector_result {
                                    crossbeam::deque::Steal::Success(_) => {
                                        metrics.injector_steals_success.fetch_add(1, Ordering::Relaxed);
                                        metrics.global_injector_pops.fetch_add(1, Ordering::Relaxed);
                                        metrics.local_queue_pushes.fetch_add(1, Ordering::Relaxed);
                                        // We have work now
                                        has_work[worker_id].store(true, Ordering::Relaxed);
                                        // Note: No wake_one here - 1ms polling handles discovery
                                    }
                                    crossbeam::deque::Steal::Empty => {
                                        // Empty injector is normal during idle - not a failure
                                        break;
                                    }
                                    crossbeam::deque::Steal::Retry => {
                                        metrics.injector_steals_retry.fetch_add(1, Ordering::Relaxed);
                                        retry_count += 1;
                                        if retry_count >= MAX_RETRIES {
                                            // For injector contention, we yield immediately
                                            std::thread::yield_now();
                                            break;
                                        }
                                    }
                                }
                            }

                            match injector_result {
                                crossbeam::deque::Steal::Success(job) => return Some(job),
                                crossbeam::deque::Steal::Empty => break,
                                crossbeam::deque::Steal::Retry => {
                                    retry_count += 1;
                                    if retry_count >= MAX_RETRIES {
                                        // For injector contention, we yield immediately
                                        std::thread::yield_now();
                                        break;
                                    }
                                }
                            }
                        }
                        None
                    })
            });

            match job {
                Some(job) => {
                    // Mark as active before running
                    active_workers.fetch_add(1, Ordering::Relaxed);

                    // We found work, so reset the backoff strategy
                    backoff.reset();

                    let Job {
                        work,
                        counter,
                        priority,
                    } = job;

                    let (mut fiber, input) = match work {
                        crate::job::Work::Resume(handle) => {
                            // Resume suspended fiber
                            // SAFETY: The handle comes from a Box<Fiber> that was leaked into raw pointer
                            // by the worker that suspended it. We reclaim ownership here.
                            let fiber = unsafe { Box::from_raw(handle.0) };

                            // Hazard Protection: Ensure the fiber has actually suspended.
                            // There is a race where a waker (decrement) schedules the fiber BEFORE
                            // the previous owner (worker) has finished the context switch / yield.
                            // We spin until the previous owner sets is_suspended = true.
                            let spin_helper = crossbeam::utils::Backoff::new();
                            while !fiber.is_suspended.load(Ordering::Acquire) {
                                spin_helper.snooze();
                            }

                            // We are now the owner. Mark as running.
                            fiber.is_suspended.store(false, Ordering::Relaxed);

                            (fiber, FiberInput::Resume)
                        }
                        work => {
                            // New job - acquire fiber from pool
                            let mut fiber = fiber_pool.get();

                            // Initialize suspension state for new run
                            fiber.is_suspended.store(false, Ordering::Relaxed);

                            // Reconstruct job (we moved work out)
                            let job = Job {
                                work,
                                counter,
                                 priority,
                            };

                            // Use local queue as scheduler for immediate wakeup locality if strategy permits
                            let scheduler: &dyn crate::counter::JobScheduler = match strategy {
                                crate::PinningStrategy::TieredSpillover => &*injector,
                                _ => &local_queue,
                            };
                            let scheduler_ptr =
                                scheduler as *const dyn crate::counter::JobScheduler;

                            let fiber_ptr = fiber.as_mut() as *mut crate::fiber::Fiber;
                            (
                                fiber,
                                FiberInput::Start(
                                    job,
                                    scheduler_ptr,
                                    fiber_ptr,
                                    Some(AllocatorPtr(&mut allocator)),
                                    Some(QueuePtr(&local_queue)),
                                ),
                            )
                        }
                    };

                    // Run the fiber
                    #[cfg(feature = "tracing")]
                    let _trace_event = crate::tracing::TraceGuard::new("Job Execution", worker_id);
                    let state = fiber.resume(input);

                    match state {
                        FiberState::Complete => {
                            #[cfg(feature = "metrics")]
                            if let Some(metrics) = &metrics {
                                metrics.jobs_completed.fetch_add(1, Ordering::Relaxed);
                            }
                            // Ensure flag is set for any concurrent waiters (though unlikely for Complete)
                            fiber.is_suspended.store(true, Ordering::Release);
                            fiber_pool.return_fiber(fiber);
                        }
                        FiberState::Yielded(reason) => {
                            // Mark as suspended implies we are done touching the stack/coroutine
                            fiber.is_suspended.store(true, Ordering::Release);

                            let raw_fiber = Box::into_raw(fiber);
                            if let crate::fiber::YieldType::Normal = reason {
                                // Reschedule dependent on strategy
                                let handle = crate::fiber::FiberHandle(raw_fiber);
                                let job = Job::resume_job(handle);

                                match strategy {
                                    crate::PinningStrategy::TieredSpillover => {
                                        // Push to global injector to wake up dormant threads
                                        if job.priority == crate::job::JobPriority::High {
                                            #[cfg(feature = "metrics")]
                                            if let Some(metrics) = &metrics {
                                                metrics.global_injector_pushes.fetch_add(1, Ordering::Relaxed);
                                            }
                                            high_priority_injector.push(job);
                                        } else {
                                            #[cfg(feature = "metrics")]
                                            if let Some(metrics) = &metrics {
                                                metrics.global_injector_pushes.fetch_add(1, Ordering::Relaxed);
                                            }
                                            injector.push(job);
                                         }
                                         // Note: No wake_one - 1ms polling handles discovery
                                    }
                                    _ => {
                                        // Keep local for other strategies
                                        #[cfg(feature = "metrics")]
                                        if let Some(metrics) = &metrics {
                                            metrics.local_queue_pushes.fetch_add(1, Ordering::Relaxed);
                                        }
                                        local_queue.push(job);
                                         // Load-Aware: We have new work
                                         has_work[worker_id].store(true, Ordering::Relaxed);
                                         // Note: No wake_one - 1ms polling handles discovery
                                    }
                                }
                            }
                        }
                        FiberState::Panic(err) => {
                            // Ensure flag is set so any concurrent waiters don't deadlock
                            fiber.is_suspended.store(true, Ordering::Release);

                            let msg = if let Some(s) = err.downcast_ref::<&str>() {
                                *s
                            } else if let Some(s) = err.downcast_ref::<String>() {
                                s.as_str()
                            } else {
                                "Unknown panic payload"
                            };
                            eprintln!("Job panicked: {}", msg);
                        }
                    }

                    // Mark as inactive after finishing
                    active_workers.fetch_sub(1, Ordering::Relaxed);
                }
                None => {
                    // No work available
                    // No work available
                    if backoff.is_completed() {
                        // Pre-warming Layer:
                        // If the fiber pool is below the target size, incrementally allocate one fiber.
                        // This allows for "Interleaved Pre-warning" where the pool grows during idle cycles
                        // rather than stalling the thread at startup.
                        if fiber_pool.len() < fiber_config.target_pool_size {
                            fiber_pool.grow(1);
                            // Reset backoff to check for work again immediately (allocation counts as work)
                            backoff.reset();
                            continue;
                        }

                        // Deep Idle: Signal-based Parking
                        // Mitigation for "Thundering Herd": When multiple workers are idle and contending
                        // for the same empty victims, we must desynchronize them.
                        
                        // Parking Strategy:
                        // - Instead of spinning or sleeping blindly, we park the thread (block).
                        // - Threads are woken up by `notify_one` when new work is pushed.
                        // - This eliminates "Empty Attempts" almost entirely as workers only wake when work exists.
                        
                        // Parking Strategy via Semaphore:
                        // - Threads block indefinitely until `wake_one` increments the signal count.
                        // - No polling, no timeout.
                        parker.park();
                        
                        // After waking, we reset backoff to `snooze` level? 
                        // No, if we woke up, we presumably have work. Reset fully.
                        backoff.reset();
                    } else {
                        // Brief Idle: Spin or yield
                        backoff.snooze();
                    }
                }
            }
        }
    }

    /// Returns the worker's ID.
    pub fn id(&self) -> usize {
        self.id
    }

    /// Waits for the worker thread to finish.
    pub fn join(mut self) -> thread::Result<()> {
        if let Some(handle) = self.handle.take() {
            handle.join()
        } else {
            Ok(())
        }
    }
}

/// A pool of worker threads with work-stealing support.
pub struct WorkerPool {
    workers: Vec<Worker>,
    injector: Arc<Injector<Job>>,
    high_priority_injector: Arc<Injector<Job>>,
    shutdown: Arc<CachePadded<AtomicBool>>,
    active_workers: Arc<CachePadded<AtomicUsize>>,
    frame_index: Arc<CachePadded<AtomicU64>>,
    parker: Arc<GlobalParker>,
    has_work: Arc<Vec<CachePadded<AtomicBool>>>,
    // _fiber_pool: Arc<FiberPool>, // Removed as it is now thread-local
}

impl WorkerPool {
    /// Creates a new worker pool with work-stealing queues.
    pub fn new(num_threads: usize, config: crate::job_system::FiberConfig) -> Self {
        Self::new_with_strategy(num_threads, PinningStrategy::None, config, #[cfg(feature = "metrics")] None)
    }

    /// Creates a new worker pool with optional CPU affinity pinning.
    pub fn new_with_affinity(
        num_threads: usize,
        pin_to_core: bool,
        config: crate::job_system::FiberConfig,
    ) -> Self {
        Self::new_with_strategy(
            num_threads,
            if pin_to_core {
                PinningStrategy::Linear
            } else {
                PinningStrategy::None
            },
            config,
            #[cfg(feature = "metrics")]
            None,
        )
    }

    /// Creates a new worker pool with a specific pinning strategy.
    pub fn new_with_strategy(
        num_threads: usize,
        strategy: PinningStrategy,
        config: crate::job_system::FiberConfig,
        #[cfg(feature = "metrics")] metrics: Option<Arc<crate::metrics::Metrics>>,
    ) -> Self {
        let injector = Arc::new(Injector::new());
        let high_priority_injector = Arc::new(Injector::new());
        let shutdown = Arc::new(CachePadded::new(AtomicBool::new(false)));
        let active_workers = Arc::new(CachePadded::new(AtomicUsize::new(0)));
        let frame_index = Arc::new(CachePadded::new(AtomicU64::new(0)));
        let topology = Arc::new(crate::topology::Topology::detect());
        let parker = Arc::new(GlobalParker::new());

        // Load-Aware Stealing: Flags for each worker
        let mut has_work_vec = Vec::with_capacity(num_threads);
        for _ in 0..num_threads {
            has_work_vec.push(CachePadded::new(AtomicBool::new(false)));
        }
        let has_work = Arc::new(has_work_vec);

        // We no longer create a global FiberPool here.
        // It is created lazily in each worker thread.

        let mut local_queues = Vec::with_capacity(num_threads);
        let mut stealers = Vec::with_capacity(num_threads);

        // Create local queues and stealers for each worker
        for _ in 0..num_threads {
            let deque = Deque::new_lifo();
            stealers.push(deque.stealer());
            local_queues.push(deque);
        }

        let stealers = Arc::new(stealers);
        let mut workers = Vec::with_capacity(num_threads);

        // Compute core mapping and tiering based on strategy
        let core_ids = core_affinity::get_core_ids().unwrap_or_default();
        let mut mapped_cores = Vec::with_capacity(num_threads);
        let mut tiers = vec![1; num_threads];
        let mut thresholds = vec![0; num_threads];

        // (Strategy logic omitted for brevity, reusing existing logic)
        // Re-implementing strategy logic since I am overwriting the file.
        match strategy {
            PinningStrategy::None => {
                for _ in 0..num_threads {
                    mapped_cores.push(None);
                }
            }
            PinningStrategy::Linear => {
                for i in 0..num_threads {
                    mapped_cores.push(core_ids.get(i % core_ids.len().max(1)).copied());
                }
            }
            PinningStrategy::AvoidSMT => {
                let physical_cores: Vec<_> = core_ids.iter().step_by(2).copied().collect();
                for i in 0..num_threads {
                    mapped_cores.push(physical_cores.get(i % physical_cores.len().max(1)).copied());
                }
            }
            PinningStrategy::CCDIsolation => {
                let ccd1_cores: Vec<_> = core_ids.iter().step_by(2).take(8).copied().collect();
                for i in 0..num_threads {
                    mapped_cores.push(ccd1_cores.get(i % ccd1_cores.len().max(1)).copied());
                }
            }
            PinningStrategy::TieredSpillover => {
                let ccd0_physical: Vec<_> = core_ids.iter().step_by(2).take(8).copied().collect();
                let ccd1_physical: Vec<_> = core_ids
                    .iter()
                    .step_by(2)
                    .skip(8)
                    .take(8)
                    .copied()
                    .collect();
                let smt_cores: Vec<_> = core_ids.iter().skip(1).step_by(2).copied().collect();

                for i in 0..num_threads {
                    if i < ccd0_physical.len() {
                        mapped_cores.push(Some(ccd0_physical[i]));
                        tiers[i] = 1;
                        thresholds[i] = 0;
                    } else if i < ccd0_physical.len() + ccd1_physical.len() {
                        let idx = i - ccd0_physical.len();
                        mapped_cores.push(Some(ccd1_physical[idx]));
                        tiers[i] = 2;
                        thresholds[i] = 7;
                    } else {
                        let idx = (i - ccd0_physical.len() - ccd1_physical.len()) % smt_cores.len();
                        mapped_cores.push(Some(smt_cores[idx]));
                        tiers[i] = 3;
                        thresholds[i] = 15;
                    }
                }
            }
        };

        // Spawn workers
        for (id, local_queue) in local_queues.into_iter().enumerate() {
            let core_id = mapped_cores.get(id).copied().flatten();
            workers.push(Worker::new(WorkerParams {
                id,
                local_queue,
                stealers: Arc::clone(&stealers),
                injector: Arc::clone(&injector),
                high_priority_injector: Arc::clone(&high_priority_injector),
                shutdown: Arc::clone(&shutdown),
                core_id,
                active_workers: Arc::clone(&active_workers),
                tier: tiers[id],
                threshold: thresholds[id],
                strategy,
                // Pass config to allow local creation (and only use per-worker limit, not total * num_threads)
                // Wait, previous code multiplied: config.initial_pool_size * num_threads.
                // Now each worker gets config.initial_pool_size. This matches.
                fiber_config: config.clone(),
                frame_index: Arc::clone(&frame_index),
                topology: Arc::clone(&topology),
                parker: Arc::clone(&parker),
                has_work: Arc::clone(&has_work),
                #[cfg(feature = "metrics")]
                metrics: metrics.as_ref().map(Arc::clone),
            }));
        }

        WorkerPool {
            workers,
            injector,
            high_priority_injector,
            shutdown,
            active_workers,
            frame_index,
            parker,
            has_work,
            // _fiber_pool: fiber_pool, // Removed
        }
    }

    /// Signals the start of a new frame, triggering allocator resets on workers.
    ///
    /// # Safety
    ///
    /// Caller must ensure no jobs are currently running or pending that reference
    /// frame-allocated memory.
    pub fn start_new_frame(&self) {
        self.frame_index.fetch_add(1, Ordering::Relaxed);
    }

    /// Wakes up a single worker thread.
    pub fn wake_one(&self) {
        self.parker.wake_one();
    }

    /// Submits a single job to the scheduling system.
    pub fn submit(&self, job: Job) {
        // Optimization: Try to push to the thread-local queue if we are on a worker thread
        let job_or_consumed = LOCAL_DEQUE.with(|tls| {
            if let Some(deque_ptr) = tls.get() {
                // SAFETY: We are on the thread that owns this deque (ensured by TLS)
                // and the deque is guaranteed to be alive by ScopedDeque guard in run_loop.
                unsafe {
                    (*deque_ptr).push(job);
                }
                None // Consumed
            } else {
                Some(job) // Not consumed, return it
            }
        });

        let job = match job_or_consumed {
            Some(j) => j,
            None => {
                // If we pushed locally, wake a sleeper
                self.parker.wake_one();
                return;
            }
        };

        // Fallback to global injectors
        if job.priority == crate::job::JobPriority::High {
            self.high_priority_injector.push(job);
        } else {
            self.injector.push(job);
        }
        self.parker.wake_one();
    }

    /// Submits multiple jobs in a batch to reduce contention.
    pub fn submit_batch(&self, jobs: Vec<Job>) {
        if jobs.is_empty() { return; }

        let jobs_or_consumed = LOCAL_DEQUE.with(|tls| {
             if let Some(deque_ptr) = tls.get() {
                 // SAFETY: see submit()
                 unsafe {
                     let deque = &*deque_ptr;
                     for job in jobs {
                         deque.push(job);
                     }
                 }
                 None
             } else {
                 Some(jobs)
             }
        });

        let jobs = match jobs_or_consumed {
            Some(j) => j,
            None => {
                self.parker.wake_one();
                return;
            }
        };

        // Fallback to global injector (batch optimized)
        let count = jobs.len();
        for job in jobs {
            if job.priority == crate::job::JobPriority::High {
                self.high_priority_injector.push(job);
            } else {
                self.injector.push(job);
            }
        }
        
        // Wake up multiple workers to handle the batch.
        // We wake up to 'jobs.len()' workers (clamped), utilizing the Semaphore count.
        // This ensures enough capacity is activated immediately.
        let wake_count = count.min(self.workers.len());
        for _ in 0..wake_count {
            self.parker.wake_one();
        }
    }

    /// Returns the number of worker threads in the pool.
    pub fn size(&self) -> usize {
        self.workers.len()
    }

    /// Returns the number of currently active workers.
    pub fn active_count(&self) -> usize {
        self.active_workers.load(Ordering::Relaxed)
    }

    /// Shuts down the worker pool and waits for all threads to finish.
    pub fn shutdown(self) -> Result<(), WorkerPoolError> {
        while !self.injector.is_empty() {
            // Ensure workers are awake to process the remaining items
            self.parker.wake_all();
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
        self.shutdown
            .store(true, std::sync::atomic::Ordering::Relaxed);
        
        // Wake up all workers so they can see the shutdown flag
        self.parker.wake_all();

        let mut failed_count = 0;
        for worker in self.workers {
            let worker_id = worker.id();
            if worker.join().is_err() {
                failed_count += 1;
                eprintln!("Worker {} panicked during execution", worker_id);
            }
        }

        if failed_count > 0 {
            Err(WorkerPoolError::WorkerPanic { failed_count })
        } else {
            Ok(())
        }
    }
}

impl crate::counter::JobScheduler for WorkerPool {
    fn schedule(&self, job: Job) {
        self.submit(job);
    }

    fn schedule_batch(&self, jobs: Vec<Job>) {
        self.submit_batch(jobs);
    }
}
