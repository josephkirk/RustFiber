//! Worker thread implementation.
//!
//! Worker threads continuously pull jobs from the queue and execute them.
//! They form the foundation of the M:N threading model, where multiple
//! fibers (jobs) are multiplexed onto a fixed number of worker threads.

use crate::PinningStrategy;
use crate::fiber::{FiberInput, FiberState};
use crate::job::Job;
use crate::fiber_pool::FiberPool;
use core_affinity::CoreId;
use crossbeam::deque::{Injector, Stealer, Worker as Deque};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
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
    pub(crate) shutdown: Arc<AtomicBool>,
    pub(crate) active_workers: Arc<AtomicUsize>,
    pub(crate) fiber_pool: Arc<FiberPool>,
    pub(crate) tier: u32,
    pub(crate) threshold: usize,
    pub(crate) core_id: Option<CoreId>,
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
            shutdown,
            active_workers,
            fiber_pool,
            tier,
            threshold,
            ..
        } = params;

        loop {
            // Check for shutdown signal
            if shutdown.load(Ordering::Relaxed) {
                break;
            }

            // Tiered Spillover Logic: Stay dormant if load is below threshold
            if tier > 1 && active_workers.load(Ordering::Relaxed) < threshold {
                if injector.is_empty() {
                    thread::yield_now();
                    continue;
                }
            }

            // Try to get a job: Local -> Global -> Steal
            let job = local_queue.pop().or_else(|| {
                // If local queue is empty, try to steal from the global injector
                let mut retry_count = 0;
                const MAX_RETRIES: usize = 3;

                loop {
                    match injector.steal_batch_and_pop(&local_queue) {
                        crossbeam::deque::Steal::Success(job) => return Some(job),
                        crossbeam::deque::Steal::Empty => break,
                        crossbeam::deque::Steal::Retry => {
                            retry_count += 1;
                            if retry_count >= MAX_RETRIES {
                                std::thread::yield_now();
                                break;
                            }
                        }
                    }
                }

                // Try to steal from other workers
                stealers
                    .iter()
                    .map(|s| s.steal())
                    .find_map(|steal_result| match steal_result {
                        crossbeam::deque::Steal::Success(job) => Some(job),
                        _ => None,
                    })
            });

            match job {
                Some(job) => {
                    // Mark as active before running
                    active_workers.fetch_add(1, Ordering::Relaxed);
                    
                    let (mut fiber, input) = match job.work {
                        crate::job::Work::Resume(handle) => {
                            // Resume suspended fiber
                            // SAFETY: The handle comes from a Box<Fiber> that was leaked into raw pointer
                            // by the worker that suspended it. We reclaim ownership here.
                            let fiber = unsafe { Box::from_raw(handle.0) };
                            (fiber, FiberInput::Resume)
                        }
                        work => {
                            // New job - acquire fiber from pool
                            let mut fiber = fiber_pool.get();
                            // Reconstruct job (we moved work out)
                            let job = Job { work, counter: job.counter }; 
                            let injector_ptr = &*injector as *const Injector<Job>;
                            let fiber_ptr = fiber.as_mut() as *mut crate::fiber::Fiber;
                            (fiber, FiberInput::Start(job, injector_ptr, fiber_ptr))
                        }
                    };

                    // Run the fiber
                    let state = fiber.resume(input);
                    
                    match state {
                        FiberState::Complete => {
                            fiber_pool.return_fiber(fiber);
                        }
                        FiberState::Yielded(reason) => {
                            let raw_fiber = Box::into_raw(fiber);
                            if let crate::fiber::YieldType::Normal = reason {
                                // Reschedule the fiber immediately
                                // SAFETY: The injector pointer is valid for the worker's lifetime
                                let injector_ptr = &*injector as *const Injector<Job>;
                                // Reclaim raw pointer safely into a Handle job
                                let handle = crate::fiber::FiberHandle(raw_fiber);
                                let job = Job::resume_job(handle);
                                
                                unsafe {
                                    (*injector_ptr).push(job);
                                }
                            }
                        }
                        FiberState::Panic(err) => {
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
                     // No work available, brief spin then yield
                    std::thread::yield_now();
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
    shutdown: Arc<AtomicBool>,
    active_workers: Arc<AtomicUsize>,
    _fiber_pool: Arc<FiberPool>,
}

impl WorkerPool {
    /// Creates a new worker pool with work-stealing queues.
    pub fn new(num_threads: usize) -> Self {
        Self::new_with_strategy(num_threads, PinningStrategy::None)
    }

    /// Creates a new worker pool with optional CPU affinity pinning.
    pub fn new_with_affinity(num_threads: usize, pin_to_core: bool) -> Self {
        Self::new_with_strategy(
            num_threads,
            if pin_to_core {
                PinningStrategy::Linear
            } else {
                PinningStrategy::None
            },
        )
    }

    /// Creates a new worker pool with a specific pinning strategy.
    pub fn new_with_strategy(num_threads: usize, strategy: PinningStrategy) -> Self {
        let injector = Arc::new(Injector::new());
        let shutdown = Arc::new(AtomicBool::new(false));
        let active_workers = Arc::new(AtomicUsize::new(0));
        // Default stack size 2MB for safety, pool size 128 per thread.
        let fiber_pool = Arc::new(FiberPool::new(num_threads * 128, 2 * 1024 * 1024));
        
        let mut local_queues = Vec::with_capacity(num_threads);
        let mut stealers = Vec::with_capacity(num_threads);

        // Create local queues and stealers for each worker
        for _ in 0..num_threads {
            let deque = Deque::new_fifo();
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
                shutdown: Arc::clone(&shutdown),
                core_id,
                active_workers: Arc::clone(&active_workers),
                fiber_pool: Arc::clone(&fiber_pool),
                tier: tiers[id],
                threshold: thresholds[id],
            }));
        }

        WorkerPool {
            workers,
            injector,
            shutdown,
            active_workers,
            _fiber_pool: fiber_pool,
        }
    }

    /// Submits a single job to the global injector.
    pub fn submit(&self, job: Job) {
        self.injector.push(job);
    }

    /// Submits multiple jobs in a batch to reduce contention.
    pub fn submit_batch(&self, jobs: Vec<Job>) {
        for job in jobs {
            self.injector.push(job);
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
        self.shutdown.store(true, std::sync::atomic::Ordering::Relaxed);

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
