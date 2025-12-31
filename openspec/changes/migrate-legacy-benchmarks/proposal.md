# Migrate Legacy Benchmarks to Criterion

## Why

The legacy benchmark suite in `src/benchmarks/` uses a custom runner with limited statistical rigor. Criterion provides:

- Automatic warmup and statistical analysis
- Nanosecond precision with confidence intervals
- Regression detection with baseline comparison
- HTML reports for visualization

Migrating to criterion standardizes all benchmarks under one harness and improves measurement quality.

## What Changes

Create new criterion benchmark files in `benches/` for each legacy benchmark, adapting patterns to criterion's API.

### Benchmarks to Migrate

| Legacy File | New Criterion File | Core Functionality |
|-------------|-------------------|-------------------|
| `latency.rs` | `benches/latency.rs` | Scheduling latency per job at various batch sizes |
| `stress.rs` | `benches/work_stealing.rs` | Work-stealing stress with steal metrics |
| `nas_benchmarks.rs` | `benches/scientific.rs` | EP, MG, CG scientific patterns |
| `transform.rs` | `benches/transform.rs` | Game engine hierarchy updates |
| `producer_consumer.rs` | `benches/producer_consumer.rs` | Lock-free queue producer-consumer |
| `allocation.rs` | `benches/allocation.rs` | Frame allocator throughput |
| `startup_latency.rs` | `benches/startup.rs` | JobSystem initialization time |

### Already Migrated (Skip)

| Legacy | Criterion | Status |
|--------|-----------|--------|
| `throughput.rs` (partial) | `benches/throughput.rs` | ✓ Done |
| `latency.rs` (raw switch) | `benches/fiber_switch.rs` | ✓ Done |

## Affected Components

| Component | Change |
|-----------|--------|
| `Cargo.toml` | Add 7 new `[[bench]]` sections |
| `benches/*.rs` | 7 new benchmark files |
| `BENCHMARK.md` | Update documentation |
