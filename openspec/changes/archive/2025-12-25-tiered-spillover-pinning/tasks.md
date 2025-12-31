# Tasks: Implement Tiered Spillover System

## Phase 1: Tracking & Foundation
- [ ] Add `active_workers: Arc<AtomicUsize>` to `WorkerPool`.
- [ ] Update `Worker::run_loop` to increment/decrement `active_workers` during job execution.
- [ ] Add `TieredSpillover` to `PinningStrategy` enum in `src/lib.rs`.

## Phase 2: Core Mapping
- [ ] Update `WorkerPool::new_with_strategy` to categorize cores:
    - Tier 1: CCD0 Physical (first 8 even core IDs).
    - Tier 2: CCD1 Physical (next 8 even core IDs).
    - Tier 3: SMT (all remaining IDs).
- [ ] Assign `tier` and `threshold` properties to `Worker` during creation.

## Phase 3: Spillover Logic
- [ ] Update `Worker::run_loop` with threshold check:
    - High-tier workers wait (`thread::yield_now()` or `park_timeout`) if `active_workers` is below their tier's threshold.
    - Tier 2 Threshold: 7
    - Tier 3 Threshold: 15

## Phase 4: Verification
- [ ] Create `tests/tiered_spillover_tests.rs`.
- [ ] Update `src/benchmarks/main.rs` to support `tiered-spillover` argument.
- [ ] Run benchmarks and verify core usage patterns (e.g., using Task Manager or `perf`).
