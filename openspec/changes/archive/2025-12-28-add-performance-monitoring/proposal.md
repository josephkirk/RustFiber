# Proposal: Add Performance Monitoring

## Why
The ARCHITECTURE.md document identifies performance monitoring as a needed feature for debugging performance issues. Currently, there are no built-in metrics to track job throughput or queue depths, making it difficult to diagnose bottlenecks in the job scheduling system. This is particularly important for high-performance applications where small inefficiencies can have significant impacts.

## What Changes
We will add optional metrics collection to the job system that tracks:
- Job throughput (jobs processed per second)
- Queue depths for local worker queues and global injectors
- Configuration to enable/disable metrics with minimal overhead

The metrics will be collected using atomic counters and exposed via a query interface for debugging and monitoring.

## Verification Plan
1. **Unit Tests**: Verify metrics counters update correctly in single-threaded scenarios.
2. **Integration Tests**: Test metrics accuracy in multi-threaded job execution.
3. **Benchmark Integration**: Update benchmarks to optionally report metrics.
4. **Performance Benchmark**: Ensure overhead is <5% when enabled.

## Risks
- **Overhead**: Even with atomics, counters could add measurable overhead in hot paths.
- **Accuracy**: Queue depth approximations may not be perfectly accurate due to concurrent access.
- **Complexity**: Adding metrics increases code complexity and maintenance burden.