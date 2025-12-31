# Spec: Panic Handling

## ADDED Requirements

#### Scenario: Worker Recovery
Given a worker executing a job
When the job panics
Then the worker thread catches the panic
And the worker thread continues to process subsequent jobs

#### Scenario: Fiber Reset
Given a fiber that panicked
When it is returned to the pool
Then it is reset cleanly before reuse
