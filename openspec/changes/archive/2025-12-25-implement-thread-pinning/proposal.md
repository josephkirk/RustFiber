# Proposal: Implement Thread Pinning Strategies

This proposal introduces configurable thread pinning strategies to the RustFiber job system to optimize performance on modern multi-core architectures (like AMD Ryzen).

## Why
Benchmarks have identified three major performance bottlenecks:
1.  **CCD Cross-Over Delay**: Latency incurred when work is stolen or data is accessed across different Core Complex Dies (CCDs).
2.  **Work-Stealing "Thievery" Overhead**: Excessive stealing across non-optimal core boundaries.
3.  **SMT Resource Fighting**: Competitive resource usage ( L1/L2 cache, ALU) between logical processors on the same physical core.

## What Changes
We will introduce a `PinningStrategy` enum and update the `JobSystem` and `WorkerPool` to support selective pinning:

- **Avoid SMT Contention**: Option to pin worker threads ONLY to physical cores (even-numbered logical processors), ensuring dedicated resources.
- **CCD Isolation**: Option to restrict workers to the first CCD (e.g., first 8 physical cores) for ultra-stable, low-latency execution.
- **Configurable Core Mapping**: An internal mechanism to map worker IDs to specific `CoreId`s based on the selected strategy.

### Rust API
- Add `PinningStrategy` to `src/lib.rs`.
- Update `JobSystem` to accept `PinningStrategy`.
- Refactor `WorkerPool::new_with_affinity` to use the strategy for core mapping.
