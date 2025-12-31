# Design: NUMA Awareness

## Topology Mapping
We need a robust way to map `WorkerID` -> `CoreID` -> `NumaNode`.
- **Primary**: Use `hwloc` crate for accurate NUMA node detection and core mapping.
- **Fallback**: Use `sysinfo` if hwloc is unavailable (limited to single NUMA node).
- Store a `Topology` struct in `JobSystem` with core-to-node and node-to-cores mappings.

## Memory Locality
Rust/OS memory allocation follows "First Touch" policy for NUMA systems.
- **Fiber Stacks**: ✅ **IMPLEMENTED** - FiberPool created in worker threads during startup.
- **Frame Allocator**: ✅ **IMPLEMENTED** - FrameAllocator initialized in worker threads with first-touch allocation.

## Hierarchical Work Stealing
✅ **IMPLEMENTED** - Replaced random stealing with topology-aware ordering.
- **Algorithm**:
  1. **Local Phase**: Prioritize stealing from siblings in the same NUMA node.
  2. **Remote Phase**: Fall back to stealing from workers on remote NUMA nodes.
  3. **Rotation**: Rotate steal order to prevent convoy effects.
- **Optimization**: Pre-calculated steal order with flattened stealer references for low-latency access.

## Unifying PinningStrategy
✅ **IMPLEMENTED** - Simplified worker logic to use unified topology-based stealing.
- `PinningStrategy` remains user configuration for core binding.
- All strategies (Linear, AvoidSMT, CCDIsolation, TieredSpillover) now use the same topology-aware steal logic.
- Removed strategy-specific code paths in favor of generic topology-driven behavior.

## Implementation Notes
- **hwloc Dependency**: Added for reliable NUMA detection on multi-socket systems.
- **Fallback Safety**: Single NUMA node assumption when hwloc unavailable (safe but suboptimal).
- **Testing**: Added topology validation tests, but multi-NUMA benefits require multi-socket hardware for verification.
