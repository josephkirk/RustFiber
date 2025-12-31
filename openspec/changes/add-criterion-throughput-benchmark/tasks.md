# Tasks: Add Criterion Throughput Benchmark

## Implementation Tasks

- [x] **1. Add benchmark entry in Cargo.toml**
  - Add `[[bench]]` section for `throughput` with `harness = false`
  - Added `num_cpus` dev-dependency for thread detection
  - Verification: `cargo check` ✓

- [x] **2. Create throughput benchmark file**
  - New file: `benches/throughput.rs`
  - Benchmarks:
    - `spawn_1m_jobs` — spawns 1,000,000 jobs with shared Counter
    - `spawn_1m` scaling — tests at 1, 2, 4, 8 threads
  - Uses `run_with_context()` + `spawn_with_counter()` pattern
  - Uses `wait_for_counter()` to trigger helping protocol
  - Verification: `cargo bench --bench throughput` ✓

## Validation Tasks

- [x] **3. Run benchmark and verify results**
  - Execute `cargo bench --bench throughput` ✓
  - Results (32 threads, 1M jobs):
    - **Time: ~72 ms** per batch
    - **Throughput: ~13.8 M jobs/second**
  - Scaling analysis shows similar throughput at different thread counts

## Dependencies

Task 2 depends on Task 1.
