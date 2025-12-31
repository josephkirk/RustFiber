# Spec Delta: Tiered Spillover System

## ADDED Requirements

### Requirement: [RSF-PIN-004] Tiered Spillover Strategy
The system SHALL provide a `TieredSpillover` pinning strategy that dynamically manages core activation based on workload intensity.

#### Scenario: Low Workload
- **Given** a `JobSystem` initialized with `TieredSpillover`.
- **And** the number of active jobs is less than 6.
- **When** new jobs are submitted.
- **Then** those jobs SHALL preferredly be executed by threads pinned to Tier 1 (CCD0 Physical).
- **And** threads pinned to Tier 2 and Tier 3 SHOULD remain in a low-power/yielding state.

#### Scenario: High Workload (Physical Expansion)
- **Given** Tier 1 workers are saturated (>80% utilization).
- **When** new jobs are submitted.
- **Then** threads pinned to Tier 2 (CCD1 Physical) SHALL "wake up" and participate in fetching and executing jobs.

#### Scenario: Peak Workload (SMT Spillover)
- **Given** Tier 1 and Tier 2 workers are saturated (>80% of total physical capacity).
- **When** the backlog continues to grow.
- **Then** threads pinned to Tier 3 (SMT) SHALL be activated to provide additional throughput.

### Requirement: [RSF-PIN-005] Active Job Tracking
The `WorkerPool` SHALL maintain an atomic count of currently executing jobs to drive the spillover threshold logic.

#### Scenario: Accurate Scaling
- **Given** 10 jobs are currently running.
- **When** 2 jobs complete.
- **Then** the active worker count SHALL decrease to 8.
- **And** the spillover logic SHALL immediately reflect this state to determine which tiers should remain active.
