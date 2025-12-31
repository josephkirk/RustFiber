# Design: Topology-Aware Allocator Integration

## Current State
- `Job` struct creation (`Job::new`) uses `Box::new` which invokes the Global Allocator (`malloc`/`heap`).
- `FrameAllocator` exists and is used via `Job::new_in_allocator`, but requires explicit passing of `&mut FrameAllocator`.
- `Context` holds the allocator, but it's only available during job execution, not necessarily during creation (unless creating nested jobs).

## Problem
- Heavy use of Global Allocator for short-lived jobs causes fragmentation and contention.
- Passing `&mut FrameAllocator` everywhere is unergonomic.
- We want strictly local allocations for better cache locality (L1/L2) and to avoid lock contention on the global heap.

## Proposed Solution
- **Shift to `Job::new` defaulting to Local if possible?**
  - Hard because `Job::new` is static.
  - We can rely on **Thread Local Storage (TLS)** to expose the current worker's `FrameAllocator` (scratchpad).
  - OR we change the API to always require a context/scope for job creation.

## Architectural Decision: Context-Bound Creation vs TLS
Option A: **TLS (Thread Local Storage)**
- `Worker` sets a TLS pointer to its `FrameAllocator` before executing the loop.
- `Job::new(closure)` checks TLS. If set, allocates in the frame. If not, falls back to Global (or panics if strict).
- Pros: Seamless API change. `Job::new` "just works" faster.
- Cons: Hidden state. Magic behavior. Safety with lifetimes (Frame allocation is temporary, must not outlive the frame reset).

Option B: **Scanner / Scope API**
- `scope(|s| { s.spawn(|| ... ) })`
- `s` holds the allocator reference.
- Pros: Explicit lifetimes.
- Cons: Refactor of all call sites.

## Decision: Hybrid / TLS Verification
Given the goal "Integrate ... more deeply ... to avoid Global allocator entirely", and "strictly local":
- We should likely aim for **TLS-based default** for `Job::new` when running on a worker, but with VERY strict lifetime checks or checks that we are effectively in a "Job Scope".
- However, `FrameAllocator` resets every frame. Allocating a Job in it means the Job must run and finish (or be dropped) before the frame resets.
- Current architecture processes jobs in batches/frames.
- If we use TLS, `Job::new` returns a `Job` allocated in the bump ptr.
- **Challenge**: `Job` struct itself is stack allocated, but it holds `Work` enum which wraps the closure. `Work::Simple` holds `Box<dyn Fn...>`. `Work::SimpleFrame` holds raw pointer `*mut u8`.
- Implementation: Provide `Job::spawn` or similar that grabs the TLS allocator.

## Per-Worker Arena Details
- Each `Worker` struct owns a `FrameAllocator`.
- During `run_loop` or `steal_loop`, the worker makes this allocator active.
- New `Job` creation uses this active allocator.

## Safety Considerations
- **Escape Analysis**: If a `Job` allocated in Frame A is somehow stored and executed in Frame B (after reset), we have UB.
- Fiber integration: Fibers can yield. If a job is part of a fiber that yields across frames, its stack (and closures?) must survive.
- *Correction*: The `Job` *struct* usually describes a *new* piece of work. Once executed, it's done.
- If a Job spawns another Job, the child Job is allocated in the *current* frame. This is safe as long as the child Job is executed within the same frame or strictly managed.
- **Strict Logic**: The Frame Allocator is generally for *temporary* data. Jobs are temporary *if* they are executed promptly.

## Plan
1. Add `scoped_allocator` TLS variable in `worker.rs` (or `job.rs` if better suited).
2. Update `Worker::run` logic to set this TLS.
3. Modify `Job::new` (or add `Job::spawn`) to try TLS allocator first.
4. Implement `SimpleFrame` and `WithContextFrame` variants reuse for this flow.

