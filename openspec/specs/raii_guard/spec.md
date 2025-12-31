# Spec: RAII Counter Guard

## ADDED Requirements

#### Scenario: Job Panics
Given a job with a dependency counter
When the job panics during execution
Then the counter is still decremented
And the system does not deadlock
