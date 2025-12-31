## ADDED Requirements

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
