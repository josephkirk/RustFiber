# Proposal: Implement Job Batching (`parallel_for`)

## Why
High-frequency job submission (e.g., millions of tiny tasks) incurs per-job overhead (allocation, queuing, synchronization) that can bottleneck performance on high-core-count systems. As core counts increase (e.g., 16 vs 8), the contention and overhead can offset the benefits of parallelism for fine-grained workloads.

"Automatic batching" allows the system to group many small operations into fewer, larger jobs. This amortizes the overhead of job creation and scheduling across multiple iterations of the user's logic, significantly improving throughput for "embarrassingly parallel" loops.

## What Changes
We will introduce `parallel_for` APIs to `JobSystem` and `Context`. These methods will:
1.  Accept a range (e.g., `0..1_000_000`), a batch size, and a closure.
2.  Divide the range into chunks based on the batch size.
3.  Spawn **one job per chunk** (instead of one per item).
4.  Execute the user's closure for each index in the chunk within that single job.

### Scenarios
- **Massive Particle Update**: Update 1M particles. Instead of 1M jobs, spawn 1000 jobs of 1000 particles each.
- **Image Processing**: Process pixels. Chunk by scanlines or blocks.

## Verification Plan
1.  **New Benchmark**: Create a benchmark comparing `parallel_for` vs. naive spawning for 1M tiny tasks.
2.  **Correctness**: Verify that all indices in the range are visited exactly once.
3.  **Performance target**: Demonstrate >2x speedup on the existing "Tiny Tasks" benchmark logic when using batching on >8 cores.

## Risks
- **Load Balancing**: If chunks are too large, we might lose the benefits of work-stealing (one thread stuck with a huge chunk while others idle).
- **Latency**: Batching increases the latency of the *first* result (must wait for batch to start) but improves *throughput*.
