# Implement Core Fiber Switch

Implement true stackful coroutines for `RustFiber` to enable user-space cooperative multitasking.

## Context
Currently, `RustFiber` relies on `std::thread::yield_now()`, which blocks the entire OS thread, defeating the purpose of a fiber system. To achieve the "Zero OS Interference" and "Lock-Free" pillars of the architecture, we must implement true stackful fibers that can yield execution without yielding the thread.

## Requirements
### Requirement: Stackful Fibers
The system SHALL provide a `Fiber` abstraction that utilizes `corosensei` to manage a separate stack and execution context.

#### Scenario: Fiber Yielding
- **Given** a job running inside a fiber.
- **When** the job calls `Context::yield_fiber()`.
- **Then** the fiber's execution is suspended, its register state is saved, and control returns to the worker thread's scheduler loop without blocking the OS thread.

### Requirement: Fiber Pooling
The system SHALL pool `Fiber` stacks to avoid the overhead of `mmap`/`VirtualAlloc` on every job submission.

#### Scenario: Fiber Recycling
- **Given** a fiber that has completed its job.
- **When** the worker finishes processing the result.
- **Then** the fiber is returned to a `FiberPool` and reset for the next job.

### Requirement: Lock-Free Intrusive Waiting
The system SHALL implement a lock-free wait list using intrusive nodes embedded in the `Fiber` struct, eliminating heap allocations during suspension.

#### Scenario: Zero-Allocation Wait
- **Given** a fiber waiting on a counter.
- **When** `wait_for_counter` is called.
- **Then** the fiber links its own `WaitNode` into the counter's atomic list using `compare_exchange`, without allocating memory.
- **Unsafe Code**: Implementing context switching requires `unsafe` code. Use of `corosensei` mitigates this but does not eliminate the need for careful lifetime management.
- **Stack Memory**: Each fiber requires pre-allocated stack memory (e.g., 64KB). A pool of 1000 fibers consumes 64MB of RAM. This is acceptable for modern PCs but requires tuning constants.
