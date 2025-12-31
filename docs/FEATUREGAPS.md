# Feature Gap Analysis: RustFiber Implementation vs Specification

This document details the discrepancies found between the `RustJobSystem.md` specification, the `GAP.md` assessment, and the actual codebase in `src/`.

## 🚨 Critical Discrepancies (GAP.md vs Reality)

### 1. Fiber Execution & Context Switching
*   **Specification**: Requires userspace context switching (stackful coroutines) via `corosensei` or `context`.
*   **GAP.md Claim**: "Implemented" (via `context` crate).
*   **Actual Code**: **STUBBED / NOT IMPLEMENTED**.
    *   `src/context.rs` explicitly states: `Temporary fallback: use thread yielding`.
    *   `src/fiber.rs` is a wrapper around a `Job` closure without any stack management.
    *   The system currently operates as a Thread Pool with `std::thread::yield_now()`, effectively degenerating into OS-scheduled threads rather than a Fiber system.

### 2. Work Stealing Queues
*   **Specification**: Requires Chase-Lev Deque.
*   **GAP.md Claim**: "Missing" (claims usage of centralized/simple queue).
*   **Actual Code**: **IMPLEMENTED**.
    *   `src/worker.rs` correctly utilizes `crossbeam::deque` (Injector, Worker, Stealer), which IS a Chase-Lev implementation.
    *   Tiered stealing logic (`PinningStrategy::TieredSpillover`) is also present in the code.
    *   *Correction*: The gap analysis was incorrect; the data structures involved are correct.

### 3. Waiting Mechanism (Wait Lists)
*   **Specification**: Intrusive Linked List in Atomic Counter.
*   **GAP.md Claim**: "Partial" (uses locking/Condvar).
*   **Actual Code**: **NON-EXISTENT / POLLING**.
    *   `src/job_system.rs` implements `wait_for_counter` via a `thread::sleep` loop with exponential backoff.
    *   There is no "list" of waiting fibers. Threads simply block execution (sleep) until the counter resolves.
    *   *Impact*: This is significantly less performant than even the "Locking" fallback described in GAP.md, as it introduces scheduling latency and does not free the worker to execute other ready jobs (except via OS preemption).

---

## ✅ Verified Gaps (Correctly Identified)

### 4. Memory Management
*   **Status**: **MISSING**
*   `src/job.rs` uses `Box::new` for every job creation. There is no Frame Linear Allocator or Bump allocation.

### 5. Panic Safety
*   **Status**: **MISSING**
*   No `catch_unwind` in `src/worker.rs`. A panic in a job will crash the worker thread.

### 6. Cache Alignment
*   **Status**: **MISSING**
*   `src/counter.rs` uses `Arc<AtomicUsize>` without `CachePadded` or `#[repr(align(...))]`.

---

## Summary of Corrective Actions Needed

1.  **Implement Real Fibers**: Replace the `Job` wrapper in `fiber.rs` with `corosensei::Coroutine`.
2.  **Implement Wait Lists**: Remove `thread::sleep` in `job_system.rs`. When waiting, push the current fiber to a list in the Counter and switch to the next fiber in the worker loop.
3.  **Optimize Memory**: Replace `Box<dyn FnOnce>` with a custom allocator functionality or intrusive job structs.
