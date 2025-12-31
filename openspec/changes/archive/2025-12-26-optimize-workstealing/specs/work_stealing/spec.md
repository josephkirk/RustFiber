# Work Stealing Specification

## Purpose
Maximize CPU utilization and data locality by dynamically balancing work across threads.

## ADDED Requirements

### Requirement: LIFO Local Access
Workers SHALL process their own locally spawned jobs in Last-In, First-Out (LIFO) order.

#### Scenario: Recursive Locality
- **Given** a worker that spawns Job A, then Job B.
- **When** the worker pops the next job.
- **Then** it MUST receive Job B (the most recently received).

### Requirement: FIFO Stealing
Workers SHALL steal jobs from other workers in First-In, First-Out (FIFO) order.

#### Scenario: Stealing Roots
- **Given** a victim worker with Job A (oldest) and Job B (newest) in its queue.
- **When** a thief steals from this worker.
- **Then** the thief MUST receive Job A.

### Requirement: Injector Priority
Workers SHALL check the global injector queue when local work is exhausted but BEFORE entering a deep idle state.

#### Scenario: Injector Fallback
- **Given** a worker with an empty local queue.
- **When** the worker looks for work.
- **Then** it attempts to pop from the global injector or steal from peers before sleeping.

### Requirement: Deep Idle
Workers SHALL yield the CPU or park when no work is found after a defined search duration.

#### Scenario: Conservation
- **Given** a system with no active jobs.
- **When** a worker has failed to find work for multiple cycles.
- **Then** the worker MUST call `thread::yield_now` or block to reduce CPU usage.
