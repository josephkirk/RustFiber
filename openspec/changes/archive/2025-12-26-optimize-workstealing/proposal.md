# Optimize Work Stealing

## Problem
The current scheduler implementation lacks a formal specification for its work-stealing behavior, leading to potential inefficiencies and difficulties in verifying performance characteristics like cache locality (LIFO for local) and load balancing (FIFO for stealing). While the code implements a Deque-based worker, specific behaviors like "safe spinning" vs "parking" and "batch stealing" need to be codified to ensure consistent throughput and latency.

## Solution
Formalize the work-stealing scheduler requirements into a new `work_stealing` capability. This spec will define:
1.  **Dual-Ended Queue Access**: Strictly enforce LIFO for local pop (owner) and FIFO for stealing (thieves).
2.  **Injector Queue**: Define interaction rules for the global injector (frequency, batching).
3.  **Backoff Strategy**: Specify the specialized "Brief Idle" (spin) and "Deep Idle" (yield/park) states to balance latency vs. CPU usage.
4.  Integration with the existing `fiber_execution` model.

## Validation
- **Benchmarks**: Use `QuickSort` (recursive) and `MatrixMult` (parallel) to verify throughput remains high and regression is avoided.
- **Unit Tests**: Verify queue ordering (LIFO/FIFO) via synthetic tests.
