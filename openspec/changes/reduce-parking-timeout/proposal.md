# Change: Reduce Parking Timeout

## Why
The current job system exhibits ~500µs latency in the cold path, dominated by a 1ms coarse-grained sleep in the worker idle loop. This limits responsiveness for latency-critical workloads and low-load scenarios.

## What Changes
- Replace `std::sync::Condvar` with a signal-based parking mechanism (e.g., `std::thread::park` or a dedicated semaphore) to eliminate the 1ms sleep loop.
- Implement an adaptive timeout strategy where workers spin/yield for a short period before parking, and parking timeout adjusts based on recent activity.
- Add an "Immediate Wake" path for high-priority jobs that bypasses the normal parking logic to wake a worker instantly.
- Optimize the `GlobalParker` to reduce contention and ensure valid wakeups (<10µs cold path).

## Impact
- **Affected Specs**: `job-system` (Efficiency, Latency)
- **Affected Code**: `src/worker.rs` (`GlobalParker`, `Worker::run_loop`), `src/job_system.rs`
- **Performance**: Target <10µs cold path latency.
