//! Job definitions and execution logic.
//!
//! Jobs are units of work that can be executed by the fiber system.
//! They encapsulate a closure and associated counter for tracking completion.

use crate::allocator::linear::FrameAllocator;
use crate::context::Context;
use crate::counter::Counter;
use crate::fiber::FiberHandle;
use crossbeam::deque::Worker;

// Helper struct to wrap pointers and implement Send
#[derive(Debug)]
pub struct SendPtr<T>(pub *mut T);
unsafe impl<T> Send for SendPtr<T> {}
impl<T> Clone for SendPtr<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Copy for SendPtr<T> {}

/// Priority level for job execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum JobPriority {
    /// Low priority - background tasks
    Low,
    /// Normal priority - standard game logic
    #[default]
    Normal,
    /// High priority - critical path (physics, reliable networking)
    High,
}

/// Internal representation of work to be executed.
pub enum Work {
    /// Simple closure without context (Heap allocated)
    Simple(Box<dyn FnOnce() + Send + 'static>),
    /// Simple closure without context (Frame allocated)
    SimpleFrame {
        func: unsafe fn(*mut u8),
        data: SendPtr<u8>,
    },
    /// Closure that requires context (Heap allocated)
    WithContext {
        work: Box<dyn FnOnce(&Context) + Send + 'static>,
        job_system_ptr: usize, // Store as usize to make it Send
    },
    /// Closure that requires context (Frame allocated)
    WithContextFrame {
        func: unsafe fn(*mut u8, &Context),
        data: SendPtr<u8>,
        job_system_ptr: usize,
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
    /// Execution priority
    pub priority: JobPriority,
}

impl Job {
    /// Creates a new job with the given work function.
    ///
    /// This method falls back to global heap allocation (`Box`) if no worker-local
    /// frame allocator is available or if it is full.
    pub fn new<F>(work: F) -> Self
    where
        F: FnOnce() + Send + 'static,
    {
        use std::alloc::Layout;

        // Try local allocation first
        if let Some(alloc_ptr) = crate::worker::get_current_allocator() {
            // SAFETY: We are on a worker thread with an active frame allocator scope.
            // The pointer is valid and properly aligned. We rely on the single-threaded nature
            // of the worker loop to avoid data races (allocator is not Sync, but TLS guarantees exclusivity).
            // We are creating a temporary mutable reference to `allocator` here.
            // This assumes `Job::new` is not called while another mutable reference to `allocator` is active
            // *in the same call stack* (e.g. inside `allocator.alloc(...)` which is impossible).
            unsafe {
                let allocator = &mut *alloc_ptr;

                unsafe fn trampoline<F: FnOnce()>(ptr: *mut u8) {
                    let f = unsafe { std::ptr::read(ptr as *mut F) };
                    f();
                }

                let layout = Layout::new::<F>();
                // Attempt allocation. If it fails (returns None), we fall through to heap allocation.
                if let Some(ptr) = allocator.alloc(layout) {
                    let ptr = ptr.as_ptr() as *mut F;
                    std::ptr::write(ptr, work);

                    return Job {
                        work: Work::SimpleFrame {
                            func: trampoline::<F>,
                            data: SendPtr(ptr as *mut u8),
                        },
                        counter: None,
                        priority: JobPriority::Normal,
                    };
                }
            }
        }

        // Fallback to heap allocation
        Job {
            work: Work::Simple(Box::new(work)),
            counter: None,
            priority: JobPriority::Normal,
        }
    }

    /// Creates a new job allocated in the provided frame allocator.
    pub fn new_in_allocator<F>(work: F, allocator: &mut FrameAllocator) -> Self
    where
        F: FnOnce() + Send + 'static,
    {
        use std::alloc::Layout;

        unsafe fn trampoline<F: FnOnce()>(ptr: *mut u8) {
            let f = unsafe { std::ptr::read(ptr as *mut F) };
            f();
        }

        let layout = Layout::new::<F>();
        if let Some(ptr) = allocator.alloc(layout) {
            let ptr = ptr.as_ptr() as *mut F;
            unsafe {
                std::ptr::write(ptr, work);
            }
            Job {
                work: Work::SimpleFrame {
                    func: trampoline::<F>,
                    data: SendPtr(ptr as *mut u8),
                },
                counter: None,
                priority: JobPriority::Normal,
            }
        } else {
            // Fallback to heap allocation
            Job::new(work)
        }
    }

    /// Creates a new job with an associated counter, allocated in the frame allocator.
    pub fn with_counter_in_allocator<F>(
        work: F,
        counter: Option<Counter>,
        allocator: &mut FrameAllocator,
    ) -> Self
    where
        F: FnOnce() + Send + 'static,
    {
        use std::alloc::Layout;

        unsafe fn trampoline<F: FnOnce()>(ptr: *mut u8) {
            let f = unsafe { std::ptr::read(ptr as *mut F) };
            f();
        }

        let layout = Layout::new::<F>();
        if let Some(ptr) = allocator.alloc(layout) {
            let ptr = ptr.as_ptr() as *mut F;
            unsafe {
                std::ptr::write(ptr, work);
            }
            Job {
                work: Work::SimpleFrame {
                    func: trampoline::<F>,
                    data: SendPtr(ptr as *mut u8),
                },
                counter,
                priority: JobPriority::Normal,
            }
        } else {
            match counter {
                Some(c) => Job::with_counter(work, c),
                None => Job::new(work),
            }
        }
    }

    /// Creates a new job that resumes a suspended fiber.
    pub fn resume_job(handle: FiberHandle) -> Self {
        Job {
            work: Work::Resume(handle),
            counter: None,
            priority: JobPriority::High, // Resumed fibers usually should continue ASAP
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
            priority: JobPriority::Normal,
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
        counter: Option<Counter>,
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
            counter,
            priority: JobPriority::Normal,
        }
    }

    /// Creates a new job with context support and an associated counter, allocated in the frame allocator.
    pub(crate) fn with_counter_and_context_in_allocator<F>(
        work: F,
        counter: Option<Counter>,
        allocator: &mut FrameAllocator,
        job_system_ptr: usize,
    ) -> Self
    where
        F: FnOnce(&Context) + Send + 'static,
    {
        use std::alloc::Layout;

        unsafe fn trampoline<F: FnOnce(&Context)>(ptr: *mut u8, ctx: &Context) {
            let f = unsafe { std::ptr::read(ptr as *mut F) };
            f(ctx);
        }

        // Ensure proper alignment for F and pointer storage
        let layout = Layout::new::<F>();

        if let Some(ptr) = allocator.alloc(layout) {
            let ptr = ptr.as_ptr() as *mut F;
            unsafe {
                std::ptr::write(ptr, work);
            }
            Job {
                work: Work::WithContextFrame {
                    func: trampoline::<F>,
                    data: SendPtr(ptr as *mut u8),
                    job_system_ptr,
                },
                counter,
                priority: JobPriority::Normal,
            }
        } else {
            Job::with_counter_and_context(work, counter, job_system_ptr)
        }
    }

    /// Sets the priority of the job.
    pub fn with_priority(mut self, priority: JobPriority) -> Self {
        self.priority = priority;
        self
    }

    /// Executes the job and decrements its counter if present.
    ///
    /// Accepts an optional allocator to supply to the Context for nested allocations.
    /// Accepts an optional local queue for optimized nesting.
    pub fn execute(
        self,
        scheduler: &dyn crate::counter::JobScheduler,
        allocator: Option<*mut FrameAllocator>,
        queue: Option<*const Worker<Job>>,
    ) {
        use crate::counter::CounterGuard;

        // Create the RAII guard for the counter if one exists.
        // This ensures the counter is decremented even if the job panics.
        let _guard = self
            .counter
            .as_ref()
            .map(|c| CounterGuard::new(c, scheduler));

        match self.work {
            Work::Simple(work) => work(),
            Work::WithContext {
                work,
                job_system_ptr,
            } => {
                // SAFETY: The JobSystem is guaranteed to outlive the jobs it creates.
                // ... (existing code)
                debug_assert_ne!(job_system_ptr, 0, "JobSystem pointer cannot be null");
                debug_assert!(
                    job_system_ptr % std::mem::align_of::<crate::job_system::JobSystem>() == 0,
                    "JobSystem pointer must be properly aligned"
                );

                unsafe {
                    let job_system = &*(job_system_ptr as *const crate::job_system::JobSystem);
                    let context = Context::new(job_system, allocator, queue);
                    work(&context);
                }
            }
            Work::SimpleFrame { func, data } => unsafe {
                (func)(data.0);
            },
            Work::WithContextFrame {
                func,
                data,
                job_system_ptr,
            } => unsafe {
                let job_system = &*(job_system_ptr as *const crate::job_system::JobSystem);
                let context = Context::new(job_system, allocator, queue);
                (func)(data.0, &context);
            },
            Work::Resume(_) => {
                panic!("Cannot execute a Resume job directly. Must be handled by worker loop.");
            }
        }

        // _guard is dropped here, triggering decrement
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crossbeam::deque::Injector;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering}; // Import trait

    #[test]
    fn test_job_execution() {
        let executed = Arc::new(AtomicBool::new(false));
        let executed_clone = executed.clone();

        let job = Job::new(move || {
            executed_clone.store(true, Ordering::SeqCst);
        });

        let injector = Injector::new();
        // Pass injector as scheduler
        job.execute(&injector, None, None);
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
        job.execute(&injector, None, None);

        // Note: Decrement logic requires valid injector if waiters exist.
        // Here no waiters, so safe.
        assert_eq!(counter.value(), 0);
    }
}
