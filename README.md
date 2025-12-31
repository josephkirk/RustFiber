# RustFiber

[![CI](https://github.com/josephkirk/RustFiber/actions/workflows/ci.yml/badge.svg)](https://github.com/josephkirk/RustFiber/actions/workflows/ci.yml)
[![Deploy Documentation](https://github.com/josephkirk/RustFiber/actions/workflows/docs.yml/badge.svg)](https://github.com/josephkirk/RustFiber/actions/workflows/docs.yml)

A high-performance fiber-based job system implementation in Rust, following the architectural principles from Naughty Dog's engine parallelization work.

> **Hobby Project Alert!** This is a learning experiment where I'm geeking out on fibers and high-performance job systems. It's meant for fun and exploration, so use it at your own risk! If it explodes (metaphorically, of course), you've been warned. 

## Features

- **Parallel Job Execution**: Efficiently schedule work across multiple CPU cores
- **Nested Parallelism**: Jobs can spawn child jobs using Context for recursive decomposition
- **Counter-Based Synchronization**: Track job completion without blocking
- **Thread-Safe**: Built on Rust's ownership model and proven concurrency primitives
- **Job Prioritization**: Schedule critical tasks with `High`, `Normal`, or `Low` priority
- ✅ **Safe Parallel Iterators**: Rayon-like `par_iter` and `par_iter_mut` for slices.
- ✅ **High Throughput**: Capable of millions of jobs per second
- ✅ **Simple API**: Easy to use interface for job submission and synchronization
- ✅ **Performance Monitoring**: Optional metrics collection for debugging throughput and bottlenecks

## v0.2 Optimizations (New)

- **Stack Reuse**: Eliminated `mmap` overhead by recycling fiber stacks.
- **Adaptive Spinning**: Reduced latency for fine-grained dependency chains.
- **Strategy-Aware Scheduling**: Hybrid Local/Global scheduling to optimize for both cache affinity (`Linear`) and load balancing (`TieredSpillover`).
- **Frame Allocator**: Bump allocation for jobs, eliminating heap fragmentation and locking.
- **Zero-Overhead Submission**: Lock-free Local Queue submission and detached jobs for maximum throughput.
- **Cache Alignment**: Critical structures (`Counter`, `WaitNode`) and global states are explicitly aligned/padded to prevent false sharing on high-core CPUs.
- **Job Batching**: `parallel_for_chunked` API for processing millions of items with optimal granularity and SIMD/cache locality.
- **Startup Optimization**: Reduced initialization time from 4-5ms to <1ms through incremental fiber pool growth.
- **NUMA Awareness Framework**: Infrastructure for NUMA-local memory placement (currently disabled on Windows due to compatibility constraints).

## Quick Start

```rust
use rustfiber::JobSystem;

// Create a job system with 4 worker threads
let job_system = JobSystem::new(4);

// Submit a job
let counter = job_system.run(|| {
    println!("Hello from a job!");
});

// Wait for completion
job_system.wait_for_counter(&counter);
job_system.shutdown();
```

### Nested Parallelism with Context

```rust
use rustfiber::JobSystem;

let job_system = JobSystem::new(4);

// Jobs can spawn child jobs for recursive decomposition
let counter = job_system.run_with_context(|ctx| {
    // Subdivide work across multiple child jobs
    let mut counters = vec![];
    
    for i in 0..4 {
        let child = ctx.spawn_job(move |_| {
            println!("Child job {} running", i);
        });
        counters.push(child);
    }
    
    // Wait for all children to complete
    for child in counters {
        ctx.wait_for(&child);
    }
});

job_system.wait_for_counter(&counter);
job_system.shutdown();
```

### Safe Parallel Iterators (Rayon-like)

Process slices in parallel with a high-level API that handles batching and lifetime management automatically. Supports capturing stack variables!

```rust
use rustfiber::{JobSystem, ParallelSliceMut};

let job_system = JobSystem::new(4);
let mut data = vec![0; 100_000];

// Safe parallel mutable iteration
data.par_iter_mut(&job_system).for_each(|x| {
    *x += 1;
});
// No need to manually wait; for_each blocks until completion.
```

### Simplified API with Presets

For common use cases, use preset constructors that provide optimized defaults:

```rust
use rustfiber::JobSystem;

// For gaming: high concurrency, avoids SMT
let gaming_system = JobSystem::for_gaming();

// For data processing: balanced settings
let data_system = JobSystem::for_data_processing();

// For low latency: minimal pools, CCD isolation
let latency_system = JobSystem::for_low_latency();
```

### Builder Pattern for Custom Configuration

Use the builder for gradual configuration:

```rust
use rustfiber::{JobSystem, PinningStrategy};

let job_system = JobSystem::builder()
    .thread_count(8)
    .stack_size(256 * 1024)
    .initial_pool_size(32)
    .pinning_strategy(PinningStrategy::AvoidSMT)
    .build();
```

### Performance Monitoring

Enable optional metrics collection to track job throughput and queue depths. This feature requires the `metrics` Cargo feature flag to be enabled.

First, enable the feature in your `Cargo.toml`:

```toml
[dependencies]
rustfiber = { version = "0.1", features = ["metrics"] }
```

Then use it in your code:

```rust
use rustfiber::JobSystem;

let job_system = JobSystem::builder()
    .enable_metrics(true)
    .build();

// Run some jobs...

if let Some(metrics) = job_system.metrics() {
    println!("Jobs completed: {}", metrics.jobs_completed);
    println!("Jobs/sec: {:.2}", metrics.jobs_per_second());
    println!("Local queue depth: {}", metrics.local_queue_depth());
}
```

Metrics are collected with minimal overhead when enabled.

## Building

```bash
cargo build --release
cargo test
cargo run --release
```

## Documentation

See [ARCHITECTURE.md](ARCHITECTURE.md) for detailed documentation on the design, implementation, and usage.

See [BENCHMARK.md](BENCHMARK.md) for information about running performance benchmarks.

### Benchmarks

Run criterion micro-benchmarks:

```bash
# Run all benchmarks
cargo bench

# Run specific benchmark
cargo bench --bench fiber_switch    # Context switch latency (~18ns)
cargo bench --bench throughput      # Job throughput (~14M jobs/sec)
cargo bench --bench latency         # Scheduling latency
cargo bench --bench work_stealing   # Work-stealing stress
cargo bench --bench scientific      # NAS patterns (EP, MG, CG)
cargo bench --bench transform       # Game engine hierarchy
cargo bench --bench producer_consumer
cargo bench --bench allocation
cargo bench --bench startup
```

HTML reports are generated in `target/criterion/report/index.html`.


## Performance

Typical performance on modern multi-core systems:
- 6+ million jobs/second throughput
- Sub-microsecond latency for simple jobs
- Efficient CPU utilization across all cores
- **Startup Latency**: <1ms (optimized) due to incremental fiber pool allocation.
    - *Note*: This prevents runtime allocation glitches in game engines while maintaining fast startup.
    - CLI tools can reduce `initial_pool_size` for even faster startup.

## Limitations

- **NUMA Prefaulting**: Disabled on Windows due to guard page violations that prevent safe memory writes to stack regions. The framework is in place for future implementation when Windows compatibility issues are resolved.
- **Large Page Support**: Deferred to v0.3. Currently uses standard 4KB pages for fiber stacks.

## License

MIT
