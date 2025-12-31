# Latency Benchmark Spec

## Overview
Measures the overhead of scheduling and executing empty jobs in the job system.

## Test Cases
- Vary number of empty jobs from 1 to 1M
- Measure total time and compute ns per job

## Metrics
- Time per job (ns)
- Throughput (jobs/sec)

## Implementation
- Use `parallel_for_chunked_auto` with empty closure
- Warmup with local jobs
- Timeout after 60s