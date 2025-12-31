# Throughput Benchmark Spec

## Overview
Measures the raw execution speed of dependency graphs and parallel operations.

## Test Cases
- Dependency Graph: Fork-join with varying task counts
- ParallelFor Scaling: Data-parallel scaling on arrays

## Metrics
- Tasks/sec for dependency graphs
- Scaling factor (serial/parallel time) for ParallelFor

## Implementation
- Dependency graph uses `run_with_context` with `spawn_job`
- ParallelFor uses `parallel_for_chunked_auto` on fibonacci computation
- Warmup and timeout as standard