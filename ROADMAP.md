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

### Parallel For Optimization
- **Goal**: Reduce contention in massive nested parallelism by using local queues instead of global injector.
- **Plan**: Refactor `parallel_for` to be generic over submission target, allowing `Context::parallel_for` to submit to local queue.
- **Benefit**: Better scalability for deeply nested parallel algorithms.

