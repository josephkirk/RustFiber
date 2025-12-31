# Strategy: Reducing Allocation Overhead

## Context
As detailed in `optimize_globalsubmission_overhead.md`, we successfully reduced the job submission overhead by ~16% using Local Queues. However, the Frame Allocator path is still slower than the Heap path (~6.8ms vs ~5.0ms) in high-throughput scenarios.

The remaining bottleneck is the **Heap Allocation of Counters**. Currently, every `spawn_job` call creates a new `Counter` (wrapping an `Arc<Inner>`), forcing a malloc even when the job closure itself is frame-allocated.

## Objective
Eliminate heap allocations for fine-grained job spawning to allow the Frame Allocator to strictly outperform Heap Allocation.

## Implementation Applied
We introduced new spawning APIs that avoid creating a new `Counter` for every job.

1.  **Detached Jobs (`spawn_detached`)**: For jobs where the parent does not need to wait for individual completion. Passes `None` for the counter.
2.  **Shared Counter (`spawn_with_counter`)**: Allows multiple jobs to share a single pre-allocated `Counter`.

## Results
We verified the impact with `tests/benchmark_detached.rs` (50k jobs):

-   **spawn_job (Baseline)**: ~10.0ms (new Counter per job)
-   **spawn_detached (Zero Alloc)**: **~5.2ms** (No Counter)
-   **spawn_with_counter (Shared)**: ~8.8ms (Shared Counter)

**Analysis:**
-   `spawn_detached` is roughly **2x faster** than creating a counter for every job.
-   It achieves performance parity (or better) with the highly-optimized Heap Allocation path (which was ~5.0ms with empty closures).
-   This proves that **Allocation Free Spawning** (Frame Allocator + Local Queue + No Counter) is the fastest path for fine-grained parallelism.

## Recommendations
-   Use `spawn_detached` for high-frequency, small tasks where individual tracking is not required (e.g., using a shared atomic for completion or waiting on the frame boundary).
-   Use `spawn_with_counter` when grouping tasks, but be aware of Atomic Contention on the shared counter.
-   Use `spawn_job` only when individual granularity waiting is required.
