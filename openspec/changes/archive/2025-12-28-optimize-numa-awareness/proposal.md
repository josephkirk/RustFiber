# Proposal: NUMA Awareness

## Why
On high-core-count systems (e.g., AMD Threadripper/EPYC, Dual-Socket Intel), performance often regresses when scaling beyond a single NUMA domain (e.g., >16 cores).
- **Symptom**: `NAS CG` benchmark performs worse on 32 cores than 16 cores.
- **Root Cause**: Workers running on Socket 1 stealing jobs or accessing data allocated on Socket 0 incur high latency across the interconnect (Infinity Fabric/UPI).
- **Current State**: We have `PinningStrategy`, but memory allocation (FrameAllocator, Fiber Stacks) and Work Stealing are not NUMA-aware.

## What Changes
Implement **NUMA Awareness** to enforce locality:
1.  **Topology Discovery**: Detect NUMA nodes and core mappings at startup using hwloc (with sysinfo fallback).
2.  **NUMA-Local Allocation**: Ensure `FrameAllocator` and `Fiber` memory pages are touched/faulted by the thread that owns them, strictly enforcing explicit NUMA binding where possible.
3.  **Hierarchical Work Stealing**: Sibling stealing (same L3/NUMA) should be prioritized over remote stealing.

## Current Implementation Status
- ✅ **Hierarchical Work Stealing**: Implemented with topology-aware steal ordering
- ✅ **NUMA-Local Allocation**: FrameAllocator and FiberPool allocated in worker threads (first-touch policy)
- ✅ **Framework**: Topology detection framework in place
- ⚠️ **NUMA Detection**: Uses hwloc for accurate detection, falls back to single NUMA node
- ❌ **Multi-NUMA Validation**: Cannot verify benefits without multi-socket test hardware

## Impact
- **Performance**: Should eliminate regression in `NAS CG` and improved scaling for bandwidth-heavy workloads.
- **Complexity**: Adds dependency on topology discovery (potentially `sysinfo` or `hwloc`, though `sysinfo` is already used).
