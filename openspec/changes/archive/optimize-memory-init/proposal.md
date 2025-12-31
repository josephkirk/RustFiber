# Proposal: Optimized Memory Initialization & NUMA-Local Stacks

## Why
1.  **Startup Stutter**: Eager allocation of 1GB fiber stacks causes a 4-5ms frame spike (or 100ms+ in larger configs).
2.  **NUMA Locality**: OS-level lazy allocation might fault pages on the wrong node if allocation happens on the main thread (before worker hand-off), or if the OS heuristic is imperfect.
3.  **TLB Pressure**: 4KB pages for large stacks (512KB) increase TLB misses.

## What Changes
1.  **NUMA-Local Zeroing**: Force a write (memset) to the fiber stack memory *on the worker thread* immediately upon creation. This guarantees physical pages are allocated on the local NUMA node (First-Touch policy). **STATUS: DISABLED** - Windows guard page violations prevent safe implementation.
2.  **Interleaved Pre-warming**: Change `FiberPool` to support incremental growth. Instead of allocating 128 fibers at once in `new()`, allocate a small batch (e.g., 16) and let the rest be allocated progressively during idle worker cycles. **STATUS: IMPLEMENTED**
3.  **Large Pages**: Use `2MB` pages (where available) for fiber stacks to reduce TLB usage. (Note: This requires custom allocator support in `corosensei` or manual `VirtualAlloc`). **STATUS: DEFERRED** to v0.3.

## Current Implementation Status
- ✅ **Interleaved Pre-warming**: Reduces startup time from ~4-5ms to <1ms
- ❌ **NUMA-Local Zeroing**: Disabled due to Windows compatibility issues
- ⏳ **Large Pages**: Deferred to future version
- ✅ **Configuration Control**: Added `prefetch_pages` option for future NUMA implementation

## Verification Plan
1.  **Benchmark**: Verify "Startup Latency" drop in linear benchmarks. ✅ **COMPLETED**
2.  **Profiling**: Confirm separate NUMA node memory usage (if possible via tools) or verify execution consistency. ❌ **NOT IMPLEMENTED** - NUMA zeroing disabled
