## ADDED Requirements

### Requirement: Local Submission
Jobs spawned from within an existing job context SHALL be submitted to the current worker's local queue when possible, bypassing global synchronization.

#### Scenario: Contention Reduction
- **Given** a worker executing Job A.
- **When** Job A spawns Job B using its execution context.
- **Then** Job B MUST be pushed to the worker's local deque, NOT the global injector.
- **And** the operation SHALL NOT lock the global injector.
