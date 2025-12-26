//! Counter-based synchronization primitives for job completion tracking.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, AtomicPtr, Ordering};
use crate::fiber::{WaitNode, NODE_STATE_WAITING, NODE_STATE_SIGNALED};
use crate::job::Job;
use crossbeam::deque::{Injector, Worker};

/// Trait for scheduling jobs to a queue (Global or Local).
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
            let mut head = self.inner.wait_list.swap(std::ptr::null_mut(), Ordering::Acquire);
            
            // Batch wake
            while !head.is_null() {
                unsafe {
                    let node = &*head;
                    let next_node = node.next.load(Ordering::Relaxed);
                    
                    // Recover the fiber handle and schedule it
                    if !node.fiber_handle.is_null() {
                        // Attempt to signal the node
                         if node.state.compare_exchange(
                             NODE_STATE_WAITING,
                             NODE_STATE_SIGNALED,
                             Ordering::AcqRel,
                             Ordering::Relaxed
                         ).is_ok() {
                             // Successfully transitioned to signaled. Push to injector.
                            let job = Job::resume_job(node.fiber_handle);
                            scheduler.schedule(job);
                         } 
                         // If state was RUNNING, the waiter aborted wait.
                         // If state was SIGNALED, double signal (should not happen in this design).
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
