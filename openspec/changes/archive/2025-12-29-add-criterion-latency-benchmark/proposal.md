# Add Criterion Nanosecond Latency Benchmark

## Summary

Introduce a **criterion-based nanosecond latency benchmark** to verify that fiber context switching operates within the target 20–50 ns range. This benchmark measures the cost of a minimal job execution cycle using the existing `run()` + `wait_for_counter()` API.

## Motivation

Precise measurement of context-switch latency is critical for performance validation. The current benchmark infrastructure (`src/benchmarks/`) measures aggregate throughput. A specialized micro-benchmark using criterion provides:

- **Statistical rigor**: Automatic warmup, outlier detection, regression tracking
- **Sub-microsecond precision**: criterion is designed for nanosecond-scale measurements
- **Reproducibility**: Standardized benchmark harness with HTML reports

## Approach

Use the existing minimal-overhead path:
```rust
let counter = job_system.run(|| { /* empty */ });
job_system.wait_for_counter(&counter);
```

This measures the full round-trip: **dispatch → fiber switch → execute → complete → signal → wake**.

## Optimization Goal

If results exceed **100 ns**, investigate:
- Unnecessary register saving in the context switch logic (corosensei)
- Heap allocations within the switch path  
- Counter signaling overhead

## Dependencies

- **criterion** (as dev-dependency)

## Affected Components

| Component | Change |
|-----------|--------|
| `Cargo.toml` | Add `criterion` dev-dependency and `[[bench]]` section |
| `benches/fiber_switch.rs` | New criterion benchmark file |
