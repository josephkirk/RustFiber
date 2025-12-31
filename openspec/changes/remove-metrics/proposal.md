# Remove Internal Metrics Collection

## Goal Description
Remove the internal `metrics` feature, the `Metrics` struct, and all associated metric collection code (atomics, counters) from the codebase. This simplifies the `RustFiber` engine, removing overhead and maintenance burden for an ad-hoc metrics system, likely in favor of external profiling tools or a more robust future solution.

## User Review Required
> [!WARNING]
> This is a breaking change for any consumers relying on the `metrics` feature or `Metrics` struct.
> Internal visibility into specific job counts and queue depths via this API will be lost.

## Proposed Changes
### Core System
#### [DELETE] [metrics.rs](file:///g:/Projects/Arcel/RustFiber/src/metrics.rs)
#### [MODIFY] [lib.rs](file:///g:/Projects/Arcel/RustFiber/src/lib.rs)
- Remove `mod metrics;` declaration.
- Remove re-exports if any.

#### [MODIFY] [worker.rs](file:///g:/Projects/Arcel/RustFiber/src/worker.rs)
- Remove `#[cfg(feature = "metrics")]` blocks updating counters.

#### [MODIFY] [job_system.rs](file:///g:/Projects/Arcel/RustFiber/src/job_system.rs)
- Remove `#[cfg(feature = "metrics")]` blocks.

#### [MODIFY] [Cargo.toml](file:///g:/Projects/Arcel/RustFiber/Cargo.toml)
- Remove `metrics` feature.

## Verification Plan
### Automated Tests
- Run `cargo test` to ensure no compilation errors.
- Run `cargo bench` to ensure benchmarks still compile and run (they likely don't depend on internal metrics feature being enabled by default).

### Manual Verification
- Verify build with `cargo build --all-features`.
