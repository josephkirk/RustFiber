# Cache Alignment Design

## Problem
In high-concurrency scenarios, shared mutable state (like atomic counters and wait queues) can suffer from "false sharing". This occurs when multiple independent variables reside in the same CPU cache line. If one core modifies a variable, the entire cache line is invalidated for other cores, forcing them to reload data even if they are accessing a *different* variable that happens to be on the same line.

## Solution
We will enforce memory alignment and padding for critical structures.

### InnerCounter
`InnerCounter` is the hot path for job completion tracking.
```rust
struct InnerCounter {
    value: AtomicUsize,
    wait_list: AtomicPtr<WaitNode>,
}
```
Currently, this is ~16 bytes. Multiple `Arc<InnerCounter>` allocations could sit next to each other.
We will align `InnerCounter` to `128` bytes (safe upper bound for most cache lines, usually 64 bytes is sufficient but 128 covers prefetchers and some archs like Apple M-series or specialized server CPUs).
Using `#[repr(align(128))]` ensures that when allocated (even in `Arc`), the data starts at a 128-byte boundary, effectively enforcing that it occupies its own cache block(s) and doesn't share with neighbors.

### WaitNode
`WaitNode` is used in the intrusive linked list of waiters.
```rust
#[repr(C)]
pub struct WaitNode {
    pub next: AtomicPtr<WaitNode>,
    pub fiber_handle: AtomicPtr<Fiber>,
    pub state: AtomicU32,
}
```
Nodes are often stack-allocated (in `Context` or `Fiber` stack) or part of a pool. Aligning them reduces the chance that an update to one node invalidates a neighbor.
We will use `#[repr(align(64))]` or `CachePadded`. Since `WaitNode` is often moved or pointed to, strict alignment is beneficial.

## Dependencies
We will use `crossbeam-utils` for `CachePadded` where convenient, or standard `#[repr(align(N))]` where structural alignment is preferred.
For `InnerCounter`, `#[repr(align(128))]` is preferred to padded fields because we want the *whole object* separated from others.
