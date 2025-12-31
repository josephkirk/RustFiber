# Optimization Finding: Global Submission Overhead

## Observation
During the verification of the **Frame Linear Allocator**, benchmarks revealed that utilizing the frame allocator for nested job creation was actually **slower** than standard heap allocation in the current implementation.

### Benchmark Results (50k jobs)
- **Heap Allocation (Box::new)**: ~4.7ms
- **Frame Allocation**: ~8.1ms

## Root Cause Analysis
The performance degradation is **not** due to the allocation strategy itself. Bump allocation (Frame Allocator) is mathematically faster than Heap allocation (`malloc`/`free`). 

The bottleneck lies in **Job Submission**.

### Current Flow
1. **Heap Case**: The main thread calls `job_system.run()`. This calls `worker_pool.submit()`, which pushes the job into the **Global Injector** queue.
2. **Frame Case**: A worker thread (executing the root job) calls `context.spawn_job()`. This currently calls `self.job_system.submit_to_injector()`, which *also* pushes the job into the **Global Injector** queue.

### The Problem
When a Worker spawns a job using `context.spawn_job()`:
1. It acquires the lock (or atomic contention) on the **Global Injector**.
2. This creates contention with other workers trying to steal or submit.
3. It fails to utilize the **Worker's Local Queue**, which is designed for this exact scenario.

A Worker's **Local Queue** (DeQueue) allows:
- **Lock-free pushes** by the worker owner (LIFO behavior).
- **Stealing** by other workers (FIFO behavior).

By forcing all nested jobs to go through the Global Injector, we incur significant synchronization overhead that outweighs the benefits of cheaper memory allocation.

## Optimization Applied: Local Queue Submission
We modified `Context` to hold a reference to the executing Worker's `local_queue` and updated `spawn_job` to push to it directly.

### Results
- **Before Optimization (Frame)**: ~8.1ms
- **After Optimization (Frame)**: ~6.8ms
- **Speedup**: ~16% improvement.

### Remaining Gap
Heap allocation path (~4.7ms) is still faster in this synthetic benchmark. This likely indicates that:
1. **Counter Allocation**: Both paths allocate `Counter` (Arc) on heap. This fixed cost might dominate.
2. **Stealing Contention**: In the Frame test, 1 worker pushes 50k jobs locally. 3 other workers must *steal* them one by one. Stealing is more expensive than popping from Global Injector (where all workers are equal consumers).
   - *Verification*: Running with 1 thread showed Frame path even slower relative to Heap, suggesting per-job overhead (Context creation, etc.) is higher than `Box::new`.

## Conclusion
The **Local Queue Optimization** successfully reduced overhead, but for bulk fan-out scenarios (1 parent -> 50k children), the Global Injector strategy (distributing work immediately) might ironically be more efficient for load balancing than filling one local queue and forcing others to steal.

However, for deep, recursive, fine-grained parallelism (Divide & Conquer), Local Queue submission is architecturally superior for cache locality (LIFO) and avoiding global bottlenecks.
