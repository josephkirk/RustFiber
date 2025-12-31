# Implement Frame Linear Allocators

## Summary
Introduce per-worker linear (bump) allocators to replace global heap allocations (`Box::new`) for job creation. This change aims to eliminate lock contention during high-frequency job spawning, meeting the requirement for "Frame Linear Allocators".

## Problem Statement
The current implementation uses `Box<dyn FnOnce>` for every job submitted to the system. This leads to:
1.  **Global Lock Contention**: `Box::new` typically uses the global allocator (e.g., system malloc), which involves locking. At high frequency (tens of thousands of jobs/frame), this becomes a bottleneck.
2.  **Memory Fragmentation & Cache Misses**: Individual allocations scatter job closures across the heap.
3.  **Performance Degradation**: The overhead of allocation/deallocation per job consumes significant CPU time.

## Proposed Solution
1.  **Frame Allocator**: Implement a `FrameAllocator` (linear/bump allocator) that allocates memory from a large contiguous chunk.
2.  **Per-Worker Instance**: Each worker thread maintains its own allocator instance to avoid locks entirely during allocation.
3.  **Frame Reset**: A mechanism to reset the allocator (pointer rewinding) at a synchronization point (end of frame).
4.  **Job System Integration**: Update `JobSystem` and `Job` to support allocating closures into the `FrameAllocator` instead of the global heap.
