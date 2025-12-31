# Add Criterion Throughput Benchmark

## Why

The latency benchmark (`fiber_switch.rs`) measures raw context-switch overhead. However, it does not evaluate how well the system saturates all CPU cores under heavy load. A **throughput benchmark** is needed to:

- Measure jobs/second when spawning 1,000,000+ tiny tasks
- Test the Chase-Lev work-stealing deque under contention
- Validate the "helping" protocol where the main thread assists workers while waiting

## What Changes

Add a new criterion benchmark file `benches/throughput.rs` that:

1. Spawns 1,000,000 minimal jobs using `run_with_context()` and shared Counter
2. Measures time to complete all jobs (jobs/second throughput)
3. Tests at multiple thread counts for scaling analysis

## Affected Components

| Component | Change |
|-----------|--------|
| `Cargo.toml` | Add `[[bench]]` section for throughput benchmark |
| `benches/throughput.rs` | New criterion benchmark file |
