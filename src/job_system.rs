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
    /// Initial number of fibers to pre-allocate per worker. Default: 16 (Fast Startup).
    pub initial_pool_size: usize,
    /// Target number of fibers to keep in the pool. Default: 128.
    /// Workers will incrementally allocate fibers up to this limit during idle cycles.
    pub target_pool_size: usize,
    /// Size of the per-worker frame allocator in bytes. Default: 1MB.
    pub frame_stack_size: usize,
}

impl Default for FiberConfig {
    fn default() -> Self {
        Self {
            stack_size: 512 * 1024,
            initial_pool_size: 16,  // Reduced for fast startup (NUMA-friendly)
            target_pool_size: 128,  // Workers will grow to this size in background
            frame_stack_size: 1024 * 1024,
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
        let job = Job::with_counter_and_context(work, Some(counter_clone), job_system_ptr);

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
                Job::with_counter_and_context(work, Some(counter_clone), job_system_ptr)
            })
            .collect();

        self.worker_pool.submit_batch(job_objs);

        counter
    }

    /// Executes a parallel for-loop over a range, split into batches.
    ///
    /// This method automatically divides the range into chunks of size `batch_size`
    /// and spawns a job for each chunk. This is significantly more efficient than
    /// spawning a job per iteration for small loop bodies.
    ///
    /// # Arguments
    ///
    /// * `range` - The range of indices to iterate over (e.g., `0..1_000_000`)
    /// * `batch_size` - The number of iterations per job.
    /// * `body` - The closure to execute for each index. Must be `Clone + Send + Sync`.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rustfiber::JobSystem;
    ///
    /// let job_system = JobSystem::new(4);
    /// let data: Vec<i32> = vec![0; 1000];
    ///
    /// // Process 1000 items in chunks of 100 (10 jobs total)
    /// let counter = job_system.parallel_for(0..1000, 100, |i| {
    ///     // Process index i
    /// });
    /// job_system.wait_for_counter(&counter);
    /// ```
    pub fn parallel_for<F>(
        &self,
        range: std::ops::Range<usize>,
        batch_size: usize,
        body: F,
    ) -> Counter
    where
        F: Fn(usize) + Send + Sync + Clone + 'static,
    {
        if range.is_empty() {
            return Counter::new(0);
        }

        let len = range.len();
        let batch_size = batch_size.max(1);
        let num_batches = len.div_ceil(batch_size);

        let start = range.start;
        let counter = Counter::new(num_batches);

        let mut jobs = Vec::with_capacity(num_batches);

        for i in 0..num_batches {
            let chunk_start = start + i * batch_size;
            let chunk_end = (chunk_start + batch_size).min(range.end);
            let body = body.clone();
            let cnt = counter.clone();

            let job = crate::job::Job::with_counter(
                move || {
                    for idx in chunk_start..chunk_end {
                        body(idx);
                    }
                },
                cnt,
            )
            .with_priority(crate::job::JobPriority::Normal);
            jobs.push(job);
        }

        self.worker_pool.submit_batch(jobs);
        counter
    }

    /// Executes a parallel for-loop with automatically calculated batch size.
    ///
    /// The batch size is chosen to be large enough to amortize overhead but small
    /// enough to ensure good load balancing across all worker threads.
    pub fn parallel_for_auto<F>(&self, range: std::ops::Range<usize>, body: F) -> Counter
    where
        F: Fn(usize) + Send + Sync + Clone + 'static,
    {
        let len = range.len();
        if len == 0 {
            return Counter::new(0);
        }

        // Heuristic: aim for 4 * num_workers batches to allow for work stealing
        let num_workers = self.worker_pool.size();
        let target_batches = num_workers * 4;
        let batch_size = (len / target_batches).max(1);

        self.parallel_for(range, batch_size, body)
    }

    /// Executes a parallel for-loop over a range, passing the whole chunk range to the closure.
    ///
    /// This variant allows manual loop unrolling or batch-local optimizations (like simd or local accumulation)
    /// within the closure, which can be more efficient than per-index calls.
    pub fn parallel_for_chunked<F>(
        &self,
        range: std::ops::Range<usize>,
        batch_size: usize,
        body: F,
    ) -> Counter
    where
        F: Fn(std::ops::Range<usize>) + Send + Sync + Clone + 'static,
    {
        if range.is_empty() {
            return Counter::new(0);
        }

        let len = range.len();
        let batch_size = batch_size.max(1);
        let num_batches = len.div_ceil(batch_size);

        let start = range.start;
        let counter = Counter::new(num_batches);

        let mut jobs = Vec::with_capacity(num_batches);

        for i in 0..num_batches {
            let chunk_start = start + i * batch_size;
            let chunk_end = (chunk_start + batch_size).min(range.end);
            let body = body.clone();
            let cnt = counter.clone();

            let job = crate::job::Job::with_counter(
                move || {
                    body(chunk_start..chunk_end);
                },
                cnt,
            )
            .with_priority(crate::job::JobPriority::Normal);
            jobs.push(job);
        }

        self.worker_pool.submit_batch(jobs);
        counter
    }

    /// Executes a parallel for-loop (chunked) with automatically calculated batch size.
    pub fn parallel_for_chunked_auto<F>(&self, range: std::ops::Range<usize>, body: F) -> Counter
    where
        F: Fn(std::ops::Range<usize>) + Send + Sync + Clone + 'static,
    {
        let len = range.len();
        if len == 0 {
            return Counter::new(0);
        }

        let num_workers = self.worker_pool.size();
        let target_batches = num_workers * 4;
        let batch_size = (len / target_batches).max(1);

        self.parallel_for_chunked(range, batch_size, body)
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

    /// Signals the start of a new frame.
    ///
    /// This triggers a reset of all worker frame allocators.
    ///
    /// # Safety
    ///
    /// The caller must ensure that all jobs allocated in the previous frame
    /// have completed. Calling this while frame-allocated jobs are running
    /// or pending will lead to undefined behavior (use-after-free).
    pub fn start_new_frame(&self) {
        self.worker_pool.start_new_frame();
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

    #[test]
    fn test_parallel_for() {
        let job_system = JobSystem::new(4);
        let len = 1000;
        let data = Arc::new(AtomicUsize::new(0));
        let data_clone = data.clone();

        let counter = job_system.parallel_for(0..len, 100, move |_idx| {
            data_clone.fetch_add(1, Ordering::Relaxed);
        });

        job_system.wait_for_counter(&counter);
        assert_eq!(data.load(Ordering::Relaxed), len);
        job_system.shutdown().expect("Shutdown failed");
    }

    #[test]
    fn test_parallel_for_auto() {
        let job_system = JobSystem::new(4);
        let len = 10000;
        let data = Arc::new(AtomicUsize::new(0));
        let data_clone = data.clone();

        let counter = job_system.parallel_for_auto(0..len, move |_idx| {
            data_clone.fetch_add(1, Ordering::Relaxed);
        });

        job_system.wait_for_counter(&counter);
        assert_eq!(data.load(Ordering::Relaxed), len);
        job_system.shutdown().expect("Shutdown failed");
    }

    #[test]
    fn test_parallel_for_uneven() {
        let job_system = JobSystem::new(4);
        let len = 105;
        let data = Arc::new(AtomicUsize::new(0));
        let data_clone = data.clone();

        // Batch size 10 means 10 chunks of 10 and 1 chunk of 5
        let counter = job_system.parallel_for(0..len, 10, move |_idx| {
            data_clone.fetch_add(1, Ordering::Relaxed);
        });

        job_system.wait_for_counter(&counter);
        assert_eq!(data.load(Ordering::Relaxed), len);
        job_system.shutdown().expect("Shutdown failed");
    }
}
