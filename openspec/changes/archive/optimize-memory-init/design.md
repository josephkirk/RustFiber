# Design: Memory Initialization & NUMA Locality

## 1. NUMA-Local Zeroing (Pre-faulting)
**Problem**: `Fiber::new()` uses `corosensei::DefaultStack`, which performs a virtual memory reservation (`mmap` / `VirtualAlloc`). The physical pages are only allocated by the OS upon "First Access" (write). If this access happens randomly or on the wrong thread, performance suffers.
**Solution**:
- In `Fiber::new()`, immediately after creating the stack, write a zero to every 4KB page (or every 2MB page) in the stack.
- **Critical**: This code MUST run on the specific `Worker` thread that will own the fiber. (We already ensure this by creating `FiberPool` inside `Worker::run_loop`).

## 2. Interleaved Pre-warming
**Problem**: The loop `for _ in 0..128 { pool.push(Fiber::new(...)) }` blocks the thread for 5ms.
**Solution**:
- Add `FiberConfig::warmup_batch_size`.
- In `FiberPool::new()`, only allocate `warmup_batch_size` (e.g., 16).
- Add `FiberPool::grow(count)` method.
- **Gradual Loading**: The `JobSystem` or `Worker` can optionally call `grow()` in idle time, or rely on `FiberPool::get()` to lazily allocate as needed.
- **Default**: We can keep standard greedy allocation for games, or enable lazy for benchmarks.

## 3. Large Pages (HugeTLB)
**Problem**: 512KB stacks use 128 4KB pages. This thrashes the TLB.
**Solution**:
- **Windows**: Use `VirtualAlloc` with `MEM_LARGE_PAGES`. Requires `SeLockMemoryPrivilege`.
- **Linux**: Use `mmap` with `MAP_HUGETLB`.
- **Implementation**: We may need to implement a custom `Stack` trait for `corosensei` if `DefaultStack` doesn't support this.
    - *Constraint*: `corosensei` 0.1 might not expose easy custom stack allocator hooks. If it's too complex, we might defer Large Pages to v0.3.

## Implementation Details
Modify `src/fiber.rs`:
- Function `Fiber::new_prefault(stack_size)`.
- Loop through `stack.base()..stack.limit()` writing to each page.
