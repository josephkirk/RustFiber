# Intrusive Waiting Specification

## Purpose
Eliminate memory allocation and lock contention during fiber suspension compared to standard `Vec` or `Mutex` based waiting.

## ADDED Requirements
### Requirement: Zero-Allocation Suspension
The system SHALL use intrusive nodes embedded within the `Fiber` structure for wait lists, requiring zero heap allocations to suspend a fiber.

#### Scenario: High Frequency Wait
- **Given** a fiber executing a job.
- **When** it calls `wait_for_counter` on a dependency.
- **Then** no calls to the global allocator (malloc) occur; the fiber simply links itself into the counter's list.

### Requirement: Batch Waking
The system SHALL wake waiting fibers in bulk when a counter reaches zero, minimizing synchronization overhead.

#### Scenario: Thundering Herd Mitigation
- **Given** 50 fibers waiting on a single counter.
- **When** the counter reaches zero.
- **Then** the waking thread claims the entire list with a single atomic operation and schedules them, rather than acquiring a lock 50 times.
