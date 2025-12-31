# Capability: Benchmark Control

## MODIFIED Requirements

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
