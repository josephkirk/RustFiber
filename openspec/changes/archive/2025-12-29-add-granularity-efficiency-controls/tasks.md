# Tasks: Add Granularity Controls and Efficiency Visualization

## 1. API Enhancements (job_system.rs)
- [x] 1.1 Add `GranularityHint` enum with `Trivial`, `Light`, `Moderate`, `Heavy` variants
- [x] 1.2 Add `parallel_for_with_hint` method accepting `GranularityHint`
- [x] 1.3 Add `parallel_for_chunked_with_hint` method accepting `GranularityHint`
- [x] 1.4 Update `parallel_for_auto` documentation with granularity guidance

## 2. Efficiency Visualization (run_benchmarks.py)
- [x] 2.1 Compute Efficiency (Speedup / Threads) after normalizing speedup
- [x] 2.2 Add secondary Y-axis for Efficiency in ParallelFor Scaling graph
- [x] 2.3 Style Efficiency line/points distinctly (e.g., dashed line, different color)
- [x] 2.4 Add legend entries for both Speedup and Efficiency

## 3. Documentation
- [x] 3.1 Update `ARCHITECTURE.md` granularity section with explicit guidance
- [x] 3.2 Add docstring examples showing GranularityHint usage

## 4. Validation
- [x] 4.1 Run `cargo test` ensuring new APIs compile and basic tests pass
- [x] 4.2 Run `uv run run_benchmarks.py linear 8` and verify Efficiency axis appears
- [x] 4.3 Visual inspection: Efficiency should be near 1.0 at low thread counts, decreasing at high counts
