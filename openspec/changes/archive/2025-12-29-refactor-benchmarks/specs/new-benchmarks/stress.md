# Stress Benchmark Spec

## Overview
Tests work-stealing efficiency under skewed workloads.

## Test Cases
- One heavy worker (many heavy tasks)
- Multiple light workers (few empty tasks)
- Measure total time for completion

## Metrics
- Total time (ms)
- CPU utilization (implied)

## Implementation
- Use `run_with_context` to spawn heavy and light jobs
- Heavy jobs: fibonacci(20) loops
- Light jobs: empty
- Verify work stealing balances load