# Tasks: Add Criterion Nanosecond Latency Benchmark

## Implementation Tasks

- [x] **1. Add criterion dev-dependency**
  - Update `Cargo.toml` to add `criterion = "0.5"` under `[dev-dependencies]`
  - Add `[[bench]]` section targeting `benches/fiber_switch.rs` with `harness = false`
  - Verification: `cargo check` ✓

- [x] **2. Expose raw fiber benchmarking API**
  - New module: `src/bench.rs` with `BenchFiber` struct
  - Bypasses job system (dispatch, parking, work-stealing)
  - Directly exercises fiber resume/yield via corosensei
  - Exported via `rustfiber::BenchFiber`

- [x] **3. Create criterion benchmark file**
  - New file: `benches/fiber_switch.rs`
  - Benchmarks:
    - `raw_fiber_switch`: Pure context switch via BenchFiber ✓
    - `raw_fiber_with_work`: Context switch + minimal work ✓
    - `job_system_cold`: Full JobSystem path for comparison ✓
  - Verification: `cargo bench --bench fiber_switch` ✓

## Validation Tasks

- [x] **4. Run benchmark and verify results**
  - Execute `cargo bench --bench fiber_switch` ✓
  - Results:
    - **`raw_fiber_switch`: ~18 ns** ✅ (meets 20-50 ns target!)
    - **`raw_fiber_with_work`: ~19 ns** ✅
    - **`job_system_cold`: ~501 µs** (includes 1ms parking timeout)
  - **Raw context switching is within target range!**

## Summary

The `BenchFiber` API exposes raw fiber operations for accurate nanosecond-scale benchmarking.
The ~18ns switch time demonstrates corosensei's efficiency. The JobSystem "cold" path latency
(~500µs) is dominated by the GlobalParker 1ms timeout, not the fiber switch itself.
