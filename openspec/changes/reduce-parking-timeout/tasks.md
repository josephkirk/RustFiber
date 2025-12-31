## 1. Implementation
- [x] 1.1 **Refactor GlobalParker**: Replace `Condvar` with `std::thread::park` or `Parker` for signal-based waiting.
- [x] 1.2 **Implement Adaptive Strategy**: Add logic to `Worker::run_loop` to track recent activity and adjust spin/park behavior.
- [x] 1.3 **Add Immediate Wake Path**: Update `JobSystem::run_priority` and `Worker::submit` to explicitly wake a specific target or broadcast if needed.
- [x] 1.4 **Optimize Idle Loop**: Tune spin loop counts (currently `Backoff`) and integrate with the parking mechanism (prevent lost wakeups).
- [x] 1.5 **Verification**: Run `job_system_cold` benchmark to verify <10µs latency.
- [x] 1.6 **Regression Test**: Ensure `scientific` benchmarks (throughput) do not regress due to increased wake-up overhead.

## 2. Refinements (Post-Implementation)
- [x] 2.1 **Fix Bitset Bias**: Implemented randomized round-robin scanning for `wake_one` to fix work-stealing regression.
- [x] 2.2 **Fix 500µs Floor**: Optimized `wait_for_counter` with hybrid spin-wait to eliminate OS timer granularity artifacts in benchmarks.
