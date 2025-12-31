# Capability: Thread Pinning

## ADDED Requirements

### Requirement: [REQ-TP-001] Avoid SMT Contention
The job system SHALL support a pinning strategy that maps worker threads only to even-numbered logical processors (dedicated physical cores).

#### Scenario: SMT avoidance on dual-thread cores
- **Given** a system with 8 physical cores and 16 logical processors (SMT enabled).
- **When** the job system is initialized with `AvoidSMT` strategy and 4 workers.
- **Then** workers SHALL be pinned to logical processors 0, 2, 4, and 6.

### Requirement: [REQ-TP-002] CCD Isolation
The job system SHALL support a pinning strategy that restricts worker threads to the first Core Complex Die (CCD) on systems with multiple CCDs.

#### Scenario: Isolation to CCD1
- **Given** an AMD Ryzen system with 2 CCDs (e.g., 16 cores).
- **When** the job system is initialized with `CCDIsolation` strategy.
- **Then** workers SHALL be pinned only to the first 8 physical cores (Logical 0–14) and SHALL NOT be pinned to processors on the second CCD.

### Requirement: [REQ-TP-003] Strategy Fallback
The job system SHALL handle cases where the selected pinning strategy provides fewer core slots than requested workers.

#### Scenario: More workers than CCD1 slots
- **Given** `CCDIsolation` strategy which provides 8 slots (logical 0, 2, ..., 14).
- **When** the user requests 12 worker threads.
- **Then** the first 8 workers SHALL be pinned to the available slots, and the remaining 4 workers SHALL either wrap around or remain unpinned, ensuring the system remains operational.
