# RustFiber

[![CI](https://github.com/josephkirk/RustFiber/actions/workflows/ci.yml/badge.svg)](https://github.com/josephkirk/RustFiber/actions/workflows/ci.yml)
[![Deploy Documentation](https://github.com/josephkirk/RustFiber/actions/workflows/docs.yml/badge.svg)](https://github.com/josephkirk/RustFiber/actions/workflows/docs.yml)

A high-performance fiber-based job system implementation in Rust, following the architectural principles from Naughty Dog's engine parallelization work.

!!! note
    This is a test to check Github Copilot capabilities

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

See [BENCHMARKS.md](BENCHMARKS.md) for information about running performance benchmarks.

## Benchmarks

Run comprehensive performance benchmarks:

```bash
# Using Python with uv (recommended)
uv run run_benchmarks.py

# Or with regular Python
python3 run_benchmarks.py
```

This will test the system with:
1. **Million Tiny Tasks (Fibonacci)** - Task creation and scheduling efficiency
2. **Recursive Task Decomposition (QuickSort)** - Work-stealing efficiency
3. **Producer-Consumer Stress Test** - Throughput under contention
4. **NAS Parallel Benchmarks** - Real-world computational patterns (EP, MG, CG)

Benchmark results and graphs are saved to the `docs/` folder.

## Performance

Typical performance on modern multi-core systems:
- 6+ million jobs/second throughput
- Sub-microsecond latency for simple jobs
- Efficient CPU utilization across all cores

## License

MIT
