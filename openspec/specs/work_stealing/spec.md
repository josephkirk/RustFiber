# work_stealing Specification

## Purpose
TBD - created by archiving change optimize-workstealing. Update Purpose after archive.
## Requirements
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
(Refined to explicitly mention randomized entry or backoff)
Workers SHALL yield the CPU or park when no work is found after a defined search duration, incorporating randomized back-off before entering deep sleep.

### Requirement: Local Submission
Jobs spawned from within an existing job context SHALL be submitted to the current worker's local queue when possible, bypassing global synchronization.

#### Scenario: Contention Reduction
- **Given** a worker executing Job A.
- **When** Job A spawns Job B using its execution context.
- **Then** Job B MUST be pushed to the worker's local deque, NOT the global injector.
- **And** the operation SHALL NOT lock the global injector.

### Requirement: Randomized Back-off
Workers SHALL implement a randomized back-off or wait duration when a steal attempt fails, to prevent synchronized polling loops (thundering herd).

#### Scenario: Contention Mitigation
- **Given** multiple idle workers attempting to steal from the same victim (or any victim).
- **When** a steal attempt fails (returns Empty or Retry).
- **Then** the worker MUST wait for a randomized duration (or perform a randomized number of spin loops) before the next attempt.
- **And** this duration SHOULD increase exponentially with repeated failures (up to a limit) or be sufficiently random to desynchronize threads.

