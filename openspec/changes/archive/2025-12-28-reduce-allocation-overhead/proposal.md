# Reduce Allocation Overhead

## Problem
Currently, `Context::spawn_job` always creates a new `Counter` (wrapping an `Arc<Inner>`). This heap allocation dominates the cost of spawning small, frame-allocated jobs, masking the performance benefits of the Frame Allocator. Benchmarks show a 2-3ms overhead for 50k jobs purely due to this allocation.

## Solution
Introduce new spawning APIs that avoid creating a new `Counter` for every job.

1.  **Detached Jobs (`spawn_detached`)**: For jobs where the parent does not need to wait for individual completion. Passes `None` for the counter.
2.  **Grouped Jobs (`spawn_group`)**: For jobs that share a single synchronization point. Reuses an existing `Counter` for multiple jobs.

## Key Changes
-   **Context API**:
    -   `spawn_detached(&self, work: F)`
    -   `spawn_group(&self, work: F, group: &Counter)` (or similar API)
-   **Job Creation**:
    -   Ensure `Job::new` (or internal variants) can handle `None` counter (already supported).
    -   Ensure `FrameAllocator` path handles `None` counter.

## Benefits
-   Eliminates `malloc` for fire-and-forget or batched jobs.
-   Unlocks true "zero-allocation" nested parallelism when used with Frame Allocator.
-   Expected performance closer to ~1-2ms for 50k jobs (vs ~6.8ms currently).
