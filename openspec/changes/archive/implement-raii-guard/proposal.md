# Proposal: Implement RAII Counter Guard and Panic Safety

## Summary
Implement `CounterGuard` to ensure `AtomicCounter` is decremented even if a job panics, and enforce `catch_unwind` boundaries in job execution to prevent worker thread crashes and permanent deadlocks.

## Motivation
Current `AtomicCounter` decrement is manual. If a job panics before decrementing, the counter never reaches zero, causing dependent jobs to wait forever (deadlock). Uncaught panics also crash worker threads.

## Detailed Description
- Introduce `CounterGuard` struct wrapping `AtomicCounter` and `JobSystem`.
- Implement `Drop` for `CounterGuard` to call `counter.signal()`.
- Wrap job execution in `std::panic::catch_unwind`.
- Ensure fiber integrity after panic.
