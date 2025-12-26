//! Context type for safe access to job system capabilities from within jobs.

use crate::counter::Counter;
use crate::job_system::JobSystem;
use crate::fiber::Fiber;
// use crate::job::Job;

/// Context provided to jobs for accessing fiber system capabilities.
pub struct Context<'a> {
    job_system: &'a JobSystem,
}

impl<'a> Context<'a> {
    pub(crate) fn new(job_system: &'a JobSystem) -> Self {
        Context { job_system }
    }

    pub fn spawn_job<F>(&self, work: F) -> Counter
    where
        F: FnOnce(&Context) + Send + 'static,
    {
        self.job_system.run_with_context(work)
    }

    pub fn spawn_jobs<I>(&self, jobs: I) -> Counter
    where
        I: IntoIterator<Item = Box<dyn FnOnce(&Context) + Send + 'static>>,
    {
        self.job_system.run_multiple_with_context(jobs)
    }

    pub fn wait_for(&self, counter: &Counter) {
        self.job_system.wait_for_counter(counter)
    }

    /// Yields execution to allow other work to run.
    /// Yields execution to allow other work to run.
    pub fn yield_now(&self) {
        use crate::fiber::YieldType;
        if Fiber::current().is_some() {
            // Suspend execution and request rescheduling
            Fiber::yield_now(YieldType::Normal);
        } else {
            // Fallback for non-fiber threads
            std::thread::yield_now();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    // Re-enable tests later (currently context tests use JobSystem::new which spawns threads)
}
