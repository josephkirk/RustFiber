# Spec: Job Batching (`parallel_for`)

## ADDED Requirements

### Requirement: `JobSystem::parallel_for`
The `JobSystem` MUST provide a `parallel_for` method to execute a loop in parallel chunks.

#### Scenario: Manual Batching
Given a range `0..100` and batch size `10`
When `parallel_for` is called
Then 10 jobs should be scheduled, each processing 10 items.
And the returned counter should signal completion of all items.

#### Scenario: uneven Batching
Given a range `0..105` and batch size `10`
When `parallel_for` is called
Then 11 jobs should be scheduled (10 jobs of 10, 1 job of 5).

### Requirement: `Context::parallel_for`
The `Context` MUST provide a `parallel_for` method to execute nested loops in parallel chunks using the parent fiber's context.

#### Scenario: Nested Batching
Given a job running inside a fiber
When it calls `ctx.parallel_for`
Then child jobs should be spawned representing the chunks.
