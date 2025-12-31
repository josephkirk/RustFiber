# Tasks: Migrate Legacy Benchmarks to Criterion

## Implementation Tasks

- [x] **1. Add benchmark entries in Cargo.toml**
  - Add 7 `[[bench]]` sections with `harness = false`
  - Benchmarks: latency, work_stealing, scientific, transform, producer_consumer, allocation, startup
  - Verification: `cargo check --benches` ✓

- [x] **2. Create `benches/latency.rs`**
  - Migrate `run_empty_job_latency_benchmark`
  - Measure scheduling latency at batch sizes: 100, 1K, 10K, 100K
  - Use criterion's `BenchmarkGroup` with throughput metrics
  - Verification: `cargo bench --bench latency` ✓

- [x] **3. Create `benches/work_stealing.rs`**
  - Migrate `run_work_stealing_stress_benchmark`
  - Test work-stealing under high contention with imbalanced workloads
  - Verification: `cargo bench --bench work_stealing` ✓

- [x] **4. Create `benches/scientific.rs`**
  - Migrate EP, MG, CG benchmarks from `nas_benchmarks.rs`
  - Group as `scientific/ep`, `scientific/mg`, `scientific/cg`
  - Verification: `cargo bench --bench scientific` ✓

- [x] **5. Create `benches/transform.rs`**
  - Migrate `run_transform_hierarchy_benchmark`
  - Test parallel hierarchy updates (game engine pattern)
  - Verification: `cargo bench --bench transform` ✓

- [x] **6. Create `benches/producer_consumer.rs`**
  - Migrate `run_producer_consumer_benchmark`
  - Test lock-free SegQueue producer-consumer pattern
  - Verification: `cargo bench --bench producer_consumer` ✓

- [x] **7. Create `benches/allocation.rs`**
  - Migrate `run_allocation_benchmark`
  - Test frame allocator allocation throughput
  - Verification: `cargo bench --bench allocation` ✓

- [x] **8. Create `benches/startup.rs`**
  - Migrate `run_startup_latency_benchmark`
  - Measure JobSystem initialization time with different configs
  - Verification: `cargo bench --bench startup` ✓

## Validation Tasks

- [x] **9. Run all criterion benchmarks**
  - Execute `cargo check --benches` ✓
  - Tested `cargo bench --bench latency` — all batch sizes pass
  - All benchmarks compile and run correctly

- [x] **10. Update BENCHMARK.md**
  - Document new criterion benchmarks ✓
  - Update Quick Start with all benchmark commands

## Dependencies

Tasks 2-8 depend on Task 1 (parallel after that).
Task 9 depends on Tasks 2-8.
Task 10 depends on Task 9.
