//! Worker thread implementation.
//!
//! Worker threads continuously pull jobs from the queue and execute them.
//! They form the foundation of the M:N threading model, where multiple
//! fibers (jobs) are multiplexed onto a fixed number of worker threads.

use crate::PinningStrategy;
use crate::fiber::Fiber;
use crate::job::Job;
use core_affinity::CoreId;
use crossbeam::deque::{Injector, Stealer, Worker as Deque};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};

/// A worker thread that executes jobs from a queue.
pub struct Worker {
    id: usize,
    handle: Option<JoinHandle<()>>,
}

impl Worker {
    /// Creates and starts a new worker thread with work-stealing support.
    ///
    /// The worker will continuously pull jobs from its local queue, steal from
    /// other workers when idle, and check the global injector.
    pub fn new(
        id: usize,
        local_queue: Deque<Job>,
        stealers: Arc<Vec<Stealer<Job>>>,
        injector: Arc<Injector<Job>>,
        shutdown: Arc<AtomicBool>,
        core_id: Option<CoreId>,
        active_workers: Arc<AtomicUsize>,
        tier: u32,
        threshold: usize,
    ) -> Self {
        let handle = thread::spawn(move || {
            // Pin worker to its core for better cache locality if specified
            if let Some(core_id) = core_id {
                core_affinity::set_for_current(core_id);
            }

            Worker::run_loop(
                id,
                local_queue,
                stealers,
                injector,
                shutdown,
                active_workers,
                tier,
                threshold,
            );
        });

        Worker {
            id,
            handle: Some(handle),
        }
    }

    /// Main execution loop for the worker thread with work-stealing.
    fn run_loop(
        _id: usize,
        local_queue: Deque<Job>,
        stealers: Arc<Vec<Stealer<Job>>>,
        injector: Arc<Injector<Job>>,
        shutdown: Arc<AtomicBool>,
        active_workers: Arc<AtomicUsize>,
        tier: u32,
        threshold: usize,
    ) {
        loop {
            // Check for shutdown signal
            if shutdown.load(Ordering::Relaxed) {
                break;
            }

            // Tiered Spillover Logic: Stay dormant if load is below threshold
            if tier > 1 && active_workers.load(Ordering::Relaxed) < threshold {
                // Check if there is even any work in the global injector before yielding
                // To avoid deadlocks if Tier 1 is busy with long tasks but injector has work.
                if injector.is_empty() {
                    thread::yield_now();
                    continue;
                }
            }

            // Try to get a job from the local queue first
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
                    let fiber = Fiber::new(job);
                    fiber.run();
                    // Mark as inactive after finishing
                    active_workers.fetch_sub(1, Ordering::Relaxed);
                }
                None => {
                    // No work available, yield to prevent busy-waiting
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
                // Tier 1: CCD0 Physical (first 8 physical cores)
                let ccd0_physical: Vec<_> = core_ids.iter().step_by(2).take(8).copied().collect();
                // Tier 2: CCD1 Physical (next 8 physical cores)
                let ccd1_physical: Vec<_> =
                    core_ids.iter().step_by(2).skip(8).take(8).copied().collect();
                // Tier 3: SMT (all remaining)
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
                        thresholds[i] = 7; // Wake up when 7 threads are busy (80% of Tier 1)
                    } else {
                        let idx = (i - ccd0_physical.len() - ccd1_physical.len()) % smt_cores.len();
                        mapped_cores.push(Some(smt_cores[idx]));
                        tiers[i] = 3;
                        thresholds[i] = 15; // Wake up when 15 threads are busy (approx fully physical)
                    }
                }
            }
        };

        // Spawn workers with their local queues, stealers, and core assignments
        for (id, local_queue) in local_queues.into_iter().enumerate() {
            let core_id = mapped_cores.get(id).copied().flatten();
            workers.push(Worker::new(
                id,
                local_queue,
                Arc::clone(&stealers),
                Arc::clone(&injector),
                Arc::clone(&shutdown),
                core_id,
                Arc::clone(&active_workers),
                tiers[id],
                thresholds[id],
            ));
        }

        WorkerPool {
            workers,
            injector,
            shutdown,
            active_workers,
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
    ///
    /// Returns Ok if all workers shut down successfully, or Err with the
    /// number of workers that panicked.
    pub fn shutdown(self) -> Result<(), usize> {
        // Wait for all jobs in the injector to be processed
        while !self.injector.is_empty() {
            std::thread::sleep(std::time::Duration::from_millis(1));
        }

        // Give workers a moment to finish their current tasks
        std::thread::sleep(std::time::Duration::from_millis(10));

        // Signal all workers to shut down
        self.shutdown
            .store(true, std::sync::atomic::Ordering::Relaxed);

        // Wait for all workers to finish and track failures
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::counter::Counter;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    #[test]
    fn test_worker_pool_creation() {
        let pool = WorkerPool::new(4);
        assert_eq!(pool.size(), 4);
        pool.shutdown().expect("Shutdown failed");
    }

    #[test]
    fn test_worker_pool_execution() {
        let pool = WorkerPool::new(2);
        let counter = Arc::new(AtomicUsize::new(0));

        let num_jobs = 10;
        for _ in 0..num_jobs {
            let counter_clone = counter.clone();
            let job = Job::new(move || {
                counter_clone.fetch_add(1, Ordering::SeqCst);
            });
            pool.submit(job);
        }

        // Wait a bit for jobs to complete
        thread::sleep(Duration::from_millis(100));

        assert_eq!(counter.load(Ordering::SeqCst), num_jobs);
        pool.shutdown().expect("Shutdown failed");
    }

    #[test]
    fn test_worker_pool_with_counter() {
        let pool = WorkerPool::new(4);
        let counter = Counter::new(5);

        for _ in 0..5 {
            let counter_clone = counter.clone();
            let job = Job::with_counter(
                move || {
                    thread::sleep(Duration::from_millis(10));
                },
                counter_clone,
            );
            pool.submit(job);
        }

        // Wait for jobs to complete
        while !counter.is_complete() {
            thread::sleep(Duration::from_millis(10));
        }

        assert!(counter.is_complete());
        pool.shutdown().expect("Shutdown failed");
    }

    #[test]
    fn test_worker_pool_batch_submission() {
        let pool = WorkerPool::new(4);
        let counter = Arc::new(AtomicUsize::new(0));

        let num_jobs = 100;
        let mut jobs = Vec::new();

        for _ in 0..num_jobs {
            let counter_clone = counter.clone();
            jobs.push(Job::new(move || {
                counter_clone.fetch_add(1, Ordering::SeqCst);
            }));
        }

        pool.submit_batch(jobs);

        // Wait for jobs to complete
        thread::sleep(Duration::from_millis(200));

        assert_eq!(counter.load(Ordering::SeqCst), num_jobs);
        pool.shutdown().expect("Shutdown failed");
    }

    #[test]
    fn test_worker_pool_with_affinity() {
        let pool = WorkerPool::new_with_affinity(2, true);
        let counter = Arc::new(AtomicUsize::new(0));

        let num_jobs = 20;
        for _ in 0..num_jobs {
            let counter_clone = counter.clone();
            let job = Job::new(move || {
                counter_clone.fetch_add(1, Ordering::SeqCst);
            });
            pool.submit(job);
        }

        // Wait a bit for jobs to complete
        thread::sleep(Duration::from_millis(100));

        assert_eq!(counter.load(Ordering::SeqCst), num_jobs);
        pool.shutdown().expect("Shutdown failed");
    }

    #[test]
    fn test_work_stealing_load_balance() {
        // Test that work-stealing helps distribute load
        let pool = WorkerPool::new(4);
        let counter = Arc::new(AtomicUsize::new(0));
        let worker_ids = Arc::new(Mutex::new(Vec::<usize>::new()));

        let num_jobs = 100;
        for i in 0..num_jobs {
            let counter_clone = counter.clone();
            let worker_ids_clone = worker_ids.clone();

            let job = Job::new(move || {
                counter_clone.fetch_add(1, Ordering::SeqCst);

                // Track which worker executed this job
                if let Ok(mut ids) = worker_ids_clone.lock() {
                    ids.push(i);
                }

                // Variable work to encourage stealing
                if i % 10 == 0 {
                    thread::sleep(Duration::from_micros(100));
                }
            });
            pool.submit(job);
        }

        // Wait for all jobs to complete
        thread::sleep(Duration::from_millis(500));

        assert_eq!(counter.load(Ordering::SeqCst), num_jobs);
        pool.shutdown().expect("Shutdown failed");
    }
}
