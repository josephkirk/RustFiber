# benchmark-control Specification

## ADDED Requirements

### Requirement: The system SHALL provide criterion-based benchmarks for all core performance metrics

All legacy benchmarks SHALL be migrated to criterion for consistent statistical analysis and regression detection.

#### Scenario: Running latency benchmark

Given the criterion benchmark harness,
When `cargo bench --bench latency` is executed,
Then the scheduling latency benchmark runs and reports per-job latency at various batch sizes.

#### Scenario: Running work-stealing benchmark

Given the criterion benchmark harness,
When `cargo bench --bench work_stealing` is executed,
Then the work-stealing stress benchmark runs with statistical warmup.

#### Scenario: Running scientific benchmarks

Given the criterion benchmark harness,
When `cargo bench --bench scientific` is executed,
Then EP, MG, and CG benchmarks run and report throughput metrics.

### Requirement: Each benchmark SHALL use criterion groups for related measurements

Benchmarks with multiple test cases SHALL organize them using `BenchmarkGroup` with appropriate throughput metrics.

#### Scenario: Grouped benchmarks

Given a benchmark with multiple batch sizes,
When the benchmark runs,
Then results are organized under a common group name (e.g., `latency/100`, `latency/1000`).
