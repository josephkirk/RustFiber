# RustFiber Roadmap

## v0.3: Micro-Optimizations & Kernel Integrations

### Large Pages (HugeTLB) Support
- **Goal**: Reduce TLB pressure for fiber stacks (512KB).
- **Plan**: Implement custom stack allocator for `corosensei` using `VirtualAlloc(MEM_LARGE_PAGES)` on Windows and `MAP_HUGETLB` on Linux.
- **Benefit**: Faster context switching and reduced cache pollution from translation misses.
- **Reference**: Investigated in v0.2 but deferred due to `corosensei` stack trait complexity.

### Hardware Topology Refinement
- **Goal**: Support >64 core systems (Windows Processor Groups).
- **Plan**: Integrate `hwloc` or advanced Windows APIs to handle processor groups correctly, as `sysinfo` flattens them.

### Manual Stack Prefaulting (First-Touch)
- **Goal**: Guarantee NUMA-local physical pages for fiber stacks.
- **Status**: Deferred.
- **Reason**: Windows Guard Page mechanism (`STATUS_GUARD_PAGE_VIOLATION`) conflicts with manual probing of `corosensei` stacks. Requires custom stack allocator to handle `VirtualAlloc` directly.


---

## v0.4: Performance Optimizations (Based on Benchmark Analysis)

*Identified from criterion benchmarks run 2025-12-29*

### 1. Startup Time Optimization (HIGH PRIORITY)
- **Observation**: Startup takes ~30ms, far exceeding the <1ms claim in README.
- **Root Cause**: Worker thread creation overhead (32 `std::thread::spawn` calls).
- **Plan**:
  - Lazy worker initialization (spawn on first job submission).
  - Thread pool pre-warming during idle periods.
  - Consider using `std::thread::available_parallelism()` for default count.
- **Target**: <5ms startup for 32-thread system.

### 2. Sequential Spawn Bottleneck (MEDIUM PRIORITY)
- **Observation**: Throughput is ~14M jobs/sec regardless of thread count (1-32).
- **Root Cause**: Jobs are spawned sequentially in a single `for` loop within one fiber.
- **Plan**:
  - Implement `spawn_batch` API that pushes multiple jobs atomically.
  - Parallel job generation from multiple workers using work-stealing on the spawn loop itself.
  - Amortize Counter operations with batch decrement.
- **Target**: Near-linear scaling for parallel spawn (56M+ jobs/sec on 32 threads).

### 3. Parking Timeout Reduction
- **Observation**: `job_system_cold` shows ~500µs latency dominated by 1ms parking timeout.
- **Trade-off**: Lower timeout = faster wake, higher timeout = less CPU spinning.
- **Plan**:
  - Adaptive timeout based on recent activity.
  - Immediate wake path for high-priority jobs (bypass parking entirely).
- **Target**: <100µs cold path latency.

### 4. Work-Stealing Efficiency
- **Observation**: Imbalanced workload throughput is ~8M elem/s, lower than balanced.
- **Plan**:
  - Implement work-stealing randomization to avoid convoy effects.
  - Consider victim selection based on queue depth hints.
- **Target**: >10M elem/s for imbalanced workloads.

### 5. Counter Contention Reduction
- **Observation**: High throughput benchmarks show atomic contention on shared Counter.
- **Plan**:
  - Per-worker counter shards with periodic synchronization.
  - Hierarchical counters for task groups.
- **Target**: Reduce atomic operations per job by 50%.

---

## v0.5: Advanced Features

### Async/Await Integration
- Bridge `JobSystem` with Tokio/async-std for mixed workloads.
- Allow fibers to await async operations without blocking workers.

### GPU Job Dispatch
- Integrate with `wgpu` for compute shader job submission.
- Unified scheduling for CPU and GPU workloads.

### Profiling Integration
- Tracy/Perfetto instrumentation for visualization.
- Real-time metrics dashboard.

