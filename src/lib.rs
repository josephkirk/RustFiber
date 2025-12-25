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

pub mod counter;
pub mod fiber;
pub mod job;
pub mod job_system;
pub mod worker;

pub use counter::Counter;
pub use job::Job;
pub use job_system::JobSystem;

#[cfg(test)]
mod tests;
