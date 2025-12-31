# Spec: NUMA-Local Zeroing and Prefaulting

## Status: NOT IMPLEMENTED
**Implementation blocked by Windows compatibility issues.** Manual prefaulting causes guard page violations on Windows due to strict stack probing requirements. The framework is in place for future implementation when a Windows-compatible approach is found.

## Requirements
1.  **First-Touch Guarantee**: Fiber stacks must be physically backed by RAM on the same NUMA node as the Worker thread.
2.  **Mechanic**:
    - When a `Fiber` is created (which happens on the Worker thread), iterating through the stack memory range.
    - Write a byte (0) to each 4096-byte offset (Page Size).
    - This forces the OS to handle the page fault immediately on the current core.
3.  **Optimization**: The compiler must not optimize away these writes (use `std::ptr::write_volatile`).

## Current Implementation
### `src/fiber.rs`
- ✅ Added `prefault_stack_memory()` method with lazy prefaulting framework
- ✅ Added `stack_prefaulted` flag and `prefetch_pages` configuration
- ❌ Actual prefaulting logic disabled due to Windows guard page violations

### `src/job_system.rs`
- ✅ Added `prefetch_pages: bool` to `FiberConfig` for future use

## Edge Cases
- **Guard Pages**: Be careful not to write to the guard page (if `corosensei` adds one). `DefaultStack` usually has a guard page at the bottom.
- **Windows Stack Probing**: Windows requires strict adherence to stack growth patterns. Manual writes to stack memory violate this.

## Future Implementation Options
1. **Post-Coroutine Creation**: Prefault after coroutine is fully initialized but before first resume
2. **Platform-Specific APIs**: Use Windows VirtualAlloc with MEM_COMMIT for explicit page commitment
3. **Custom Stack Allocator**: Implement custom stack allocation in corosensei with proper Windows support
