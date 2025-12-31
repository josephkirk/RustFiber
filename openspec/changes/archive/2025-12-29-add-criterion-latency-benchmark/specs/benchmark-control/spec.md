# benchmark-control Specification

## ADDED Requirements

### Requirement: The system SHALL provide criterion-based micro-benchmarks for latency measurement

The job system SHALL include criterion benchmark(s) that measure fiber switch latency at nanosecond precision.

#### Scenario: Running fiber switch latency benchmark

Given the criterion benchmark harness,
When `cargo bench --bench fiber_switch` is executed,
Then the `fiber_switch_latency` benchmark runs with statistical warmup and reports median latency.

#### Scenario: Benchmark uses single-threaded JobSystem

Given the fiber switch latency benchmark,
When the benchmark executes,
Then it uses a single-threaded JobSystem to isolate fiber switching from work-stealing overhead.

### Requirement: The benchmark SHALL report latency with sub-microsecond precision

The fiber switch latency benchmark SHALL use criterion for statistical rigor and report results in nanoseconds.

#### Scenario: Performance validation

Given a clean benchmark run with warm fiber pool,
When `cargo bench --bench fiber_switch` completes,
Then the reported median latency is available in nanoseconds.

#### Scenario: Performance regression detection

Given criterion baseline comparison,
When the benchmark detects regression,
Then the regression percentage is flagged in the benchmark report.
