# Optimize Cache Alignment

This change introduces cache alignment and padding to critical synchronization primitives in RustFiber to prevent false sharing and improve scalability on high-core-count systems.

## User Review Required

> [!IMPORTANT]
> This change introduces a new dependency `crossbeam-utils` to leverage `CachePadded`.

## Proposed Changes

### Cache Alignment

#### [MODIFY] [counter.rs](file:///src/counter.rs)
- Wrap `InnerCounter` fields or the struct itself with padding to ensure it occupies a full cache line (or distinct cache lines for fields if necessary, though struct alignment is usually sufficient for `Arc` allocations).
- Specifically, align `InnerCounter` to prevent "ping-pong" invalidation between adjacent counter allocations.

#### [MODIFY] [fiber.rs](file:///src/fiber.rs)
- Ensure `WaitNode` is cache-aligned to prevent false sharing when nodes are stack-allocated or densely packed.

#### [MODIFY] [Cargo.toml](file:///Cargo.toml)
- Add `crossbeam-utils`.

## Verification Plan

### Automated Tests
- Run existing test suite to ensure no regression in logic: `cargo test`
- Run benchmarks to verify performance improvement (specifically on high core counts if available, otherwise ensure no regression): `cargo bench`
- **Critical Verification**: Run `NAS MG` and `NAS CG` benchmarks to verify false sharing mitigation.


### Manual Verification
- Verify `std::mem::align_of` and `std::mem::size_of` for modified structures via a temporary test or script.
