use crate::fiber::Fiber;
// use std::sync::Arc;

/// A pool of reusable fibers to minimize stack allocation overhead.
/// Note: This pool is designed to be Thread-Local (used by a single Worker).
pub struct FiberPool {
    pool: Vec<Box<Fiber>>,
    stack_size: usize,
}

impl FiberPool {
    /// Creates a new fiber pool with pre-allocated fibers.
    pub fn new(initial_count: usize, stack_size: usize) -> Self {
        let mut pool = FiberPool {
            pool: Vec::with_capacity(initial_count),
            stack_size,
        };
        pool.grow(initial_count);
        pool
    }

    /// Grows the pool by the specified number of fibers.
    /// This allows for interleaved pre-warming (allocating over multiple frames)
    /// rather than taking a massive stall at startup.
    pub fn grow(&mut self, count: usize) {
        for _ in 0..count {
            self.pool.push(Box::new(Fiber::new(self.stack_size)));
        }
    }

    /// Retrieves a fiber from the pool or creates a new one if empty.
    pub fn get(&mut self) -> Box<Fiber> {
        if let Some(mut fiber) = self.pool.pop() {
            fiber.reset(self.stack_size);
            fiber
        } else {
            Box::new(Fiber::new(self.stack_size))
        }
    }

    /// Returns a fiber to the pool for reuse.
    pub fn return_fiber(&mut self, fiber: Box<Fiber>) {
        self.pool.push(fiber);
    }

    /// Returns the current number of fibers in the pool.
    pub fn len(&self) -> usize {
        self.pool.len()
    }
}
