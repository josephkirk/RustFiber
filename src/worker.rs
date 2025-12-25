//! Worker thread implementation.
//!
//! Worker threads continuously pull jobs from the queue and execute them.
//! They form the foundation of the M:N threading model, where multiple
//! fibers (jobs) are multiplexed onto a fixed number of worker threads.

use crate::fiber::Fiber;
use crate::job::Job;
use crossbeam::channel::{Receiver, Sender};
use std::thread::{self, JoinHandle};

/// A worker thread that executes jobs from a queue.
pub struct Worker {
    id: usize,
    handle: Option<JoinHandle<()>>,
}

impl Worker {
    /// Creates and starts a new worker thread.
    ///
    /// The worker will continuously pull jobs from the receiver and execute them
    /// until the channel is closed.
    pub fn new(id: usize, receiver: Receiver<Job>) -> Self {
        let handle = thread::spawn(move || {
            Worker::run_loop(id, receiver);
        });

        Worker {
            id,
            handle: Some(handle),
        }
    }

    /// Main execution loop for the worker thread.
    fn run_loop(_id: usize, receiver: Receiver<Job>) {
        loop {
            match receiver.recv() {
                Ok(job) => {
                    let fiber = Fiber::new(job);
                    fiber.run();
                }
                Err(_) => {
                    // Channel closed, worker should exit
                    break;
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

/// A pool of worker threads.
pub struct WorkerPool {
    workers: Vec<Worker>,
    sender: Sender<Job>,
}

impl WorkerPool {
    /// Creates a new worker pool with the specified number of threads.
    pub fn new(num_threads: usize) -> Self {
        let (sender, receiver) = crossbeam::channel::unbounded();
        let mut workers = Vec::with_capacity(num_threads);

        for id in 0..num_threads {
            workers.push(Worker::new(id, receiver.clone()));
        }

        WorkerPool { workers, sender }
    }

    /// Submits a job to the worker pool.
    pub fn submit(&self, job: Job) -> Result<(), crossbeam::channel::SendError<Job>> {
        self.sender.send(job)
    }

    /// Returns the number of worker threads in the pool.
    pub fn size(&self) -> usize {
        self.workers.len()
    }

    /// Shuts down the worker pool and waits for all threads to finish.
    pub fn shutdown(self) {
        // Drop the sender to close the channel
        drop(self.sender);

        // Wait for all workers to finish
        for worker in self.workers {
            let _ = worker.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::counter::Counter;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::time::Duration;

    #[test]
    fn test_worker_pool_creation() {
        let pool = WorkerPool::new(4);
        assert_eq!(pool.size(), 4);
        pool.shutdown();
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
        pool.shutdown();
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
        pool.shutdown();
    }
}
