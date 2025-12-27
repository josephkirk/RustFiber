//! Counter-based synchronization primitives for job completion tracking.

use crate::fiber::{NODE_STATE_SIGNALED, NODE_STATE_SPINNING, NODE_STATE_WAITING, WaitNode};
use crate::job::Job;
use crossbeam::deque::{Injector, Worker};
use std::sync::Arc;
use std::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};

pub trait JobScheduler {
    fn schedule(&self, job: Job);
}

impl JobScheduler for Injector<Job> {
    fn schedule(&self, job: Job) {
        self.push(job);
    }
}

impl JobScheduler for Worker<Job> {
    fn schedule(&self, job: Job) {
        self.push(job);
    }
}

struct InnerCounter {
    value: AtomicUsize,
    wait_list: AtomicPtr<WaitNode>,
}

/// A thread-safe counter for tracking job completion.
#[derive(Clone)]
pub struct Counter {
    inner: Arc<InnerCounter>,
}

impl Counter {
    /// Creates a new counter with the specified initial value.
    pub fn new(initial: usize) -> Self {
        Counter {
            inner: Arc::new(InnerCounter {
                value: AtomicUsize::new(initial),
                wait_list: AtomicPtr::new(std::ptr::null_mut()),
            }),
        }
    }

    /// Increments the counter by one.
    pub fn increment(&self) {
        self.inner.value.fetch_add(1, Ordering::SeqCst);
    }

    /// Decrements the counter by one and wakes waiting fibers if zero.
    ///
    /// Returns true if the counter reached zero.
    /// Decrements the counter by one and wakes waiting fibers if zero.
    ///
    /// Returns true if the counter reached zero.
    pub fn decrement<S: JobScheduler + ?Sized>(&self, scheduler: &S) -> bool {
        // Use Release ordering so that all prior work is visible to woken fibers
        let old_val = self.inner.value.fetch_sub(1, Ordering::Release);
        if old_val == 1 {
            // Counter reached zero. Wake all waiters.
            // Acquire ensures we see the node linking done by waiters.
            let mut head = self
                .inner
                .wait_list
                .swap(std::ptr::null_mut(), Ordering::Acquire);

            // Batch wake
            while !head.is_null() {
                unsafe {
                    let node = &*head;
                    let next_node = node.next.load(Ordering::Relaxed);

                    // Recover the fiber handle and schedule it
                    // The handle must be visible if state implies it is set.
                    // We use Acquire ordering on state check to synchronize with writer's Release.
                    let mut current_state = node.state.load(Ordering::Acquire);
                    #[allow(unused_assignments)]
                    let mut fiber_ptr = std::ptr::null_mut();

                    // If we see WAITING, we must be able to see the fiber_ptr.
                    loop {
                        if current_state == NODE_STATE_WAITING {
                            // Re-load fiber handle to be sure
                            fiber_ptr = node.fiber_handle.load(Ordering::Relaxed);

                            match node.state.compare_exchange_weak(
                                NODE_STATE_WAITING,
                                NODE_STATE_SIGNALED,
                                Ordering::AcqRel,
                                Ordering::Relaxed,
                            ) {
                                Ok(_) => {
                                    // Successfully transitioned to signaled. Push to injector.
                                    use crate::fiber::FiberHandle;
                                    if !fiber_ptr.is_null() {
                                        let job = Job::resume_job(FiberHandle(fiber_ptr));
                                        scheduler.schedule(job);
                                    }
                                    break;
                                }
                                Err(actual) => current_state = actual,
                            }
                        } else if current_state == NODE_STATE_SPINNING {
                            match node.state.compare_exchange_weak(
                                NODE_STATE_SPINNING,
                                NODE_STATE_SIGNALED,
                                Ordering::AcqRel,
                                Ordering::Relaxed,
                            ) {
                                Ok(_) => {
                                    // Spinner will wake itself up. Do NOT enqueue.
                                    break;
                                }
                                Err(actual) => current_state = actual,
                            }
                        } else {
                            // Already RUNNING or SIGNALED.
                            break;
                        }
                    }

                    head = next_node;
                }
            }
            true
        } else {
            false
        }
    }

    /// Adds a waiter to the intrusive list.
    ///
    /// # Safety
    /// The `wait_node` must be pinned/stable in memory.
    pub unsafe fn add_waiter(&self, wait_node: *mut WaitNode) {
        unsafe {
            let mut current_head = self.inner.wait_list.load(Ordering::Relaxed);
            loop {
                // Link node -> current_head
                (*wait_node).next.store(current_head, Ordering::Relaxed);

                // CAS head -> node
                match self.inner.wait_list.compare_exchange_weak(
                    current_head,
                    wait_node,
                    Ordering::Release,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => break,
                    Err(new_head) => current_head = new_head,
                }
            }
        }
    }

    /// Returns the current value of the counter.
    pub fn value(&self) -> usize {
        self.inner.value.load(Ordering::SeqCst)
    }

    /// Checks if the counter has reached zero.
    pub fn is_complete(&self) -> bool {
        self.value() == 0
    }

    // Test helper
    pub fn reset(&self, value: usize) {
        self.inner.value.store(value, Ordering::SeqCst);
    }
}

/// An RAII guard that decrements the counter when dropped.
pub struct CounterGuard<'a> {
    counter: &'a Counter,
    scheduler: &'a dyn JobScheduler,
}

impl<'a> CounterGuard<'a> {
    /// Creates a new counter guard.
    pub fn new(counter: &'a Counter, scheduler: &'a dyn JobScheduler) -> Self {
        Self { counter, scheduler }
    }
}

impl<'a> Drop for CounterGuard<'a> {
    fn drop(&mut self) {
        self.counter.decrement(self.scheduler);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_counter_basic() {
        let counter = Counter::new(5);
        assert_eq!(counter.value(), 5);
        assert!(!counter.is_complete());

        let injector = Injector::new();
        counter.decrement(&injector);
        assert_eq!(counter.value(), 4);

        counter.increment();
        assert_eq!(counter.value(), 5);
    }

    #[test]
    fn test_counter_completion() {
        let counter = Counter::new(1);
        assert!(!counter.is_complete());

        let injector = Injector::new();
        counter.decrement(&injector);
        assert!(counter.is_complete());
    }

    #[test]
    fn test_counter_reset() {
        let counter = Counter::new(10);
        counter.reset(5);
        assert_eq!(counter.value(), 5);
    }
}
