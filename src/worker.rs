//! Worker thread implementation.
//!
//! Worker threads continuously pull jobs from the queue and execute them.
//! They form the foundation of the M:N threading model, where multiple
//! fibers (jobs) are multiplexed onto a fixed number of worker threads.

use crate::PinningStrategy;
use crate::allocator::linear::FrameAllocator;
use crate::fiber::{AllocatorPtr, FiberInput, FiberState, QueuePtr};
use crate::fiber_pool::FiberPool;
use crate::job::Job;
use core_affinity::CoreId;
use crossbeam::deque::{Injector, Stealer, Worker as Deque};
use crossbeam::utils::{Backoff, CachePadded};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::thread::{self, JoinHandle};

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
    pub(crate) fiber_pool: Arc<FiberPool>,
    pub(crate) tier: u32,
    pub(crate) threshold: usize,
    pub(crate) core_id: Option<CoreId>,
    pub(crate) strategy: PinningStrategy,
    pub(crate) allocator: FrameAllocator,
    pub(crate) frame_index: Arc<CachePadded<AtomicU64>>,
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
            local_queue,
            stealers,
            injector,
            high_priority_injector,
            shutdown,
            active_workers,
            fiber_pool,
            tier,
            threshold,
            strategy,
            mut allocator,
            frame_index,
            ..
        } = params;

        // Init backoff outside the loop so it persists across iterations
        let backoff = Backoff::new();

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
                unsafe {
                    allocator.reset();
                }
                local_frame_index = global_frame_index;
            }

            // Tiered Spillover Logic: Stay dormant if load is below threshold
            if tier > 1 && active_workers.load(Ordering::Relaxed) < threshold && injector.is_empty()
            {
                thread::yield_now();
                continue;
            }

            // Try to get a job: High Priority -> Local -> Steal -> Global
            // We check high priority global queue first
            let job = loop {
                match high_priority_injector.steal() {
                    crossbeam::deque::Steal::Success(job) => break Some(job),
                    crossbeam::deque::Steal::Retry => continue,
                    crossbeam::deque::Steal::Empty => break None,
                }
            }
            .or_else(|| local_queue.pop())
            .or_else(|| {
                // Try to steal from other workers first
                stealers
                    .iter()
                    .map(|s| s.steal())
                    .find_map(|steal_result| match steal_result {
                        crossbeam::deque::Steal::Success(job) => Some(job),
                        _ => None,
                    })
                    .or_else(|| {
                        // If stealing failed, try the global injector
                        let mut retry_count = 0;
                        const MAX_RETRIES: usize = 3;

                        loop {
                            match injector.steal_batch_and_pop(&local_queue) {
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
                    let state = fiber.resume(input);

                    match state {
                        FiberState::Complete => {
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
                                            high_priority_injector.push(job);
                                        } else {
                                            injector.push(job);
                                        }
                                    }
                                    _ => {
                                        // Keep local for other strategies
                                        local_queue.push(job);
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
                    if backoff.is_completed() {
                        // Deep Idle: Release SMT resources completely for a short while
                        thread::sleep(std::time::Duration::from_micros(100));
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
    _fiber_pool: Arc<FiberPool>,
}

impl WorkerPool {
    /// Creates a new worker pool with work-stealing queues.
    pub fn new(num_threads: usize, config: crate::job_system::FiberConfig) -> Self {
        Self::new_with_strategy(num_threads, PinningStrategy::None, config)
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
        )
    }

    /// Creates a new worker pool with a specific pinning strategy.
    pub fn new_with_strategy(
        num_threads: usize,
        strategy: PinningStrategy,
        config: crate::job_system::FiberConfig,
    ) -> Self {
        let injector = Arc::new(Injector::new());
        let high_priority_injector = Arc::new(Injector::new());
        let shutdown = Arc::new(CachePadded::new(AtomicBool::new(false)));
        let active_workers = Arc::new(CachePadded::new(AtomicUsize::new(0)));
        let frame_index = Arc::new(CachePadded::new(AtomicU64::new(0)));

        let fiber_pool = Arc::new(FiberPool::new(
            config.initial_pool_size * num_threads,
            config.stack_size,
        ));

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
                fiber_pool: Arc::clone(&fiber_pool),
                tier: tiers[id],
                threshold: thresholds[id],
                strategy,
                allocator: FrameAllocator::new(config.frame_stack_size),
                frame_index: Arc::clone(&frame_index),
            }));
        }

        WorkerPool {
            workers,
            injector,
            high_priority_injector,
            shutdown,
            active_workers,
            frame_index,
            _fiber_pool: fiber_pool,
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

    /// Submits a single job to the scheduling system.
    pub fn submit(&self, job: Job) {
        if job.priority == crate::job::JobPriority::High {
            self.high_priority_injector.push(job);
        } else {
            self.injector.push(job);
        }
    }

    /// Submits multiple jobs in a batch to reduce contention.
    pub fn submit_batch(&self, jobs: Vec<Job>) {
        // We could optimize this by splitting batch, but for now simple iteration is fine
        for job in jobs {
            self.submit(job);
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
    pub fn shutdown(self) -> Result<(), usize> {
        while !self.injector.is_empty() {
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
        self.shutdown
            .store(true, std::sync::atomic::Ordering::Relaxed);

        let mut failed_count = 0;
        for worker in self.workers {
            let worker_id = worker.id();
            if worker.join().is_err() {
                failed_count += 1;
                eprintln!("Worker {} panicked during execution", worker_id);
            }
        }

        if failed_count > 0 {
            Err(failed_count)
        } else {
            Ok(())
        }
    }
}

impl crate::counter::JobScheduler for WorkerPool {
    fn schedule(&self, job: Job) {
        self.submit(job);
    }
}
