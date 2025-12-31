# Capability: Benchmark Control
## Requirements
### Requirement: [REQ-BM-001] Benchmark Timeout
The benchmark system SHALL limit the duration of each individual benchmark test to a maximum of 60 seconds by default.

#### Scenario: Benchmark exceeds timeout
- **Given** a benchmark test that takes more than 60 seconds to complete all its pre-defined data points.
- **When** the benchmark runs.
- **Then** the benchmark SHALL stop collecting new data points after the 60-second limit is reached and return the results collected up to that point.

### Requirement: [REQ-BM-002] Incremental Output
The benchmark system SHALL output the results of each benchmark individually as soon as it completes.

#### Scenario: Streamed results
- **Given** multiple benchmark tests to be run.
- **When** the benchmark runner is executed.
- **Then** each benchmark result SHALL be printed to stdout as a single JSON object (or line) immediately after completion.

### Requirement: [REQ-BM-003] Incremental Graphing
The benchmark visualization script SHALL generate graphs for each benchmark test as soon as its results are available.

#### Scenario: Real-time graph generation
- **Given** the benchmark runner is streaming results.
- **When** a result for a specific benchmark is received by the visualization script.
- **Then** the script SHALL generate and save the corresponding PNG graph immediately.

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

### Requirement: Efficiency Visualization
The benchmark visualization system MUST plot Parallel Efficiency ($\frac{Speedup}{Threads}$) on a secondary Y-axis for scaling benchmarks.

#### Scenario: ParallelFor Scaling with Efficiency Axis
- **WHEN** the ParallelFor Scaling benchmark is visualized
- **THEN** the graph MUST show:
  - Primary Y-axis: Speedup (x factor)
  - Secondary Y-axis: Efficiency (0.0 to 1.0)
  - Efficiency plotted as a distinct line style (e.g., dashed)

#### Scenario: Efficiency Calculation
- **GIVEN** normalized speedup values at various thread counts
- **WHEN** efficiency is computed
- **THEN** Efficiency = Speedup / Threads for each data point

#### Scenario: Visual Distinction
- **WHEN** both Speedup and Efficiency are plotted
- **THEN** they MUST use distinct colors and line styles to avoid confusion
- **AND** a legend clearly labeling both metrics MUST be present

