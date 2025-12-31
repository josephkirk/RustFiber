# Fiber Prioritization Specification

## Purpose
Prioritize critical gaming tasks (e.g., physics, input) over background tasks (e.g., AI, loading) to ensure frame-rate stability.

## ADDED Requirements
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
