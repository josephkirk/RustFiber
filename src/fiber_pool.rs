use crate::fiber::Fiber;
use crossbeam::queue::SegQueue;
// use std::sync::Arc;

/// A pool of reusable fibers to minimize stack allocation overhead.
pub struct FiberPool {
    pool: SegQueue<Box<Fiber>>,
    stack_size: usize,
}

impl FiberPool {
    /// Creates a new fiber pool with pre-allocated fibers.
    pub fn new(initial_count: usize, stack_size: usize) -> Self {
        let pool = SegQueue::new();
        for _ in 0..initial_count {
            pool.push(Box::new(Fiber::new(stack_size)));
        }

        FiberPool { pool, stack_size }
    }

    /// Retrieves a fiber from the pool or creates a new one if empty.
    pub fn get(&self) -> Box<Fiber> {
        if let Some(mut fiber) = self.pool.pop() {
            fiber.reset(self.stack_size);
            fiber
        } else {
            Box::new(Fiber::new(self.stack_size))
        }
    }

    /// Returns a fiber to the pool for reuse.
    pub fn return_fiber(&self, fiber: Box<Fiber>) {
        self.pool.push(fiber);
    }
}
