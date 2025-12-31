# Tasks: Implement Thread Pinning Strategies

## Phase 1: Core Logic
- [x] Define `PinningStrategy` enum in `src/lib.rs` and export it.
- [x] Add `new_with_strategy` constructor to `WorkerPool` in `src/worker.rs`.
- [x] Implement core mapping logic based on strategy in `WorkerPool`.
- [x] Update `Worker::new` to accept an optional `CoreId` instead of `pin_to_core: bool`.

## Phase 2: Public API
- [x] Update `JobSystem` in `src/job_system.rs` to support `PinningStrategy`.
- [x] Add `JobSystem::new_with_strategy(num_threads, strategy)` method.
- [x] (Optional) Update `new_with_affinity` to use `PinningStrategy::Linear` internally.

## Phase 3: Verification
- [x] Create an integration test `tests/affinity_tests.rs` to verify thread pinning behavior (if possible to verify programmatically).
- [x] Add a benchmark check (or manual verification guide) to ensure workers are landing on the correct logical processors.
