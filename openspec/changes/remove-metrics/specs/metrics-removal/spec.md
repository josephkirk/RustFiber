# Metric Removal

## REMOVED Requirements
### Requirement: The job system SHALL provide optional performance metrics collection
The internal metrics collection feature is being removed to simplify the codebase.

#### Scenario: Codebase Cleanup
Given the codebase
When inspected
Then the `Metrics` struct and `metrics` feature are absent.

### Requirement: The system SHALL collect metrics on job processing throughput
This requirement is removed.

#### Scenario: No Throughput Collection
Given the job system
When jobs run
Then no internal throughput counters are updated.

### Requirement: The system SHALL collect metrics on queue depths
This requirement is removed.

#### Scenario: No Queue Depth Collection
Given the job system
When jobs run
Then no internal queue depth counters are updated.
