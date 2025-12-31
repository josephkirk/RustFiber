# Refactor and Unify Benchmarks

## Goal
Transform the current ad-hoc benchmark suite into a rigorous performance engineering tool. The goal is to measure:
1.  **Overhead/Latency**: Cost of scheduling empty jobs.
2.  **Throughput**: Raw execution speed of dependency graphs.
3.  **Scaling**: Efficiency of `ParallelFor` across cores.
4.  **Load Balancing**: Efficiency of work-stealing under skewed loads.
5.  **Real-world Scene**: Simulation of a transform hierarchy update.

## Context
The current benchmarks (`fibonacci`, `quicksort`, `nas`) are good for general checks but don't isolate specific job system characteristics like dispatch overhead or steal efficiency. We need targeted micro-benchmarks.

## Key Changes
1.  **New Benchmark Modules**:
    - `latency`: Empty job scheduling cost.
    - `throughput`: Dependency graph processing.
    - `parallel_for`: Data-parallel scaling.
    - `stress`: Skewed workloads for work-stealing.
    - `transform`: Hierarchy update simulation.
2.  **Metrics Integration**:
    - Use the `metrics` feature to capture internal scheduler stats (steals,parks, wakeups) during benchmark runs.
3.  **Unified Output**:
    - Standardize JSON output to include "Target" vs "Actual" for key metrics.
4.  **Documentation & Context**:
    - Update `BENCHMARKS.md` to map technical metrics to real-world scenarios (e.g., "Empty Job" -> "AI Triggers").

## Risks
- **High Variance**: Micro-benchmarks like "empty job" are sensitive to OS noise. We need robust statistical aggregation (p99, p50).
- **Measurement Overhead**: Measuring time inside the job loop can introduce observer effect.
