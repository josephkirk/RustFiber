# job_batching Specification

## Purpose
TBD - created by archiving change implement-job-batching. Update Purpose after archive.
## Requirements
### Requirement: `JobSystem::parallel_for`
The `JobSystem` MUST provide a `parallel_for` method to execute a loop in parallel chunks.

#### Scenario: Manual Batching
Given a range `0..100` and batch size `10`
When `parallel_for` is called
Then 10 jobs should be scheduled, each processing 10 items.
And the returned counter should signal completion of all items.

#### Scenario: Uneven Batching
Given a range `0..105` and batch size `10`
When `parallel_for` is called
Then 11 jobs should be scheduled (10 jobs of 10, 1 job of 5).

#### Scenario: Documentation Guidance
- **WHEN** a developer reads the `parallel_for` documentation
- **THEN** explicit guidance on choosing batch sizes based on per-element work cost MUST be present

### Requirement: `Context::parallel_for`
The `Context` MUST provide a `parallel_for` method to execute nested loops in parallel chunks using the parent fiber's context.

#### Scenario: Nested Batching
Given a job running inside a fiber
When it calls `ctx.parallel_for`
Then child jobs should be spawned representing the chunks.

### Requirement: GranularityHint for Parallel For
The `JobSystem` MUST provide a `GranularityHint` enum to guide automatic batch size selection based on expected per-element work cost.

#### Scenario: Using Trivial Hint for Cheap Work
- **WHEN** `parallel_for_with_hint` is called with `GranularityHint::Trivial` on a range of 1,000,000 elements
- **THEN** the batch size should be larger (fewer total jobs) than the default Light hint

#### Scenario: Using Heavy Hint for Expensive Work
- **WHEN** `parallel_for_with_hint` is called with `GranularityHint::Heavy` on a range of 1,000 elements
- **THEN** the batch size should be smaller (more jobs, finer granularity) to enable better load balancing

### Requirement: `parallel_for_with_hint`
The `JobSystem` MUST provide a `parallel_for_with_hint` method that accepts a `GranularityHint` and adjusts batch size accordingly.

#### Scenario: Hint-Based Auto Batching
- **GIVEN** a job system with 8 workers
- **WHEN** `parallel_for_with_hint(0..10000, GranularityHint::Moderate, body)` is called
- **THEN** approximately `8 * 8 = 64` batches should be created (element count permitting)

### Requirement: `parallel_for_chunked_with_hint`
The `JobSystem` MUST provide a `parallel_for_chunked_with_hint` method for chunk-based processing with granularity hints.

#### Scenario: Chunked Processing with Hint
- **WHEN** `parallel_for_chunked_with_hint(0..1000, GranularityHint::Light, body)` is called
- **THEN** the closure receives `Range<usize>` chunks with batch size derived from the Light hint

