# benchmark-control Specification

## ADDED Requirements

### Requirement: The system SHALL provide a criterion throughput benchmark

The job system SHALL include a criterion benchmark that measures job throughput when spawning large batches of tasks.

#### Scenario: Running throughput benchmark

Given the criterion benchmark harness,
When `cargo bench --bench throughput` is executed,
Then the `spawn_1m_jobs` benchmark spawns 1,000,000 jobs and reports total time.

#### Scenario: Benchmark uses multi-threaded JobSystem

Given the throughput benchmark,
When the benchmark executes,
Then it uses a multi-threaded JobSystem to test work-stealing saturation.

### Requirement: The throughput benchmark SHALL use shared Counter for coordination

The benchmark SHALL use a shared `Counter` to track batch completion and trigger the helping protocol.

#### Scenario: Shared Counter usage

Given a batch of 1,000,000 jobs,
When jobs are spawned with `spawn_with_counter()`,
Then all jobs share a single Counter initialized to 1,000,000.

#### Scenario: Helping protocol activation

Given the main thread calls `wait_for_counter()`,
When workers are processing jobs,
Then the main thread enters helping mode and assists with job execution.
