# fiber_prioritization Specification

## Purpose
TBD - created by archiving change implement-fiber-yielding-gaps. Update Purpose after archive.
## Requirements
### Requirement: Priority Levels
The system SHALL support at least 3 distinct priority levels for jobs.
#### Scenario: Critical Work
- **Given** a high-priority job and a low-priority job are submitted simultaneously.
- **When** a worker becomes available.
- **Then** the high-priority job is executed first.

### Requirement: Priority Inheritance (Optional)
Child jobs spawned from a parent job SHALL inherit the parent's priority by default.
#### Scenario: Nested Spawn
- **Given** a high-priority job spawns a child job.
- **When** the child job is submitted.
- **Then** it is assigned high priority automatically.

