# Tasks: Implement Frame Linear Allocators

- [x] Implement `FrameAllocator` struct with bump allocation logic (alloc, reset) <!-- id: 0 -->
- [x] Add `FrameAllocator` to `Worker` struct (thread-local or per-worker ownership) <!-- id: 1 -->
- [x] Implement `Job::new_in_allocator` or similar API to create jobs using the allocator <!-- id: 2 -->
- [x] Update `Job` struct to hold `&'a dyn FnOnce` (or unsafe raw pointer wrapper) instead of `Box` <!-- id: 3 -->
- [x] Update `JobSystem::run` families to use the allocator if available <!-- id: 4 -->
- [x] Implement `reset` mechanism for `FrameAllocator` at frame boundaries <!-- id: 5 -->
- [x] Create benchmark comparing `Box` vs `FrameAllocator` job spawning <!-- id: 6 -->
- [x] Verify no memory leaks or use-after-free with Miri or ASAN <!-- id: 7 -->
