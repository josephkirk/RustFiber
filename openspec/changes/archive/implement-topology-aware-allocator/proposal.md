# Topology-Aware Allocator

## Goal
Ensure all job-related allocations (box, closure captures) are strictly local by integrating a per-worker memory arena (bump pointer) more deeply into `Job` struct creation to avoid the `Global` allocator entirely.

## Requirements
- **Local Allocation**: Jobs created within a worker thread MUST use that worker's local `FrameAllocator` (memory arena) instead of the global heap.
- **Topology Awareness**: The allocator must be aware of the underlying NUMA topology (implied by per-worker locality).
- **Zero Overhead**: The integration should minimize overhead, ideally matching the performance of raw pointer manipulation or stack allocation.
- **Usage Ergonomics**: Creating a job locally should be as simple as `Job::spawn(...)` or similar, without manually dragging `&mut FrameAllocator` everywhere if possible, or by streamlining the API.
