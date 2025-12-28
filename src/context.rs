//! Context type for safe access to job system capabilities from within jobs.

use crate::allocator::linear::FrameAllocator;
use crate::counter::Counter;
use crate::fiber::Fiber;
use crate::job::{Job, SendPtr};
use crate::job_system::JobSystem;
use crossbeam::deque::Worker;

/// Context provided to jobs for accessing fiber system capabilities.
pub struct Context<'a> {
    job_system: &'a JobSystem,
    allocator: Option<SendPtr<FrameAllocator>>,
    local_queue: Option<SendPtr<Worker<Job>>>,
}

impl<'a> Context<'a> {
    pub(crate) fn new(
        job_system: &'a JobSystem,
        allocator: Option<*mut FrameAllocator>,
        local_queue: Option<*const Worker<Job>>,
    ) -> Self {
        Context {
            job_system,
            allocator: allocator.map(SendPtr),
            local_queue: local_queue.map(|p| SendPtr(p as *mut _)),
        }
    }

    pub fn spawn_job<F>(&self, work: F) -> Counter
    where
        F: FnOnce(&Context) + Send + 'static,
    {
        let counter = Counter::new(1);
        self.spawn_internal(work, Some(counter.clone()));
        counter
    }

    /// Spawns a job without a counter (fire-and-forget).
    /// This avoids the overhead of allocating a Counter on the heap.
    pub fn spawn_detached<F>(&self, work: F)
    where
        F: FnOnce(&Context) + Send + 'static,
    {
        self.spawn_internal(work, None);
    }

    /// Spawns a job that shares an existing counter.
    /// Useful for grouping multiple jobs under a single synchronization primitive.
    pub fn spawn_with_counter<F>(&self, work: F, counter: Counter)
    where
        F: FnOnce(&Context) + Send + 'static,
    {
        self.spawn_internal(work, Some(counter));
    }

    fn spawn_internal<F>(&self, work: F, counter: Option<Counter>)
    where
        F: FnOnce(&Context) + Send + 'static,
    {
        let job = if let Some(alloc_ptr) = self.allocator {
            // SAFETY: The allocator acts as a bump allocator owned by the worker provided to the fiber.
            // Nested jobs run before the frame ends, or rather, we must ensure integrity.
            // The FrameAllocator is valid as long as the Worker is running the job/fiber loop.
            // Mutable access is unique because `spawn_job` is sequential within the single fiber.
            unsafe {
                let alloc = &mut *alloc_ptr.0;
                let job_system_ptr = self.job_system as *const _ as usize;

                // Use generic internal method we implemented on Job
                crate::job::Job::with_counter_and_context_in_allocator(
                    work,
                    counter,
                    alloc,
                    job_system_ptr,
                )
            }
        } else {
            // Fallback to heap allocation
            let job_system_ptr = self.job_system as *const _ as usize;
            crate::job::Job::with_counter_and_context(work, counter, job_system_ptr)
        };

        if let Some(queue) = self.local_queue {
            // Push to local queue (Lock-free)
            // SAFETY: The worker and its queue are guaranteed to alive while the context is processing jobs.
            // Queue access is thread-safe for push (owner) and steal (concurrent).
            unsafe {
                (*queue.0).push(job);
            }
        } else {
            // Fallback to global injector (Lock contention)
            self.job_system.submit_to_injector(job);
        }
    }

    pub fn spawn_jobs<I>(&self, jobs: I) -> Counter
    where
        I: IntoIterator<Item = Box<dyn FnOnce(&Context) + Send + 'static>>,
    {
        // For batch operations, we currently only support Box<dyn> input iterator
        // Optimizing this to use FrameAllocator is harder because input is already Boxed!
        // So we keep using existing run_multiple logic
        self.job_system.run_multiple_with_context(jobs)
    }

    pub fn wait_for(&self, counter: &Counter) {
        self.job_system.wait_for_counter(counter)
    }

    /// Yields execution to allow other work to run.
    pub fn yield_now(&self) {
        yield_now();
    }
}

/// Yields execution to allow other work to run.
///
/// If called from within a fiber, yields the fiber.
/// If called from a thread, yields the thread.
pub fn yield_now() {
    use crate::fiber::YieldType;
    if Fiber::current().is_some() {
        // Suspend execution and request rescheduling
        Fiber::yield_now(YieldType::Normal);
    } else {
        // Fallback for non-fiber threads
        std::thread::yield_now();
    }
}

#[cfg(test)]
mod tests {

    // Re-enable tests later (currently context tests use JobSystem::new which spawns threads)
}
