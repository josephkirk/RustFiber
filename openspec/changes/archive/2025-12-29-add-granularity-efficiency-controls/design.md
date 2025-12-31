# Design: Granularity Controls and Efficiency Visualization

## Context
Work-stealing job systems are sensitive to job granularity. Jobs that are too fine-grained incur more scheduling overhead than computation time, leading to poor parallel efficiency. The current `parallel_for_auto` uses a simple `len / (num_workers * 4)` heuristic that does not account for per-element work cost.

Benchmarks currently show Speedup (serial_time / parallel_time) but not Efficiency (Speedup / Threads). Efficiency reveals:
- Values near 1.0: near-ideal scaling
- Values < 1.0: wasted CPU cycles due to overhead, contention, or imbalanced work

## Goals / Non-Goals
**Goals:**
- Provide explicit guidance on choosing batch sizes for `parallel_for`
- Add a `GranularityHint` mechanism for common workload patterns
- Plot Efficiency as a secondary Y-axis in ParallelFor Scaling benchmarks

**Non-Goals:**
- Dynamic runtime granularity adjustment (future work)
- Automatic profiling-based tuning

## Decisions

### Decision 1: GranularityHint Enum
Add an optional hint for `parallel_for_auto` variants:

```rust
pub enum GranularityHint {
    /// Very cheap per-element work (< 100 cycles). Use large batches.
    Trivial,
    /// Light computation (100-1000 cycles). Default.
    Light,
    /// Moderate computation (1K-10K cycles). Smaller batches OK.
    Moderate,
    /// Heavy computation (> 10K cycles). Fine granularity acceptable.
    Heavy,
}
```

The auto-tuning formula becomes:
- `Trivial`: `target_batches = num_workers * 2`
- `Light`: `target_batches = num_workers * 4` (current default)
- `Moderate`: `target_batches = num_workers * 8`
- `Heavy`: `target_batches = num_workers * 16`

**Alternative considered:** Pass an explicit `estimated_cycles_per_element` parameter. Rejected for being too low-level and hard to estimate accurately.

### Decision 2: Efficiency Visualization
Add a secondary Y-axis to the ParallelFor Scaling graph showing Efficiency (0.0-1.0 scale). This requires:
1. Computing efficiency from speedup after normalization
2. Plotting on `ax.twinx()` with distinct color/style

## Risks / Trade-offs
- **Risk**: GranularityHint may be ignored or misused.
  - Mitigation: Clear documentation and sensible defaults.
- **Trade-off**: Adding a secondary axis increases visual complexity.
  - Acceptable: Efficiency is critical for performance analysis.

## Open Questions
- Should `parallel_for_chunked_auto` also accept `GranularityHint`? (Proposed: yes)
