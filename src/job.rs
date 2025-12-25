//! Job definitions and execution logic.
//!
//! Jobs are units of work that can be executed by the fiber system.
//! They encapsulate a closure and associated counter for tracking completion.

use crate::counter::Counter;

/// A unit of work to be executed by the job system.
///
/// Jobs consist of a closure to execute and an optional counter
/// that is decremented upon completion.
pub struct Job {
    /// The work to be executed
    work: Box<dyn FnOnce() + Send + 'static>,
    /// Optional counter to decrement when the job completes
    counter: Option<Counter>,
}

impl Job {
    /// Creates a new job with the given work function.
    pub fn new<F>(work: F) -> Self
    where
        F: FnOnce() + Send + 'static,
    {
        Job {
            work: Box::new(work),
            counter: None,
        }
    }

    /// Creates a new job with an associated counter.
    pub fn with_counter<F>(work: F, counter: Counter) -> Self
    where
        F: FnOnce() + Send + 'static,
    {
        Job {
            work: Box::new(work),
            counter: Some(counter),
        }
    }

    /// Executes the job and decrements its counter if present.
    pub fn execute(self) {
        (self.work)();
        
        if let Some(counter) = self.counter {
            counter.decrement();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    #[test]
    fn test_job_execution() {
        let executed = Arc::new(AtomicBool::new(false));
        let executed_clone = executed.clone();

        let job = Job::new(move || {
            executed_clone.store(true, Ordering::SeqCst);
        });

        job.execute();
        assert!(executed.load(Ordering::SeqCst));
    }

    #[test]
    fn test_job_with_counter() {
        let counter = Counter::new(1);
        let counter_clone = counter.clone();

        let job = Job::with_counter(
            move || {
                // Do some work
            },
            counter_clone,
        );

        assert_eq!(counter.value(), 1);
        job.execute();
        assert_eq!(counter.value(), 0);
    }
}
