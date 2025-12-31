# Fiber Pooling Specification

## Purpose
Minimize memory allocation overhead during high-frequency job execution.

## ADDED Requirements
### Requirement: Stack Reuse
The system SHALL reuse fiber stacks for subsequent jobs to avoid allocation costs.

#### Scenario: High Frequency Job Spawn
- **Given** a loop spawning 10,000 small jobs.
- **When** the jobs complete.
- **Then** the number of stack allocations should be bounded by the number of concurrent fibers, not the total number of jobs.

### Requirement: Safe Reset
Recycled fibers SHALL have their state (instruction pointer, stack pointer) reset safely before reuse.

#### Scenario: Clean Slate
- **Given** a fiber that completed a previous job.
- **When** it is allocated for a new job.
- **Then** the new job starts with a fresh stack frame and no residual state from the previous job.
