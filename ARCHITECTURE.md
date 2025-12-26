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
- **Local Queue**: Workers push/pop to their own LIFO queue (Hot cache optimization).
- **Stealing**: If a worker runs out of work, it attempts to steal from the *top* (FIFO) of other workers' queues.
- **Global Injector**: Handles entry-point jobs from external threads.

---

## 4. Implementation Details & Optimizations (v0.2)

### 4.1 Stack Reuse & Memory Efficiency
Allocating new stacks for every fiber is prohibitive (`mmap` syscalls).
- **Solution**: We implemented `DefaultStack` reuse in `FiberPool`.
- **Mechanism**: When a fiber completes, its stack is reset (rewind stack pointer) rather than deallocated. This reduced fiber allocation cost to near-zero.
- **Impact**: Removing allocation overhead was critical to achieving >6M jobs/sec.

### 4.2 Adaptive Spinning
Context switching, even in user space, has overhead (~50ns + cache pollution).
- **Problem**: Yielding immediately for a very short dependency (nanoseconds) is inefficient.
- **Solution**: `wait_for_counter` implements **Adaptive Spinning**. It spins for a calibrated number of cycles (`SPIN_LIMIT = 2000`) before yielding.
- **Result**: Drastically reduced latency for fine-grained dependency chains.

### 4.3 Strategy-Aware Scheduling (Hybrid Wakeup)
Different pinning strategies require different scheduling logic.
- **Problem**: A naive "Local Wakeup" (pushing woke fibers to local queue) works great for `Linear` (affinity) but breaks `TieredSpillover` (dormant threads never see the work).
- **Solution**: The `Worker` now dynamically selects the scheduler based on the active `PinningStrategy`.
    - **Linear / AvoidSMT**: Uses **Local Queue** for rescheduled fibers. Preserves CPU affinity.
    - **TieredSpillover**: Uses **Global Injector** for rescheduled fibers. Ensures dormant threads (Tier 2/3) can pick up spillover work.

---

## 5. Performance

Our benchmarks demonstrate the effectiveness of this architecture:

- **Fibonacci (Tiny Tasks)**: >3.5 Million tasks/sec.
- **Conjugate Gradient (Memory Bound)**: <1.5ms execution time for 100k elements (Linear Strategy).
- **QuickSort (Recursive)**: Linear scaling with core count.

See [BENCHMARKS.md](BENCHMARKS.md) for detailed graphs and results.