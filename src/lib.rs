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
//! N hardware threads (worker threads). Key components include:
//!
//! - **Fibers**: Lightweight execution contexts that can be suspended and resumed
//! - **Job Queue**: Thread-safe queue for pending work units
//! - **Counters**: Synchronization primitives for tracking job completion
//! - **Worker Threads**: OS threads that execute fibers from the job queue
//!
//! ## Example
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

pub mod context;
pub mod counter;
pub mod fiber;
pub mod fiber_pool;
pub mod job;
pub mod job_system;
pub mod worker;
pub mod allocator;

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
pub use job_system::JobSystem;

#[cfg(test)]
mod tests;
