# Fiber Execution Specification

## Purpose
Enable true cooperative multitasking via user-space context switching.

## ADDED Requirements
### Requirement: Stackful Execution
The system SHALL use stackful fibers (coroutines) to execute jobs, enabling suspension from any stack depth.

#### Scenario: Deep Yield
- **Given** a job with a nested function call depth of 10.
- **When** the deepest function calls `wait_for_counter`.
- **Then** the entire call stack is preserved while the fiber is suspended.

### Requirement: Non-Blocking Yield
Yielding a fiber SHALL NOT yield the OS thread if other fibers are ready to run.

#### Scenario: Cooperative Switch
- **Given** two fibers scheduled on the same worker thread.
- **When** Fiber A yields.
- **Then** the worker thread immediately switches context to Fiber B (microseconds) instead of yielding to the OS (milliseconds).
## Requirements
### Requirement: Detached Execution
The system SHALL support spawning jobs without creating an associated synchronization primitive (Counter), avoiding allocation overhead.

#### Scenario: Fire-and-Forget
- **Given** a parent job spawning a child job for side effects.
- **When** the parent calls `spawn_detached`.
- **Then** the system MUST schedule the job.
- **And** the system MUST NOT allocate a new `Counter` on the heap.
- **And** the system MUST NOT return a `Counter` to the parent.

### Requirement: Grouped Execution
The system SHALL support spawning multiple jobs that share a single synchronization primitive (Counter).

#### Scenario: Shared Counter
- **Given** a parent job with a pre-allocated `Counter` initialized to N.
- **When** the parent spawns N jobs using `spawn_with_counter` passing the existing Counter.
- **Then** each job MUST decrement the shared Counter upon completion.
- **And** the system MUST NOT allocate a new internal Counter for each job.

