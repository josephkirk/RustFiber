# Architecture Guide: High-Performance Fiber Job System

## 1. Executive Summary

This document details the architecture of `RustFiber`, a high-performance job system heavily inspired by **Naughty Dog's GDC 2015 presentation** ("Parallelizing the Naughty Dog Engine Using Fibers"). The system solves the problem of efficiently scheduling thousands of small, granular tasks across a fixed number of CPU cores without the overhead of OS-level context switching.

By decoupling execution contexts ("fibers") from OS threads, the system allows for:
- **Zero OS Interference**: Threads are pinned to cores and never block.
- **Lock-Free Synchronization**: Dependencies are resolved via atomic counters.
- **High Throughput**: Capable of processing millions of jobs per second.
- **Low Latency**: User-space context switches take nanoseconds, not microseconds.

---

## 2. Core Concepts

### 2.1 The Fiber
A **fiber** is a lightweight, user-space thread of execution with its own stack. Unlike OS threads, fibers are cooperatively scheduled.
- **Stackful**: Fibers preserve their call stack when yielding. This allows deep call graphs to pause and resume (e.g., inside a nested traversal).
- **Pooled**: Stacks are expensive to allocate (mmap). Fibers are allocated from a pool and recycled.
- **Switching**: We use the `corosensei` crate to handle the assembly-level context switching.

### 2.2 The Job
A **job** is the smallest unit of work, defined as a `FnOnce`.
- **Granularity**: Jobs should be small but not trivial.
- **Lifetime**: Jobs are transient. They run, complete, and modify the dependency graph.

### 2.3 The Atomic Counter
The universal synchronization primitive.
- **Dependency Tracking**: Jobs wait on counters, not other jobs. This creates a flexible DAG.
- **Wait Lists**: When a fiber waits on a counter, it adds itself to the counter's intrusive wait list and yields. When the counter reaches zero, the waiting list is flushed, and the fibers are rescheduled.

---

## 3. System Architecture

### 3.1 Worker Threads & Pinning
The system spawns `N` worker threads (where `N` = logical cores).
- **Pinning**: Threads are pinned to specific cores to maximize L1/L2 cache locality.
- **Strategies**:
    - `Linear`: 1:1 mapping (Best for uniform workloads).
    - `TieredSpillover`: Dynamic expansion. Uses physical cores first, spills to SMT/Hyperthreads only under load.
    - `AvoidSMT`: Strictly physical cores.

### 3.2 Scheduling: Work Stealing
We use a **Chase-Lev Work-Stealing Deque** (via `crossbeam-deque`).
- **Local Queue (LIFO)**: Workers push/pop to their own deque. LIFO order ensures hot cache locality (most recently added job is executed first).
- **Stealing (FIFO)**: If a worker runs out of work, it steals from the *top* (FIFO) of other workers' queues. This minimizes conflict with the owner (who operates on the bottom).
- **Global Injector**: Handles entry-point jobs from external threads. Accessed only after local work and stealing attempts fail.

---

## 4. Implementation Details & Optimizations (v0.2)

### 4.1 Stack Reuse & Memory Efficiency
Allocating new stacks for every fiber is prohibitive (`mmap` syscalls).
- **Solution**: We implemented `DefaultStack` reuse in `FiberPool`.
- **Mechanism**: When a fiber completes, its stack is reset (rewind stack pointer) rather than deallocated. This reduced fiber allocation cost to near-zero.
- **Machine Cost**: Pre-allocating fibers incurs a fixed startup latency (~4-5ms for 16 threads) and virtual memory reservation (~1GB). **UPDATE**: Optimized to <1ms startup through incremental pool growth.
- **Benefit**: This "Warmup Cost" guarantees no allocation glitches during runtime execution, which is critical for consistent game frame times.
- **Reference**: See [Startup Analysis](docs/startup_analysis.md) for details.

### 4.2 Intelligent Backoff & SMT Mitigation
Efficiently handling idle states is crucial for both power and performance (especially on SMT/Hyperthreading).
- **Tiered Backoff**: Workers use a 3-stage backoff strategy when no work is found:
    1.  **Spinning**: Brief tight loop to catch immediately arriving work (lowest latency).
    2.  **Yielding**: Calls `std::thread::yield_now()` to play nice with OS scheduler.
    3.  **Deep Idle (Sleep)**: If still idle, the worker sleeps (`100µs`).
- **SMT Contention Fix**: The "Deep Idle" sleep is critical for SMT processors. By forcing idle threads to sleep, we release execution resources (ALUs, L1 cache) to the sibling thread running on the same physical core, preventing "lazy" threads from slowing down "busy" ones.
- **Result**: Eliminates performance spikes at high core counts and improving scaling stability.

### 4.3 Strategy-Aware Scheduling (Hybrid Wakeup)
Different pinning strategies require different scheduling logic.
- **Problem**: A naive "Local Wakeup" (pushing woke fibers to local queue) works great for `Linear` (affinity) but breaks `TieredSpillover` (dormant threads never see the work).
- **Solution**: The `Worker` now dynamically selects the scheduler based on the active `PinningStrategy`.
    - **Linear / AvoidSMT**: Uses **Local Queue** for rescheduled fibers. Preserves CPU affinity.
    - **TieredSpillover**: Uses **Global Injector** for rescheduled fibers. Ensures dormant threads (Tier 2/3) can pick up spillover work.

### 4.4 Configurable Stack & Job Priorities
Different workloads require different resource guarantees.
- **Fiber Configuration**: `FiberConfig` allows tuning of stack sizes (default 512KB) and initial pool sizes to match application memory constraints.
- **Job Priorities**:
    - **High**: Critical path tasks (physics, audio). Checked first by workers.
    - **Normal**: Standard logic.
    - **Low**: Background tasks.
    - **Scheduling**: High-priority jobs utilize a dedicated global injector and are prioritized during work stealing and yielding. In `TieredSpillover`, yielded High-priority fibers are immediately pushed to the high-priority global injector to wake up available workers.

### 4.5 Frame Allocator (Bump Allocation)
Memory allocation is a significant bottleneck for fine-grained tasks.
- **Per-Fiber Allocator**: Each fiber context owns a `Linear` (bump) allocator.
- **Zero-Cost**: Allocating a job closure or a `Counter` inside `ctx.spawn_job` is essentially a pointer increment.
- **Reset**: The allocator is reset automatically when the fiber is recycled or when a new frame begins, making deallocation free.

### 4.6 Zero-Overhead Job Submission
We introduced specialized spawn methods to eliminate remaining overheads:
- **`spawn_detached`**: "Fire-and-forget" jobs. No `Counter` is allocated. Perfect for "million-particle" simulations where individual completion tracking is unnecessary (track the group instead).
- **`spawn_with_counter`**: Allows thousands of jobs to share a single `Counter` (via `Arc`), reducing atomic hardware synchronization traffic.
- **Local Queue Submission**: Jobs spawned from within a fiber are pushed directly to the worker's **Lock-Free Local Queue** (LIFO), bypassing the Global Injector lock entirely.

### 4.7 Cache Alignment & False Sharing Mitigation
To ensure scalability on high-core-count systems (e.g., Threadripper, EPYC), we aggressively manage data layout to prevent **false sharing**.
- **`InnerCounter`**: Aligned to **128 bytes** (`#[repr(align(128))]`). This ensures that `Counter` objects, even when allocated consecutively in memory (common in loops), reside on distinct cache lines. The 128-byte alignment also effectively pads the surrounding `Arc` control block.
- **`WaitNode`**: Aligned to **64 bytes** (`#[repr(C, align(64))]`). Since these nodes are often stack-allocated or pooled, alignment prevents adjacent nodes from sharing cache lines during high-contention wakeups.
- **Global State**: Hot global atomics in the `Worker` (`active_workers`, `shutdown`, `frame_index`) are padded using `crossbeam::utils::CachePadded`. This prevents Read-Write false sharing where a worker updating `active_workers` could invalidate the cache line containing `shutdown` or `frame_index` for all other workers.

### 4.8 Job Batching & Range Splitting
For effectively parallelizing loops with millions of tiny iterations (e.g., entity updates, particle simulations), creating a job per iteration is too expensive due to scheduler overhead.
- **`parallel_for_chunked`**: Automatically splits a `Range<usize>` into large chunks (batches) relative to the number of worker threads.
- **Optimization**: Instead of processing simple indices, the closure receives a `Range<usize>` (e.g., `0..10_000`). This allows the user to:
    1.  Perform **Batch-Local Accumulation**: Maintain loop-local variables (sums, states) and only sync to global atomic state once per batch, eliminating cache line contention.
    2.  Apply **SIMD**: Process contiguous data chunks efficiently using vector instructions.
    3.  Reduce Scheduling Overhead: One job managing 10k items reduces scheduler pressure by 10,000x.

### 4.9 NUMA Awareness Framework
To optimize memory locality on multi-socket systems, we implemented infrastructure for NUMA-local fiber stack allocation.
- **First-Touch Policy**: Memory pages are allocated and zeroed on the worker thread that will use them, ensuring placement on the correct NUMA node.
- **Configuration**: `FiberConfig` includes a `prefetch_pages` option to control this behavior.
- **Current Status**: Framework implemented but disabled on Windows due to guard page violations that prevent safe memory writes to stack regions. The infrastructure remains ready for future Windows-compatible implementations.

---

## 5. Performance

Our benchmarks demonstrate the effectiveness of this architecture:

- **Fibonacci (Tiny Tasks)**: >3.5 Million tasks/sec.
- **Conjugate Gradient (Memory Bound)**: <1.5ms execution time for 100k elements (Linear Strategy).
- **QuickSort (Recursive)**: Linear scaling with core count.
- **Startup Latency**: Reduced from 4-5ms to <1ms through incremental fiber allocation.

See [BENCHMARKS.md](BENCHMARKS.md) for detailed graphs and results.

## 6. Current Limitations

- **Windows NUMA Prefaulting**: Disabled due to guard page violations that prevent direct memory writes to fiber stack regions. The framework is in place for future resolution of Windows compatibility issues.
- **Large Page Support**: Deferred to v0.3. Currently uses standard 4KB pages for fiber stacks, which may increase TLB pressure on large deployments.


### General Improvements Plan
- **Error Handling**: Consider adding more granular error types instead of relying on `Result<(), Box<dyn std::error::Error>>` in some places for better debugging.
- **Documentation**: Ensure all public APIs have comprehensive documentation with examples, especially for complex types like `PinningStrategy`.
- **Testing**: Add more integration tests for edge cases, such as system shutdown during job execution or memory allocation failures.
- **Performance Monitoring**: Consider adding optional metrics collection (e.g., job throughput, queue depths) for debugging performance issues.

### Specific Code Areas
- **Fiber Pool Management**: The incremental growth logic in `fiber_pool.rs` could benefit from more detailed comments explaining the hysteresis and growth thresholds.
- **Work Stealing**: Verify that the LIFO/FIFO deque implementation is correctly optimized for the target architectures, potentially adding SIMD optimizations if beneficial.
- **Allocator Safety**: Review all unsafe blocks in `allocator/` for potential race conditions, especially in multi-threaded scenarios.
