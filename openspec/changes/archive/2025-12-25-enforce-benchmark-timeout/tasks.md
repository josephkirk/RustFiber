# Tasks: Enforce Benchmark Timeout and Incremental Graph Generation

## Phase 1: Rust Benchmark Enhancements
- [x] Add timeout logic to `src/benchmarks/utils.rs`
- [x] Modify `src/benchmarks/main.rs` to output JSON per benchmark result
- [x] Implement timeout checks in:
    - [x] `src/benchmarks/fibonacci.rs`
    - [x] `src/benchmarks/quicksort.rs`
    - [x] `src/benchmarks/producer_consumer.rs`
    - [x] `src/benchmarks/nas_benchmarks.rs`

## Phase 2: Python Script Refactoring
- [x] Update `run_benchmarks.py` to read stdout line-by-line
- [x] Parse individual benchmark results and generate graphs immediately

## Verification
- [x] Run `python3 run_benchmarks.py` and verify:
    - [x] Graphs appear in `docs/` as soon as each test finishes
    - [x] No test runs for significantly longer than 1 minute
