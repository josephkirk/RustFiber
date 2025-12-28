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

/// Configuration for the fiber system.
#[derive(Clone, Debug)]
pub struct FiberConfig {
    /// Stack size for each fiber in bytes. Default: 512KB.
    pub stack_size: usize,
    /// Initial number of fibers to pre-allocate per worker. Default: 128.
    pub initial_pool_size: usize,
}

impl Default for FiberConfig {
    fn default() -> Self {
        Self {
            stack_size: 512 * 1024,
            initial_pool_size: 128,
        }
    }
}

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
        Self::new_with_config(num_threads, FiberConfig::default())
    }

    /// Creates a new job system with custom configuration.
    pub fn new_with_config(num_threads: usize, config: FiberConfig) -> Self {
        JobSystem {
            worker_pool: WorkerPool::new(num_threads, config),
        }
    }

    /// Creates a new job system with CPU core pinning enabled.
    ///
    /// Each worker thread is pinned to a specific CPU core for better
    /// cache locality and reduced context switching overhead.
    /// By default uses `PinningStrategy::Linear`.
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
            worker_pool: WorkerPool::new_with_strategy(
                num_threads,
                crate::PinningStrategy::Linear,
                FiberConfig::default(),
            ),
        }
    }

    /// Creates a new job system with a specific pinning strategy.
    ///
    /// # Arguments
    ///
    /// * `num_threads` - Number of worker threads to create
    /// * `strategy` - The pinning strategy to use
    ///
    /// # Example
    ///
    /// ```
    /// use rustfiber::{JobSystem, PinningStrategy};
    ///
    /// let job_system = JobSystem::new_with_strategy(4, PinningStrategy::AvoidSMT);
    /// ```
    pub fn new_with_strategy(num_threads: usize, strategy: crate::PinningStrategy) -> Self {
        JobSystem {
            worker_pool: WorkerPool::new_with_strategy(
                num_threads,
                strategy,
                FiberConfig::default(),
            ),
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
        // Default to Normal priority
        let job = Job::with_counter(work, counter.clone());
        self.worker_pool.submit(job);
        counter
    }

    /// Submits a job with a specific priority.
    pub fn run_priority<F>(&self, priority: crate::job::JobPriority, work: F) -> Counter
    where
        F: FnOnce() + Send + 'static,
    {
        let counter = Counter::new(1);
        let job = Job::with_counter(work, counter.clone()).with_priority(priority);
        self.worker_pool.submit(job);
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

        self.worker_pool.submit_batch(job_objs);

        counter
    }

    /// Submits a job with context access to be executed by the worker pool.
    ///
    /// Returns a counter that can be used to wait for the job's completion.
    /// The job receives a Context that provides safe access to the job system
    /// for nested parallelism and synchronization.
    ///
    /// # Arguments
    ///
    /// * `work` - The function to execute, which receives a Context
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rustfiber::JobSystem;
    ///
    /// let job_system = JobSystem::new(4);
    /// let counter = job_system.run_with_context(|ctx| {
    ///     println!("Hello from a job with context!");
    ///     // Can spawn child jobs
    ///     let child = ctx.spawn_job(|_| {
    ///         println!("Child job");
    ///     });
    ///     ctx.wait_for(&child);
    /// });
    /// job_system.wait_for_counter(&counter);
    /// ```
    pub fn run_with_context<F>(&self, work: F) -> Counter
    where
        F: FnOnce(&crate::context::Context) + Send + 'static,
    {
        let counter = Counter::new(1);
        let counter_clone = counter.clone();

        let job_system_ptr = self as *const JobSystem as usize;
        let job = Job::with_counter_and_context(work, counter_clone, job_system_ptr);

        self.worker_pool.submit(job);

        counter
    }

    /// Submits multiple jobs with context access and returns a counter tracking all of them.
    ///
    /// # Arguments
    ///
    /// * `jobs` - Iterator of functions to execute, each receiving a Context
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rustfiber::{JobSystem, Context};
    ///
    /// let job_system = JobSystem::new(4);
    /// let mut jobs: Vec<Box<dyn FnOnce(&Context) + Send>> = Vec::new();
    /// jobs.push(Box::new(|_ctx| println!("Job 1")));
    /// jobs.push(Box::new(|_ctx| println!("Job 2")));
    /// jobs.push(Box::new(|_ctx| println!("Job 3")));
    /// let counter = job_system.run_multiple_with_context(jobs);
    /// job_system.wait_for_counter(&counter);
    /// ```
    pub fn run_multiple_with_context<I>(&self, jobs: I) -> Counter
    where
        I: IntoIterator<Item = Box<dyn FnOnce(&crate::context::Context) + Send + 'static>>,
    {
        let jobs_vec: Vec<_> = jobs.into_iter().collect();
        let counter = Counter::new(jobs_vec.len());

        let job_system_ptr = self as *const JobSystem as usize;

        // Convert to Job objects and submit in batch for better performance
        let job_objs: Vec<_> = jobs_vec
            .into_iter()
            .map(|work| {
                let counter_clone = counter.clone();
                Job::with_counter_and_context(work, counter_clone, job_system_ptr)
            })
            .collect();

        self.worker_pool.submit_batch(job_objs);

        counter
    }

    /// Submits a raw Job object to the global injector.
    /// Used for rescheduling yielded fibers.
    pub fn submit_to_injector(&self, job: Job) {
        self.worker_pool.submit(job);
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
    /// Waits for a counter to reach zero (all tracked jobs completed).
    ///
    /// If running in a fiber, this yields execution to other jobs.
    /// If running on a thread (outside fiber), this blocks with a sleep loop.
    pub fn wait_for_counter(&self, counter: &Counter) {
        use crate::fiber::Fiber;
        use std::sync::atomic::Ordering;

        if counter.is_complete() {
            return;
        }

        if let Some(fiber_handle) = Fiber::current() {
            // Fiber path: Intrusive wait
            unsafe {
                let fiber = &*fiber_handle.0;
                let node_ptr = fiber.wait_node.get();
                // Changed from &mut *node_ptr to &*node_ptr to avoid aliasing UB with the
                // concurrent reader in Counter::decrement. Since all fields are atomic/UnsafeCell,
                // shared reference is sufficient and correct.
                let node_ref = &*node_ptr;

                // Adaptive spinning phase (before adding to wait list)
                // This is safe because we are not yet in the wait list, so no "Double Resume" is possible.
                // We double-check is_complete() after registering to avoid "Missed Wakeup".
                const SPIN_LIMIT: usize = 5000;
                let mut spin_count = 0;

                while !counter.is_complete() {
                    if spin_count < SPIN_LIMIT {
                        std::hint::spin_loop();
                        spin_count += 1;
                    } else {
                        break;
                    }
                }

                if counter.is_complete() {
                    return;
                }

                loop {
                    // Check completion first
                    if counter.is_complete() {
                        return;
                    }

                    // Initialize node for waiting
                    node_ref
                        .fiber_handle
                        .store(fiber_handle.0, Ordering::Relaxed);
                    node_ref
                        .state
                        .store(crate::fiber::NODE_STATE_WAITING, Ordering::Release);

                    // Add to counter's wait list
                    counter.add_waiter(node_ptr);

                    // Double check to avoid race condition (Missed Wakeup)
                    if counter.is_complete() {
                        // Stranded Waiter Fix:
                        // If we added ourselves but the counter is already complete, the decrementer might have misses us
                        // (because we added after the flush).
                        // We must ensure the list is flushed.
                        // We act as the "Cleanup Crew".
                        counter.notify_all(&self.worker_pool);

                        // If we successfully woke ourselves, we are now scheduled.
                        // If we yielded, we would be woken.
                        // But we can just return?
                        // No, we yielded, so we MUST call yield_now(Wait) so the stack is preserved until the worker picks us up?
                        // NO. If we are scheduled, we are in the Run Queue.
                        // If we call yield_now(Wait), we suspend.
                        // The worker loop will pick up the "scheduled" version of us (duplicate?).
                        // Wait, notify_all calls Job::resume_job(handle).
                        // This creates a NEW Job with the same FiberHandle.
                        // If we are currently running, and we suspend...
                        // The Worker will drop the "current" fiber reference.
                        // Then the Worker loop continues.
                        // It picks up the NEW Job.
                        // It resumes the fiber.
                        // Fiber returns from yield_now.
                        // This is correct.
                    }

                    // Yield execution. We will be resumed when the counter signals us.
                    Fiber::yield_now(crate::fiber::YieldType::Wait);

                    // On resume, reset state.
                    node_ref
                        .state
                        .store(crate::fiber::NODE_STATE_RUNNING, Ordering::Relaxed);
                }
            }
        } else {
            // Thread path: Blocking wait
            let mut backoff_us = 1;
            const MAX_BACKOFF_US: u64 = 1000;

            while !counter.is_complete() {
                thread::sleep(Duration::from_micros(backoff_us));
                backoff_us = (backoff_us * 2).min(MAX_BACKOFF_US);
            }
        }
    }

    /// Returns the number of worker threads in the system.
    pub fn num_workers(&self) -> usize {
        self.worker_pool.size()
    }

    /// Returns the number of currently active workers.
    pub fn active_workers(&self) -> usize {
        self.worker_pool.active_count()
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
