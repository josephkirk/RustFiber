//! # RustFiber - High-Performance Fiber-Based Job System
//!
//! A fiber-based job system implementation in Rust following the architecture
//! pioneered by Naughty Dog's engine. This system provides efficient task parallelism
//! using lightweight user-space execution contexts (fibers) scheduled across a pool
//! of worker threads.
//!
//! ## Architecture
//!
//! The system follows an M:N threading model where M fibers are multiplexed onto
//! N hardware threads (worker threads).
//!
//! ### Key Components
//!
//! - **Fibers**: Lightweight execution contexts (coroutines) that can be suspended and resumed.
//!   Switching fibers is a user-space operation (nanoseconds) vs OS thread switching (microseconds).
//! - **Work Stealing**: Uses a Chase-Lev deque for load balancing.
//!   - **Local Queue (LIFO)**: Hot tasks are pushed/popped locally for cache locality.
//!   - **Stealing (FIFO)**: Idle workers steal cold tasks from the top of other workers' queues.
//! - **Lock-Free Synchronization**: Dependencies are tracked via `Counter`s.
//!   Waiting on a counter adds the fiber to a wait-list and yields execution, freeing the
//!   worker to run other jobs immediately.
//! - **Frame Allocator**: Each fiber possesses a linear bump-allocator for temporary allocations.
//!   This enables **Zero-Allocation** job spawning, where closures are allocated on the stack/bump-ptr
//!   without `malloc`/`free` overhead.
//!
//! ## Examples
//!
//! ### 1. Basic Usage
//!
//! Fire and forget a job and wait for it.
//!
//! ```no_run
//! use rustfiber::JobSystem;
//!
//! let job_system = JobSystem::new(4); // 4 worker threads
//!
//! let counter = job_system.run(|| {
//!     println!("Hello from a fiber job!");
//! });
//!
//! job_system.wait_for_counter(&counter);
//! ```
//!
//! ### 2. Nested Parallelism (Divide & Conquer)
//!
//! Use `run_with_context` to spawn child jobs. This is the primary pattern for
//! parallel algorithms like Ray Tracing, QuickSort, or Entity Updates.
//!
//! ```no_run
//! use rustfiber::JobSystem;
//!
//! let job_system = JobSystem::new(4);
//!
//! let root = job_system.run_with_context(|ctx| {
//!     // Spawn 10 child jobs
//!     let mut children = vec![];
//!     for i in 0..10 {
//!         let counter = ctx.spawn_job(move |_| {
//!             println!("Processing chunk {}", i);
//!         });
//!         children.push(counter);
//!     }
//!
//!     // Wait for all children to complete
//!     for child in children {
//!         ctx.wait_for(&child);
//!     }
//! });
//!
//! job_system.wait_for_counter(&root);
//! ```
//!
//! ### 3. Zero-Allocation "Fire-and-Forget"
//!
//! For massive numbers of tiny tasks (e.g., 100,000 particles), managing individual counters is too heavy.
//! Use `spawn_detached` to fire tasks with zero allocation overhead.
//!
//! ```no_run
//! use rustfiber::JobSystem;
//! use std::sync::atomic::{AtomicUsize, Ordering};
//! use std::sync::Arc;
//!
//! let job_system = JobSystem::new(8);
//! let completed_count = Arc::new(AtomicUsize::new(0));
//!
//! let root = job_system.run_with_context(move |ctx| {
//!     for _ in 0..10_000 {
//!         let c = completed_count.clone();
//!         // No Counter returned, no heap allocation, extremely fast!
//!         ctx.spawn_detached(move |_| {
//!             // Do work...
//!             c.fetch_add(1, Ordering::Relaxed);
//!         });
//!     }
//! });
//!
//! job_system.wait_for_counter(&root);
//! // Note: 'root' implies spawning finished, but detached jobs might still be running.
//! // Use an external atomic or a shared Group Counter (see below) to track completion.
//! ```
//!
//! ### 4. Shared Counter (Task Groups)
//!
//! To track a group of jobs without allocating a counter for each one, use `spawn_with_counter`.
//!
//! ```no_run
//! use rustfiber::{JobSystem, Counter};
//!
//! let job_system = JobSystem::new(4);
//!
//! let root = job_system.run_with_context(|ctx| {
//!     // Create a counter for 100 jobs
//!     let group = Counter::new(100);
//!
//!     for _ in 0..100 {
//!         // All jobs share the same counter (auto-decremented on completion)
//!         ctx.spawn_with_counter(|_| {
//!             // Work...
//!         }, group.clone());
//!     }
//!
//!     // Efficiently wait for the entire group
//!     ctx.wait_for(&group);
//! });
//!
//! job_system.wait_for_counter(&root);
//! ```
//!
//! ### 5. Simplified API with Presets
//!
//! For common use cases, use preset constructors that provide optimized defaults:
//!
//! ```no_run
//! use rustfiber::JobSystem;
//!
//! // For gaming: high concurrency, avoids SMT
//! let gaming_system = JobSystem::for_gaming();
//!
//! // For data processing: balanced settings
//! let data_system = JobSystem::for_data_processing();
//!
//! // For low latency: minimal pools, CCD isolation
//! let latency_system = JobSystem::for_low_latency();
//! ```
//!
//! ### 6. Builder Pattern for Custom Configuration
//!
//! Use the builder for gradual configuration:
//!
//! ```no_run
//! use rustfiber::{JobSystem, PinningStrategy};
//!
//! let job_system = JobSystem::builder()
//!     .thread_count(8)
//!     .stack_size(256 * 1024)
//!     .initial_pool_size(32)
//!     .pinning_strategy(PinningStrategy::AvoidSMT)
//!     .build();
//! ```
//!
//! ## Best Use Cases
//!
//! ### Gaming
//! - Use `JobSystem::for_gaming()` for game engines
//! - High fiber counts, avoids SMT for cache performance
//! - Suitable for entity updates, physics, rendering
//!
//! ### Data Processing
//! - Use `JobSystem::for_data_processing()` for batch jobs
//! - Balanced settings for analytics, image processing
//! - Linear pinning for predictable performance
//!
//! ### Low Latency
//! - Use `JobSystem::for_low_latency()` for real-time systems
//! - Small pools, CCD isolation to reduce jitter
//! - Ideal for audio processing, control systems

pub mod allocator;
pub mod context;
pub mod counter;
pub mod fiber;
pub mod fiber_pool;
pub mod job;
pub mod job_system;
#[cfg(feature = "metrics")]
pub mod metrics;
pub mod topology;
pub mod tracing;
pub mod worker;

use serde::{Deserialize, Serialize};

/// Strategy for pinning worker threads to CPU cores.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum PinningStrategy {
    /// No pinning (standard OS scheduling).
    #[default]
    None,
    /// Linear pinning (worker i -> logical processor i).
    Linear,
    /// Pin to physical cores only (even-numbered logical processors), avoiding SMT contention.
    AvoidSMT,
    /// Pin to physical cores on the first CCD only (Logical 0, 2, ..., 14).
    /// This is optimized for AMD Ryzen systems to avoid CCD cross-over latency.
    CCDIsolation,
    /// Dynamic strategy: Prioritizes CCD0, then CCD1, then SMT threads based on load.
    TieredSpillover,
}

pub use context::Context;
pub use counter::Counter;
pub use job::Job;
pub use job_system::{GranularityHint, JobSystem, JobSystemBuilder, JobSystemError, Partitioner};

#[cfg(test)]
mod tests;
