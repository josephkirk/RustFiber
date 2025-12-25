# RustFiber

A high-performance fiber-based job system implementation in Rust, following the architectural principles from Naughty Dog's engine parallelization work.

## Features

- **Parallel Job Execution**: Efficiently schedule work across multiple CPU cores
- **Counter-Based Synchronization**: Track job completion without blocking
- **Thread-Safe**: Built on Rust's ownership model and proven concurrency primitives
- **High Throughput**: Capable of processing millions of jobs per second
- **Simple API**: Easy-to-use interface for job submission and synchronization

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

## Building

```bash
cargo build --release
cargo test
cargo run --release
```

## Documentation

See [ARCHITECTURE.md](ARCHITECTURE.md) for detailed documentation on the design, implementation, and usage.

## Performance

Typical performance on modern multi-core systems:
- 6+ million jobs/second throughput
- Sub-microsecond latency for simple jobs
- Efficient CPU utilization across all cores

## License

MIT
