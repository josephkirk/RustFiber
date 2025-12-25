//! Counter-based synchronization primitives for job completion tracking.
//!
//! Counters are the primary synchronization mechanism in the fiber job system.
//! They track the number of incomplete jobs and allow fibers to efficiently
//! wait for job completion without blocking worker threads.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

/// A thread-safe counter for tracking job completion.
///
/// Counters start at a specified value and decrement as jobs complete.
/// Fibers can wait on a counter to reach zero, indicating all tracked
/// jobs have finished.
#[derive(Clone)]
pub struct Counter {
    inner: Arc<AtomicUsize>,
}

impl Counter {
    /// Creates a new counter with the specified initial value.
    pub fn new(initial: usize) -> Self {
        Counter {
            inner: Arc::new(AtomicUsize::new(initial)),
        }
    }

    /// Increments the counter by one.
    pub fn increment(&self) {
        self.inner.fetch_add(1, Ordering::SeqCst);
    }

    /// Decrements the counter by one.
    pub fn decrement(&self) {
        self.inner.fetch_sub(1, Ordering::SeqCst);
    }

    /// Returns the current value of the counter.
    pub fn value(&self) -> usize {
        self.inner.load(Ordering::SeqCst)
    }

    /// Checks if the counter has reached zero.
    pub fn is_complete(&self) -> bool {
        self.value() == 0
    }

    /// Resets the counter to the specified value.
    pub fn reset(&self, value: usize) {
        self.inner.store(value, Ordering::SeqCst);
    }
}

impl Default for Counter {
    fn default() -> Self {
        Counter::new(0)
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

        counter.decrement();
        assert_eq!(counter.value(), 4);

        counter.increment();
        assert_eq!(counter.value(), 5);
    }

    #[test]
    fn test_counter_completion() {
        let counter = Counter::new(1);
        assert!(!counter.is_complete());

        counter.decrement();
        assert!(counter.is_complete());
    }

    #[test]
    fn test_counter_reset() {
        let counter = Counter::new(10);
        counter.reset(5);
        assert_eq!(counter.value(), 5);
    }
}
