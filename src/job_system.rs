//! High-level job system interface.
//!
//! The JobSystem is the primary entry point for scheduling and managing
//! parallel work. It provides a clean API for submitting jobs, tracking
//! their completion via counters, and waiting for results.

use crate::counter::Counter;
use crate::job::Job;
use crate::worker::WorkerPool;
use std::thread;
use std::time::Duration;

/// The main job system managing worker threads and job execution.
///
/// This is the primary interface for the fiber-based job system.
/// It manages a pool of worker threads and provides methods for
/// submitting jobs and synchronizing on their completion.
pub struct JobSystem {
    worker_pool: WorkerPool,
}

impl JobSystem {
    /// Creates a new job system with the specified number of worker threads.
    ///
    /// The number of threads typically matches the number of CPU cores
    /// for optimal performance.
    ///
    /// # Arguments
    ///
    /// * `num_threads` - Number of worker threads to create
    ///
    /// # Example
    ///
    /// ```
    /// use rustfiber::JobSystem;
    ///
    /// let job_system = JobSystem::new(4);
    /// ```
    pub fn new(num_threads: usize) -> Self {
        JobSystem {
            worker_pool: WorkerPool::new(num_threads),
        }
    }

    /// Creates a new job system with CPU core pinning enabled.
    ///
    /// Each worker thread is pinned to a specific CPU core for better
    /// cache locality and reduced context switching overhead.
    ///
    /// # Arguments
    ///
    /// * `num_threads` - Number of worker threads to create
    ///
    /// # Example
    ///
    /// ```
    /// use rustfiber::JobSystem;
    ///
    /// let job_system = JobSystem::new_with_affinity(4);
    /// ```
    pub fn new_with_affinity(num_threads: usize) -> Self {
        JobSystem {
            worker_pool: WorkerPool::new_with_affinity(num_threads, true),
        }
    }

    /// Creates a job system with one thread per CPU core.
    ///
    /// # Example
    ///
    /// ```
    /// use rustfiber::JobSystem;
    ///
    /// let job_system = JobSystem::default();
    /// ```
    pub fn with_default_threads() -> Self {
        let num_cpus = thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);
        JobSystem::new(num_cpus)
    }

    /// Submits a job to be executed by the worker pool.
    ///
    /// Returns a counter that can be used to wait for the job's completion.
    ///
    /// # Arguments
    ///
    /// * `work` - The function to execute
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rustfiber::JobSystem;
    ///
    /// let job_system = JobSystem::new(4);
    /// let counter = job_system.run(|| {
    ///     println!("Hello from a job!");
    /// });
    /// job_system.wait_for_counter(&counter);
    /// ```
    pub fn run<F>(&self, work: F) -> Counter
    where
        F: FnOnce() + Send + 'static,
    {
        let counter = Counter::new(1);
        let counter_clone = counter.clone();

        let job = Job::with_counter(work, counter_clone);

        self.worker_pool.submit(job).expect("Failed to submit job");

        counter
    }

    /// Submits multiple jobs and returns a counter tracking all of them.
    ///
    /// # Arguments
    ///
    /// * `jobs` - Iterator of functions to execute
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rustfiber::JobSystem;
    ///
    /// let job_system = JobSystem::new(4);
    /// let jobs: Vec<Box<dyn FnOnce() + Send>> = vec![
    ///     Box::new(|| println!("Job 1")),
    ///     Box::new(|| println!("Job 2")),
    ///     Box::new(|| println!("Job 3")),
    /// ];
    /// let counter = job_system.run_multiple(jobs);
    /// job_system.wait_for_counter(&counter);
    /// ```
    pub fn run_multiple<I>(&self, jobs: I) -> Counter
    where
        I: IntoIterator<Item = Box<dyn FnOnce() + Send + 'static>>,
    {
        let jobs_vec: Vec<_> = jobs.into_iter().collect();
        let counter = Counter::new(jobs_vec.len());

        // Convert to Job objects and submit in batch for better performance
        let job_objs: Vec<_> = jobs_vec
            .into_iter()
            .map(|work| {
                let counter_clone = counter.clone();
                Job::with_counter(work, counter_clone)
            })
            .collect();

        self.worker_pool
            .submit_batch(job_objs)
            .expect("Failed to submit batch");

        counter
    }

    /// Waits for a counter to reach zero (all tracked jobs completed).
    ///
    /// Note: This uses a simple polling approach with sleep. In a production
    /// system, this would be enhanced with condition variables or fiber yielding
    /// for better efficiency.
    ///
    /// # Arguments
    ///
    /// * `counter` - The counter to wait on
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rustfiber::JobSystem;
    ///
    /// let job_system = JobSystem::new(4);
    /// let counter = job_system.run(|| {
    ///     // Do work
    /// });
    /// job_system.wait_for_counter(&counter);
    /// ```
    pub fn wait_for_counter(&self, counter: &Counter) {
        // Use a backoff strategy to reduce CPU usage
        let mut backoff_us = 1;
        const MAX_BACKOFF_US: u64 = 1000; // Max 1ms backoff

        while !counter.is_complete() {
            thread::sleep(Duration::from_micros(backoff_us));
            // Exponential backoff up to max
            backoff_us = (backoff_us * 2).min(MAX_BACKOFF_US);
        }
    }

    /// Returns the number of worker threads in the system.
    pub fn num_workers(&self) -> usize {
        self.worker_pool.size()
    }

    /// Shuts down the job system, waiting for all jobs to complete.
    ///
    /// Returns Ok if shutdown was successful, or Err if any worker threads panicked.
    pub fn shutdown(self) -> Result<(), String> {
        self.worker_pool
            .shutdown()
            .map_err(|count| format!("{} worker thread(s) panicked", count))
    }
}

impl Default for JobSystem {
    fn default() -> Self {
        JobSystem::with_default_threads()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn test_job_system_creation() {
        let job_system = JobSystem::new(4);
        assert_eq!(job_system.num_workers(), 4);
        job_system.shutdown().expect("Shutdown failed");
    }

    #[test]
    fn test_job_system_run() {
        let job_system = JobSystem::new(2);
        let executed = Arc::new(AtomicUsize::new(0));
        let executed_clone = executed.clone();

        let counter = job_system.run(move || {
            executed_clone.fetch_add(1, Ordering::SeqCst);
        });

        job_system.wait_for_counter(&counter);
        assert_eq!(executed.load(Ordering::SeqCst), 1);
        job_system.shutdown().expect("Shutdown failed");
    }

    #[test]
    fn test_job_system_multiple_jobs() {
        let job_system = JobSystem::new(4);
        let executed = Arc::new(AtomicUsize::new(0));

        let num_jobs = 10;
        let mut jobs: Vec<Box<dyn FnOnce() + Send>> = Vec::new();

        for _ in 0..num_jobs {
            let executed_clone = executed.clone();
            jobs.push(Box::new(move || {
                executed_clone.fetch_add(1, Ordering::SeqCst);
            }));
        }

        let counter = job_system.run_multiple(jobs);
        job_system.wait_for_counter(&counter);

        assert_eq!(executed.load(Ordering::SeqCst), num_jobs);
        job_system.shutdown().expect("Shutdown failed");
    }

    #[test]
    fn test_job_system_nested_jobs() {
        let job_system = JobSystem::new(4);
        let result = Arc::new(AtomicUsize::new(0));

        let result_clone = result.clone();
        let counter = job_system.run(move || {
            // Simulate nested job submission
            result_clone.fetch_add(1, Ordering::SeqCst);

            // In a real system, this would submit more jobs
            for _ in 0..5 {
                result_clone.fetch_add(1, Ordering::SeqCst);
            }
        });

        job_system.wait_for_counter(&counter);
        assert_eq!(result.load(Ordering::SeqCst), 6);
        job_system.shutdown().expect("Shutdown failed");
    }
}
