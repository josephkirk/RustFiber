## Context
The current `GlobalParker` uses `Condvar::wait_timeout` with 1ms. This works for throughput but hurts latency. Workers sleep for 1ms even if work arrives 10µs later if the wakeup signal isn't propagated effectively or if we rely solely on timeout for discovery in some paths.
The goal is to rely on *signals* (wakeups) rather than *polling* (timeouts).

## Goals / Non-Goals
- **Goals**:
  - Cold path latency < 10µs.
  - No regression in throughput for heavy loads.
  - Correctness (no deadlocks, no lost wakeups).
- **Non-Goals**:
  - Major architectural rewrite of the scheduler (Steal-Half is preserved).

## Decisions
- **Decision 1**: Use `std::thread::park` / `Thread::unpark` (or `crossbeam::sync::Parker`) per worker instead of a single Global `Condvar`.
    - *Rationale*: A single global mutex/condvar causes contention when all workers try to sleep or wake. Per-thread parking allows targeted wakeups (e.g., wake 1 specific worker).
    - *Alternative*: Keep global `Condvar` but remove timeout. Requires all workers to contend on the lock to wait.
- **Decision 2**: Adaptive Spinning.
    - *Rationale*: Spinning is cheap for very short idle periods (<< 1 syscall). Parking is expensive (~5-10µs overhead). We should spin briefly before parking.
    - *Mechanism*: Use `crossbeam::AppBackoff` or similar with a capped spin count before calling `park()`.
- **Decision 3**: Atomic State for "Has Work".
    - *Rationale*: To avoid "lost wakeup" where a worker checks queue, sees empty, decides to park, but work is added *before* it parks.
    - *Mechanism*: `parking_lot` style "sequenced lock" or `AtomicUsize` state (IDLE, SEARCHING, RUNNING).
    - *Wait Logic*: Set state to SLEEPING. Check queues one last time. If empty, park.
    - *Wake Logic*: If pushing work and target is SLEEPING, unpark it.

## Risks / Trade-offs
- **Risk**: "Thundering Herd" if we wake all.
    - *Mitigation*: targeted `wake_one`.
- **Risk**: Higher CPU usage if spin limit is too high.
    - *Mitigation*: Tunable constant, default conservative.

## Migration Plan
- Replace `GlobalParker` struct.
- Update `Worker` to hold its own `Parker` (or `Thread` handle).
- Update `WorkerPool` to manage handles/parkers.
