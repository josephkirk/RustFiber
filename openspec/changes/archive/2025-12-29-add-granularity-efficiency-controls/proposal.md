# Change: Add Granularity Controls and Efficiency Visualization

## Why
When each job in `ParallelFor` is too small, job system overhead (scheduling, stealing, context switching) overwhelms the actual computation, leading to poor scaling. Additionally, current benchmark visualizations show Speedup but not **Efficiency** (Speedup/Threads), making it difficult to identify exactly where the system begins to waste CPU cycles.

## What Changes
- **API Enhancement**: Document and improve granularity guidance for `parallel_for*` methods, including a new `GranularityHint` enum for domain-specific tuning.
- **Auto-Tuning Improvement**: Enhance `parallel_for_auto` heuristic to account for expected work cost, not just element count.
- **Visualization**: Add Efficiency metric ($\frac{Speedup}{Threads}$) to ParallelFor Scaling benchmark as a secondary Y-axis, clearly showing scaling degradation.

## Impact
- Affected specs: `job_batching`, `benchmark-control`
- Affected code: `src/job_system.rs`, `src/benchmarks/throughput.rs`, `run_benchmarks.py`
