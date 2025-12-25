# RustFiber - High-Performance Fiber-Based Job System

A Rust implementation of a fiber-based job system following the architectural principles from Naughty Dog's engine parallelization work presented at GDC 2015.

## Overview

RustFiber provides a high-performance job system that enables efficient task parallelism using an M:N threading model, where M fibers (lightweight execution contexts) are multiplexed onto N hardware threads.

## Architecture

The system consists of several key components:

### Core Components

1. **JobSystem** - The main entry point for scheduling and managing parallel work
2. **Context** - Safe interface for jobs to access the job system for nested parallelism
3. **Worker Pool** - A pool of OS threads that execute jobs
4. **Job Queue** - Thread-safe queue for pending work units
5. **Counters** - Synchronization primitives for tracking job completion
6. **Fibers** - Lightweight execution contexts for jobs

### Design Principles

- **Work-Stealing Model**: Efficient job distribution using local queues, a global injector, and crossbeam-deque
- **Thread Pinning Strategies**: Optimized core mapping for x86 and Ryzen architectures
- **Tiered Spillover System**: Dynamic core activation based on system load
- **Counter-Based Synchronization**: Efficient tracking of job completion without blocking
- **Lock-Free Communication**: Using crossbeam-deque for minimal contention
- **Zero-Cost Abstractions**: Leveraging Rust's type system for safety without overhead

## Features

- ✅ **Parallel Job Execution**: Schedule work across multiple CPU cores
- ✅ **Nested Parallelism**: Jobs can spawn child jobs using Context for recursive work decomposition
- ✅ **Work-Stealing Scheduler**: Dynamic load balancing between worker threads
- ✅ **Advanced Pinning**: Strategies like `AvoidSMT`, `CCDIsolation`, and `TieredSpillover`
- ✅ **Counter-Based Waiting**: Wait for job completion without busy-waiting
- ✅ **Thread-Safe**: Built on Rust's ownership model and crossbeam primitives
- ✅ **High Throughput**: Capable of millions of jobs per second
- ✅ **Simple API**: Easy to use interface for job submission and synchronization

## Thread Pinning Strategies

The system supports several strategies to optimize performance for different hardware:

- **None**: Default OS scheduling.
- **Linear**: Simple 1:1 mapping of workers to logical processors.
- **AvoidSMT**: Pins only to physical cores, avoiding hyperthreading contention.
- **CCDIsolation**: Pins to physical cores on the first CCD only (Ryzen optimization).
- **TieredSpillover**: A dynamic system that activates physical cores first (Tier 1), then spills over to secondary CCDs (Tier 2), and finally SMT threads (Tier 3) only when load thresholds (80% utilization) are exceeded.

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
rustfiber = "0.1.0"
```

## Usage

### Basic Example

```rust
use rustfiber::JobSystem;

// Create a job system with 4 worker threads
let job_system = JobSystem::new(4);

// Submit a simple job
let counter = job_system.run(|| {
    println!("Hello from a job!");
});

// Wait for completion
job_system.wait_for_counter(&counter);

// Clean shutdown
job_system.shutdown();
```

### Parallel Computation

```rust
use rustfiber::JobSystem;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

let job_system = JobSystem::new(4);
let sum = Arc::new(AtomicUsize::new(0));

// Submit multiple jobs
let mut jobs: Vec<Box<dyn FnOnce() + Send>> = Vec::new();
for i in 0..100 {
    let sum_clone = sum.clone();
    jobs.push(Box::new(move || {
        sum_clone.fetch_add(i, Ordering::SeqCst);
    }));
}

let counter = job_system.run_multiple(jobs);
job_system.wait_for_counter(&counter);

println!("Sum: {}", sum.load(Ordering::SeqCst));
job_system.shutdown();
```

### Using Counters Directly

```rust
use rustfiber::{JobSystem, Counter};

let job_system = JobSystem::new(4);
let counter = Counter::new(10);

// Submit 10 jobs that share a counter
for _ in 0..10 {
    let counter_clone = counter.clone();
    job_system.run(move || {
        // Do work...
        counter_clone.decrement();
    });
}

// Wait for all jobs to complete
job_system.wait_for_counter(&counter);
```

### Nested Parallelism with Context

The Context type enables safe nested parallelism, allowing jobs to spawn child jobs:

```rust
use rustfiber::JobSystem;

let job_system = JobSystem::new(4);

// Parent job spawns child jobs
let counter = job_system.run_with_context(|ctx| {
    // Subdivide work into parallel chunks
    let mut counters = vec![];
    
    for chunk_id in 0..4 {
        let child = ctx.spawn_job(move |ctx| {
            // Child can spawn its own children
            let grandchild = ctx.spawn_job(move |_| {
                println!("Grandchild of chunk {} running", chunk_id);
            });
            ctx.wait_for(&grandchild);
        });
        counters.push(child);
    }
    
    // Wait for all children
    for c in counters {
        ctx.wait_for(&c);
    }
});

job_system.wait_for_counter(&counter);
job_system.shutdown();
```

This pattern is useful for:
- Recursive algorithms (quicksort, parallel reduce)
- Hierarchical decomposition (particle systems, physics simulation)
- Fork-join parallelism
- Producer-consumer patterns

## Performance

On a typical multi-core system, RustFiber achieves:
- **6+ million jobs/second** throughput
- **Sub-microsecond** latency for simple jobs
- **Efficient CPU utilization** across all cores

## Building

```bash
# Debug build
cargo build

# Release build (optimized)
cargo build --release

# Run tests
cargo test

# Run example
cargo run --release
```

## Testing

The project includes comprehensive tests:

```bash
cargo test
```

Tests cover:
- Counter synchronization
- Job execution
- Parallel job scheduling
- Worker pool management
- High-throughput scenarios

## Implementation Details

### Thread Safety

The system uses:
- `Arc` for shared ownership
- `AtomicUsize` for counter operations and active worker tracking
- `crossbeam::deque` for lock-free work-stealing queues
- Rust's `Send` and `Sync` traits for compile-time safety

### Scheduling

RustFiber uses a sophisticated **Work-Stealing** scheduler:
1. **Local Queue**: Each worker has a local FIFO queue for minimal contention.
2. **Global Injector**: Jobs submitted from outside the systems are deposited here.
3. **Stealing Logic**: Idle workers first check their local queue, then the global injector, and finally attempt to "steal" work from other workers' queues to ensure balanced load.

### Memory Management

- Jobs are heap-allocated boxed closures
- Counters use atomic reference counting
- Worker threads own their receiver endpoints

### Context and Nested Parallelism

The Context type provides safe access to the job system from within jobs:

**Design**: Context holds a reference to the JobSystem with lifetime checking. When a job is created with `run_with_context`, the JobSystem pointer is stored as a `usize` (to make it `Send`) and converted back safely when the job executes.

**Safety Guarantees**:
- The JobSystem is guaranteed to outlive all jobs it creates
- Jobs are fully executed before `JobSystem::shutdown()` completes
- Context lifetime is bounded by job execution scope
- No unsafe pointer dereferencing outside controlled execution paths

**API**:
- `ctx.spawn_job(|ctx| ...)` - Spawn a single child job
- `ctx.spawn_jobs(vec![...])` - Spawn multiple child jobs
- `ctx.wait_for(&counter)` - Wait for child jobs to complete
- `ctx.yield_now()` - Cooperatively yield execution (thread-level for now)

This enables patterns like:
- Recursive parallel algorithms
- Hierarchical work decomposition
- Fork-join parallelism
- Cooperative yielding in long-running tasks

## Future Enhancements

Potential improvements for production use:

1. **True Fiber Context Switching**: Implement stackful fibers with context switching
2. **Priority Queues**: Support job priorities
3. **Fiber Yielding**: Enhance cooperative yielding beyond thread-level to true fiber suspension
4. **Instrumentation**: Add profiling and performance metrics
5. **Async Integration**: Bridge with Rust's async/await

## Solved Problems

### Context Type for Nested Parallelism (✅ Implemented)

Previously, applications like Arcel needed unsafe workarounds to access the JobSystem from within jobs:

```rust
// OLD: Unsafe workaround (no longer needed)
let job_system_ptr_addr = &self.job_system as *const JobSystem as usize;
jobs.push(Box::new(move || unsafe {
   let js_ptr = job_system_ptr_addr as *const JobSystem;
   (*js_ptr).run(...);  // Unsafe pointer dereference
}));
```

The Context type provides a safe, ergonomic alternative:

```rust
// NEW: Safe with Context
let counter = job_system.run_with_context(|ctx| {
    let child = ctx.spawn_job(|_| {
        // Child work
    });
    ctx.wait_for(&child);
});
```

**Benefits**:
- ✅ Type-safe: Lifetime-checked references instead of raw pointers
- ✅ Ergonomic: Clean, composable API
- ✅ Maintainable: No unsafe code required by users
- ✅ Future-proof: Foundation for advanced features like fiber yielding

## References

This implementation is inspired by:

- Christian Gyrling's GDC 2015 talk: "Parallelizing the Naughty Dog Engine Using Fibers"
- Various fiber job system implementations in the game industry
- Rust's async runtime architectures (Tokio, async-std)

## License

MIT License - See LICENSE file for details

## Contributing

Contributions are welcome! Please feel free to submit issues or pull requests.
