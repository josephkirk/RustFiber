//! Job definitions and execution logic.
//!
//! Jobs are units of work that can be executed by the fiber system.
//! They encapsulate a closure and associated counter for tracking completion.

use crate::context::Context;
use crate::counter::Counter;

use crate::fiber::FiberHandle;

/// Internal representation of work to be executed.
pub enum Work {
    /// Simple closure without context
    Simple(Box<dyn FnOnce() + Send + 'static>),
    /// Closure that requires context, along with JobSystem reference
    WithContext {
        work: Box<dyn FnOnce(&Context) + Send + 'static>,
        job_system_ptr: usize, // Store as usize to make it Send
    },
    /// Resumption of a suspended fiber
    Resume(FiberHandle),
}

/// A unit of work to be executed by the job system.
///
/// Jobs consist of a closure to execute and an optional counter
/// that is decremented upon completion.
pub struct Job {
    /// The work to be executed
    pub work: Work,
    /// Optional counter to decrement when the job completes
    pub(crate) counter: Option<Counter>,
}

impl Job {
    /// Creates a new job with the given work function.
    pub fn new<F>(work: F) -> Self
    where
        F: FnOnce() + Send + 'static,
    {
        Job {
            work: Work::Simple(Box::new(work)),
            counter: None,
        }
    }

    /// Creates a new job that resumes a suspended fiber.
    pub fn resume_job(handle: FiberHandle) -> Self {
        Job {
             work: Work::Resume(handle),
             counter: None,
        }
    }

    /// Creates a new job with an associated counter.
    pub fn with_counter<F>(work: F, counter: Counter) -> Self
    where
        F: FnOnce() + Send + 'static,
    {
        Job {
            work: Work::Simple(Box::new(work)),
            counter: Some(counter),
        }
    }

    /// Creates a new job with context support and an associated counter.
    ///
    /// # Safety
    ///
    /// The job_system_ptr must remain valid for the lifetime of this job.
    /// This is guaranteed by the JobSystem's design where jobs are executed
    /// before the JobSystem is dropped.
    pub(crate) fn with_counter_and_context<F>(
        work: F,
        counter: Counter,
        job_system_ptr: usize,
    ) -> Self
    where
        F: FnOnce(&Context) + Send + 'static,
    {
        Job {
            work: Work::WithContext {
                work: Box::new(work),
                job_system_ptr,
            },
            counter: Some(counter),
        }
    }

    /// Executes the job and decrements its counter if present.
    pub fn execute(self, injector_ptr: *const crossbeam::deque::Injector<Job>) {
        // SAFETY: injector_ptr is guaranteed valid by the worker
        let injector = unsafe { &*injector_ptr };
        
        match self.work {
            Work::Simple(work) => work(),
            Work::WithContext {
                work,
                job_system_ptr,
            } => {
                // SAFETY: The JobSystem is guaranteed to outlive the jobs it creates.
                debug_assert_ne!(job_system_ptr, 0, "JobSystem pointer cannot be null");
                debug_assert!(
                    job_system_ptr % std::mem::align_of::<crate::job_system::JobSystem>() == 0,
                    "JobSystem pointer must be properly aligned"
                );

                unsafe {
                    let job_system = &*(job_system_ptr as *const crate::job_system::JobSystem);
                    let context = Context::new(job_system);
                    work(&context);
                }
            }
            Work::Resume(_) => {
                panic!("Cannot execute a Resume job directly. Must be handled by worker loop.");
            }
        }

        if let Some(counter) = self.counter {
            counter.decrement(injector);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use crossbeam::deque::Injector;

    #[test]
    fn test_job_execution() {
        let executed = Arc::new(AtomicBool::new(false));
        let executed_clone = executed.clone();

        let job = Job::new(move || {
            executed_clone.store(true, Ordering::SeqCst);
        });

        let injector = Injector::new();
        let injector_ptr = &injector as *const _;
        job.execute(injector_ptr);
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
        let injector = Injector::new();
        let injector_ptr = &injector as *const _;
        job.execute(injector_ptr);
        
        // Note: Decrement logic requires valid injector if waiters exist.
        // Here no waiters, so safe.
        assert_eq!(counter.value(), 0);
    }
}
