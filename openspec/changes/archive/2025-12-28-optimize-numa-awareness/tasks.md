- [x] Map topology using `sysinfo` (Cores -> NUMA Nodes). **[IMPLEMENTED: hwloc primary, sysinfo fallback]**
- [x] Refactor `Worker` initialization to allocate `FiberPool` and `FrameAllocator` lazily inside the worker thread (ensure First-Touch policy).
- [x] Implement `StealOrder` strategy: Precompute stealing lists (Local first, then Remote).
- [x] Update `Worker::steal_task` to use the `StealOrder`.
- [x] Verify using `NAS CG` benchmark on high-core count.

### Regression Fixes (v0.2.1)
- [x] Investigate performance regression on high-core counts (Fixed Steal Order & Optimized Stealer List).
