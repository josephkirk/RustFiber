# RustFiber Roadmap

## v0.3: Micro-Optimizations & Kernel Integrations

### Large Pages (HugeTLB) Support
- **Goal**: Reduce TLB pressure for fiber stacks (512KB).
- **Plan**: Implement custom stack allocator for `corosensei` using `VirtualAlloc(MEM_LARGE_PAGES)` on Windows and `MAP_HUGETLB` on Linux.
- **Benefit**: Faster context switching and reduced cache pollution from translation misses.
- **Reference**: Investigated in v0.2 but deferred due to `corosensei` stack trait complexity.

### Topology-Aware Allocator
- **Goal**: Ensure all job-related allocations (box, closure captures) are strictly local.
- **Plan**: Integrate a per-worker memory arena (bump pointer) more deeply into `Job` struct creation to avoid `Global` allocator entirely.

### Hardware Topology Refinement
- **Goal**: Support >64 core systems (Windows Processor Groups).
- **Plan**: Integrate `hwloc` or advanced Windows APIs to handle processor groups correctly, as `sysinfo` flattens them.

### Manual Stack Prefaulting (First-Touch)
- **Goal**: Guarantee NUMA-local physical pages for fiber stacks.
- **Status**: Deferred.
- **Reason**: Windows Guard Page mechanism (`STATUS_GUARD_PAGE_VIOLATION`) conflicts with manual probing of `corosensei` stacks. Requires custom stack allocator to handle `VirtualAlloc` directly.

