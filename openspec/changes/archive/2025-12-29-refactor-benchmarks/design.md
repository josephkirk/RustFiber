# Design: Benchmark Architecture

## Benchmark Categories

| Name | Type | Goal | Metric |
|---|---|---|---|
| `empty_job` | Latency | Measure dispatch/schedule overhead | Time per job (ns) |
| `dependency_graph` | Throughput | Measure graph resolution speed | Jobs/sec |
| `parallel_for` | Scaling | Measure linear scaling on arrays | Speedup Factor (vs serial) |
| `skewed_workload` | Efficiency | Measure work-stealing effectiveness | CPU Utilization % |
| `transform_hierarchy` | Simulation | Real-world game loop scenario | Frame time (ms) |

## Implementation Details

### `Empty Job`
Loop scheduling `N` jobs that do `return`. Measure `(Total Time / N)`.

### `ParallelFor`
Use `job_system.parallel_for` on a `Vec<f32>` of 1M elements.
- Run `seq` (serial).
- Run `par` (parallel).
- `Scaling Factor = T_seq / T_par`.

### `Skewed Workload`
- Worker 0: Heavy tasks (Matrix Mul).
- Worker 1..N: Empty tasks.
- Verify if Worker 1..N steal chunks from Worker 0 effectiveley.

### `Transform Hierarchy`
- Flat array of `Transform` struct.
- `parent_indices` array.
- Process in levels (dependency layers).
- Measure total update time.

## Metrics Feature Integration
In `utils.rs`:
```rust
#[cfg(feature = "metrics")]
pub fn capture_internal_metrics() -> Option<InternalMetrics> {
    // Access global metric store
}
```
Include these in the `BenchmarkResult` JSON.
