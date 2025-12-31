# Implementation Tasks

1.  [x] Implement `CounterGuard` in `src/counter.rs`
    - [x] Add struct `CounterGuard`
    - [x] Implement `Drop`
    - [x] Update `Job` to use/provide `CounterGuard`
2.  [x] Add `catch_unwind` to Job Execution
    - [x] Update `Fiber::new` or `Job::execute` with `catch_unwind`
    - [x] Handle `Err` from `catch_unwind` by logging and ensuring Clean-up
3.  [x] Verify Panic Safety
    - [x] Add test case with panicking job
    - [x] Verify counter handles decrement
    - [x] Verify worker survives
