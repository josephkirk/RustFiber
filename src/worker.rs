//! Worker thread implementation.
//!
//! Worker threads continuously pull jobs from the queue and execute them.
//! They form the foundation of the M:N threading model, where multiple
//! fibers (jobs) are multiplexed onto a fixed number of worker threads.

use crate::fiber::Fiber;
use crate::job::Job;
use crossbeam::deque::{Injector, Stealer, Worker as Deque};
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
        shutdown: Arc<std::sync::atomic::AtomicBool>,
        pin_to_core: bool,
    ) -> Self {
        let handle = thread::spawn(move || {
            // Pin worker to its core for better cache locality
            if pin_to_core {
                if let Some(core_ids) = core_affinity::get_core_ids() {
                    if id < core_ids.len() {
                        core_affinity::set_for_current(core_ids[id]);
                    }
                }
            }

            Worker::run_loop(id, local_queue, stealers, injector, shutdown);
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
        shutdown: Arc<std::sync::atomic::AtomicBool>,
    ) {
        loop {
            // Check for shutdown signal
            if shutdown.load(std::sync::atomic::Ordering::Relaxed) {
                break;
            }

            // Try to get a job from the local queue first
            let job = local_queue.pop().or_else(|| {
                // If local queue is empty, try to steal from the global injector
                loop {
                    match injector.steal_batch_and_pop(&local_queue) {
                        crossbeam::deque::Steal::Success(job) => return Some(job),
                        crossbeam::deque::Steal::Empty => break,
                        crossbeam::deque::Steal::Retry => continue,
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
                    let fiber = Fiber::new(job);
                    fiber.run();
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
    shutdown: Arc<std::sync::atomic::AtomicBool>,
}

impl WorkerPool {
    /// Creates a new worker pool with work-stealing queues.
    pub fn new(num_threads: usize) -> Self {
        Self::new_with_affinity(num_threads, false)
    }

    /// Creates a new worker pool with optional CPU affinity pinning.
    ///
    /// When `pin_to_core` is true, each worker thread is pinned to a specific
    /// CPU core, which improves cache locality and reduces context switching overhead.
    pub fn new_with_affinity(num_threads: usize, pin_to_core: bool) -> Self {
        let injector = Arc::new(Injector::new());
        let shutdown = Arc::new(std::sync::atomic::AtomicBool::new(false));
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

        // Spawn workers with their local queues and stealers
        for (id, local_queue) in local_queues.into_iter().enumerate() {
            workers.push(Worker::new(
                id,
                local_queue,
                Arc::clone(&stealers),
                Arc::clone(&injector),
                Arc::clone(&shutdown),
                pin_to_core,
            ));
        }

        WorkerPool {
            workers,
            injector,
            shutdown,
        }
    }

    /// Submits a single job to the global injector.
    pub fn submit(&self, job: Job) -> Result<(), String> {
        self.injector.push(job);
        Ok(())
    }

    /// Submits multiple jobs in a batch to reduce contention.
    pub fn submit_batch(&self, jobs: Vec<Job>) -> Result<(), String> {
        for job in jobs {
            self.injector.push(job);
        }
        Ok(())
    }

    /// Returns the number of worker threads in the pool.
    pub fn size(&self) -> usize {
        self.workers.len()
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
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
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
            pool.submit(job).unwrap();
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
            pool.submit(job).unwrap();
        }

        // Wait for jobs to complete
        while !counter.is_complete() {
            thread::sleep(Duration::from_millis(10));
        }

        assert!(counter.is_complete());
        pool.shutdown().expect("Shutdown failed");
    }
}
