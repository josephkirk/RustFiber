# Design: Performance Monitoring

## Overview
The performance monitoring system will be implemented as an optional component that collects metrics on job throughput and queue depths. It will be designed to have minimal overhead when disabled and thread-safe collection when enabled.

## Architecture

### Metrics Collection
- **Metrics Struct**: A `Metrics` struct containing atomic counters for jobs processed, queue operations, etc.
- **Shared State**: An `Arc<Metrics>` shared across all workers and the job system.
- **Conditional Compilation**: Use a feature flag `metrics` to enable/disable the functionality.

### Job Throughput
- Track total jobs completed using an atomic counter.
- Calculate throughput as jobs per second over a time window.
- Update counter on job completion in worker loop.

### Queue Depths
- For local queues: Since `crossbeam::deque::Worker` doesn't expose length, maintain separate atomic counters for pushes and pops per worker.
- For global injectors: Similarly, track pushes and pops.
- Approximate depth as pushes - pops (may not be exact due to concurrent access).

### Integration Points
- **Worker**: Increment job counter on completion, track local queue operations.
- **Job System**: Track global injector operations.
- **Configuration**: Add `enable_metrics` flag to `JobSystemConfig`.

### Thread Safety
- All metrics use `AtomicU64` for lock-free updates.
- No blocking operations in hot paths.

### Overhead Minimization
- When disabled, metrics struct is not allocated.
- Counters are simple atomic increments.
- No heap allocations in hot paths.

### Export Interface
- Provide a method to snapshot current metrics.
- Possibly integrate with external monitoring systems in the future.

## Trade-offs
- **Accuracy vs Performance**: Approximate queue depths for speed.
- **Granularity**: Per-worker vs global metrics.
- **Memory**: Small fixed overhead when enabled.