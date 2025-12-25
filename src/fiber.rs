//! Fiber management and execution context.
//!
//! This module provides lightweight execution contexts for jobs.
//! In this implementation, we use a simplified approach with closures
//! and work-stealing queues rather than low-level context switching.

use crate::job::Job;

/// Represents a fiber - a lightweight execution context.
///
/// In this implementation, fibers are represented by the jobs they execute.
/// The actual scheduling and multiplexing is handled by the worker threads.
pub struct Fiber {
    job: Option<Job>,
}

impl Fiber {
    /// Creates a new fiber with the given job.
    pub fn new(job: Job) -> Self {
        Fiber { job: Some(job) }
    }

    /// Executes the fiber's job.
    pub fn run(mut self) {
        if let Some(job) = self.job.take() {
            job.execute();
        }
    }

    /// Checks if the fiber has a job to execute.
    pub fn is_ready(&self) -> bool {
        self.job.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    #[test]
    fn test_fiber_execution() {
        let executed = Arc::new(AtomicBool::new(false));
        let executed_clone = executed.clone();

        let job = Job::new(move || {
            executed_clone.store(true, Ordering::SeqCst);
        });

        let fiber = Fiber::new(job);
        assert!(fiber.is_ready());

        fiber.run();
        assert!(executed.load(Ordering::SeqCst));
    }
}
