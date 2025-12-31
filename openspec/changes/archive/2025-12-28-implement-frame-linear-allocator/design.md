# Design: Frame Linear Allocators

## Architectural Changes

### 1. FrameAllocator
A bump allocator that manages a large contiguous block of memory (e.g., `Vec<u8>` or `mmap`).
- **Internal Structure**: `base_ptr`, `cursor`, `capacity`.
- **Thread Safety**: Each worker has its own *local* allocator for *spawning* new data.
- **Cross-Thread Access**: Allocated data (Jobs) must be `Send` and `Sync` (if shared) because they can be stolen by other workers. The backing memory of the allocator must remain valid as long as any job references it.

### 2. Job Struct Modification
Currently:
```rust
pub enum Work {
    Simple(Box<dyn FnOnce() + Send + 'static>),
    // ...
}
```
Proposed:
```rust
pub enum JobAllocation {
    Heap(Box<dyn FnOnce() + Send + 'static>),
    Frame(*mut (dyn FnOnce() + Send + 'static)), // Unsafe pointer to arena memory
}

pub struct Job {
    pub work: JobAllocation,
    // ...
}
```
*Note*: We might use a custom wrapper `FrameBox<T>` to encapsulate the unsafe pointer and implement `Send`/`Sync` appropriately.

### 3. Lifetime Management & Safety
This is the hardest part. `Box` owns the data. `FrameBox` just points to it.
- **Invariant**: The `FrameAllocator` MUST NOT be reset while any job pointing to its memory is still live (in a queue or executing).
- **Frame Sync**: The user must explicitly synchronize (wait for all jobs) before calling `reset_frame()`.
- **Safety Checks**: Ideally, we track active allocations or use a "generation" counter, but for raw speed/simplicity, we might rely on the `wait_for_all` contract.

### 4. Integration
`Worker` struct will own a `FrameAllocator`.
`JobSystem` will expose `run_frame_job(...)` or similar, or we implicitly use the thread-local allocator if we are inside a worker.
Actually, `JobSystem::run` is often called from the main thread or within jobs.
- **From Main Thread**: Use a main-thread allocator or just fallback to `Box`.
- **From Worker (Nested)**: Use the `Worker`'s current allocator context.

## Trade-offs
- **Complexity**: Introduction of `unsafe` pointers and manual lifetime management (via frame sync boundaries).
- **Memory Usage**: Pre-allocated arenas consumption vs on-demand heap.
- **Safety**: Risk of Use-After-Free if `reset()` is called prematurely. Debug mode guards should be added (e.g., poison memory on reset).
