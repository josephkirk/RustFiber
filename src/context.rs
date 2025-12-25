//! Context type for safe access to job system capabilities from within jobs.
//!
//! The Context provides a safe, lifetime-checked interface for jobs to:
//! - Spawn child jobs (nested parallelism)
//! - Wait for job completion
//! - Yield execution cooperatively
//!
//! This eliminates the need for unsafe pointer manipulation when jobs need
//! to access the job system.

use crate::counter::Counter;
use crate::job_system::JobSystem;

/// Context provided to jobs for accessing fiber system capabilities.
///
/// The Context holds a reference to the JobSystem and provides safe methods
/// for spawning child jobs, waiting on counters, and yielding execution.
/// The lifetime parameter ensures the context cannot outlive the JobSystem.
///
/// # Example
///
/// ```no_run
/// use rustfiber::JobSystem;
///
/// let job_system = JobSystem::new(4);
/// let counter = job_system.run_with_context(|ctx| {
///     // Spawn child jobs for nested parallelism
///     let child_counter = ctx.spawn_job(|_| {
///         println!("Child job executing!");
///     });
///     ctx.wait_for(&child_counter);
/// });
/// job_system.wait_for_counter(&counter);
/// ```
pub struct Context<'a> {
    job_system: &'a JobSystem,
}

impl<'a> Context<'a> {
    /// Creates a new Context with a reference to the JobSystem.
    ///
    /// This is an internal API used by the job system when executing
    /// context-aware jobs.
    pub(crate) fn new(job_system: &'a JobSystem) -> Self {
        Context { job_system }
    }

    /// Spawns a child job and returns a counter to wait on.
    ///
    /// This enables nested parallelism where jobs can subdivide work
    /// into smaller child jobs.
    ///
    /// # Arguments
    ///
    /// * `work` - The function to execute, which receives its own Context
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rustfiber::JobSystem;
    ///
    /// let job_system = JobSystem::new(4);
    /// let counter = job_system.run_with_context(|ctx| {
    ///     let child = ctx.spawn_job(|_| {
    ///         // Child work here
    ///     });
    ///     ctx.wait_for(&child);
    /// });
    /// job_system.wait_for_counter(&counter);
    /// ```
    pub fn spawn_job<F>(&self, work: F) -> Counter
    where
        F: FnOnce(&Context) + Send + 'static,
    {
        self.job_system.run_with_context(work)
    }

    /// Spawns multiple child jobs and returns a counter tracking all of them.
    ///
    /// This is more efficient than calling spawn_job multiple times as it
    /// batches the job submissions.
    ///
    /// # Arguments
    ///
    /// * `jobs` - Iterator of functions to execute
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rustfiber::{JobSystem, Context};
    ///
    /// let job_system = JobSystem::new(4);
    /// let counter = job_system.run_with_context(|ctx| {
    ///     let mut jobs: Vec<Box<dyn FnOnce(&Context) + Send>> = Vec::new();
    ///     jobs.push(Box::new(|_ctx| println!("Job 1")));
    ///     jobs.push(Box::new(|_ctx| println!("Job 2")));
    ///     let children = ctx.spawn_jobs(jobs);
    ///     ctx.wait_for(&children);
    /// });
    /// job_system.wait_for_counter(&counter);
    /// ```
    pub fn spawn_jobs<I>(&self, jobs: I) -> Counter
    where
        I: IntoIterator<Item = Box<dyn FnOnce(&Context) + Send + 'static>>,
    {
        self.job_system.run_multiple_with_context(jobs)
    }

    /// Waits for a counter to reach zero (all tracked jobs completed).
    ///
    /// This allows a job to wait for its child jobs to complete before
    /// continuing execution.
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
    /// let counter = job_system.run_with_context(|ctx| {
    ///     let child = ctx.spawn_job(|_| {
    ///         // Child work
    ///     });
    ///     ctx.wait_for(&child); // Wait for child to complete
    /// });
    /// job_system.wait_for_counter(&counter);
    /// ```
    pub fn wait_for(&self, counter: &Counter) {
        self.job_system.wait_for_counter(counter)
    }

    /// Yields execution to allow other work to run.
    ///
    /// This is useful for long-running jobs that want to cooperatively
    /// yield to prevent worker thread starvation.
    ///
    /// Note: Current implementation uses thread yielding as a fallback.
    /// Future enhancements may implement true fiber-level yielding.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rustfiber::JobSystem;
    ///
    /// let job_system = JobSystem::new(4);
    /// let counter = job_system.run_with_context(|ctx| {
    ///     for i in 0..1000 {
    ///         // Do some work
    ///         if i % 100 == 0 {
    ///             ctx.yield_now(); // Let other jobs run
    ///         }
    ///     }
    /// });
    /// job_system.wait_for_counter(&counter);
    /// ```
    pub fn yield_now(&self) {
        // Temporary fallback: use thread yielding
        // Future enhancement: implement fiber-level yielding
        std::thread::yield_now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn test_context_spawn_job() {
        let job_system = JobSystem::new(4);
        let executed = Arc::new(AtomicUsize::new(0));
        let executed_clone = executed.clone();

        let counter = job_system.run_with_context(move |ctx| {
            let exec = executed_clone.clone();
            let child_counter = ctx.spawn_job(move |_| {
                exec.fetch_add(1, Ordering::SeqCst);
            });
            ctx.wait_for(&child_counter);
        });

        job_system.wait_for_counter(&counter);
        assert_eq!(executed.load(Ordering::SeqCst), 1);
        job_system.shutdown().expect("Shutdown failed");
    }

    #[test]
    fn test_context_multiple_children() {
        let job_system = JobSystem::new(4);
        let executed = Arc::new(AtomicUsize::new(0));
        let executed_clone = executed.clone();

        let counter = job_system.run_with_context(move |ctx| {
            let mut counters = Vec::new();
            for _ in 0..5 {
                let exec = executed_clone.clone();
                let child = ctx.spawn_job(move |_| {
                    exec.fetch_add(1, Ordering::SeqCst);
                });
                counters.push(child);
            }
            for child_counter in counters {
                ctx.wait_for(&child_counter);
            }
        });

        job_system.wait_for_counter(&counter);
        assert_eq!(executed.load(Ordering::SeqCst), 5);
        job_system.shutdown().expect("Shutdown failed");
    }

    #[test]
    #[allow(clippy::type_complexity)]
    fn test_context_spawn_jobs() {
        let job_system = JobSystem::new(4);
        let executed = Arc::new(AtomicUsize::new(0));
        let executed_clone = executed.clone();

        let counter = job_system.run_with_context(move |ctx| {
            let mut jobs: Vec<Box<dyn FnOnce(&Context) + Send>> = Vec::new();
            for _ in 0..10 {
                let exec = executed_clone.clone();
                jobs.push(Box::new(move |_| {
                    exec.fetch_add(1, Ordering::SeqCst);
                }));
            }
            let children = ctx.spawn_jobs(jobs);
            ctx.wait_for(&children);
        });

        job_system.wait_for_counter(&counter);
        assert_eq!(executed.load(Ordering::SeqCst), 10);
        job_system.shutdown().expect("Shutdown failed");
    }

    #[test]
    fn test_context_nested_parallelism() {
        let job_system = JobSystem::new(4);
        let result = Arc::new(AtomicUsize::new(0));
        let result_clone = result.clone();

        let counter = job_system.run_with_context(move |ctx| {
            // Parent job spawns children
            let res1 = result_clone.clone();
            let child1 = ctx.spawn_job(move |ctx| {
                // Each child spawns grandchildren
                let res2 = res1.clone();
                let grandchild = ctx.spawn_job(move |_| {
                    res2.fetch_add(1, Ordering::SeqCst);
                });
                ctx.wait_for(&grandchild);
                res1.fetch_add(1, Ordering::SeqCst);
            });
            ctx.wait_for(&child1);
        });

        job_system.wait_for_counter(&counter);
        assert_eq!(result.load(Ordering::SeqCst), 2);
        job_system.shutdown().expect("Shutdown failed");
    }

    #[test]
    fn test_context_yield() {
        let job_system = JobSystem::new(2);
        let counter = job_system.run_with_context(|ctx| {
            for _ in 0..10 {
                ctx.yield_now();
            }
        });
        job_system.wait_for_counter(&counter);
        job_system.shutdown().expect("Shutdown failed");
    }
}
