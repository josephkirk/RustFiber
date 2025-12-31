## MODIFIED Requirements

### Requirement: Latency and Responsiveness
The system SHALL provide low-latency execution for cold-start jobs.

#### Scenario: Cold path submission
- **WHEN** a job is submitted to an idle system
- **THEN** execution MUST begin within 10µs (excluding system scheduler noise).
- **AND** workers MUST NOT sleep for fixed intervals > 1ms when work is available.
- **AND** high-priority jobs MUST trigger immediate worker wakeup.

## ADDED Requirements

### Requirement: Adaptive Parking
The system SHALL employ an adaptive parking strategy to balance latency and CPU usage.

#### Scenario: Adaptive spin
- **WHEN** a worker becomes idle
- **THEN** it SHALL spin/yield for a brief period (e.g., up to 10-50µs) before parking.
- **AND** if work arrives during spinning, it SHALL resume immediately without syscall overhead.
