# RustFiber Benchmarks

This document describes the benchmark suite for RustFiber, including criterion micro-benchmarks and the legacy benchmark runner.

## Quick Start

```bash
# Run all criterion micro-benchmarks
cargo bench

# Run specific criterion benchmark
cargo bench --bench fiber_switch
cargo bench --bench throughput
cargo bench --bench latency
cargo bench --bench work_stealing
cargo bench --bench scientific
cargo bench --bench transform
cargo bench --bench producer_consumer
cargo bench --bench allocation
cargo bench --bench startup

# View HTML reports
open target/criterion/report/index.html
```

---

## Criterion Micro-Benchmarks

Located in `benches/`, these provide statistically rigorous nanosecond-precision measurements.

### `fiber_switch.rs` — Context Switch Latency

Measures the raw cost of fiber context switching.

| Benchmark | Description | Expected Result |
|-----------|-------------|-----------------|
| `raw_fiber_switch` | Pure corosensei context switch (Caller → Fiber → Caller) | **~18 ns** |
| `raw_fiber_with_work` | Context switch + minimal work inside fiber | **~19 ns** |
| `job_system_cold` | Full JobSystem round-trip including 1ms parking timeout | **~500 µs** |

---

### `throughput.rs` — Job Throughput

Measures job throughput when spawning large batches of tasks.

| Benchmark | Description | Expected Result |
|-----------|-------------|-----------------|
| `spawn_1m_jobs` | Spawn 1,000,000 jobs with shared Counter | **~14 M jobs/sec** |
| `throughput_scaling/spawn_1m/N` | Scaling across N threads | Similar throughput |

---

### `latency.rs` — Scheduling Latency

Measures per-job scheduling latency at various batch sizes.

| Benchmark | Description |
|-----------|-------------|
| `scheduling_latency/batch/100` | 100 jobs batch |
| `scheduling_latency/batch/1000` | 1K jobs batch |
| `scheduling_latency/batch/10000` | 10K jobs batch |
| `scheduling_latency/batch/100000` | 100K jobs batch |

---

### `work_stealing.rs` — Work-Stealing Stress

Tests work-stealing under high contention with imbalanced workloads.

| Benchmark | Description |
|-----------|-------------|
| `work_stealing/imbalanced/1000` | 1K imbalanced jobs |
| `work_stealing/imbalanced/10000` | 10K imbalanced jobs |
| `work_stealing/imbalanced/100000` | 100K imbalanced jobs |

---

### `scientific.rs` — NAS Scientific Patterns

Implements NAS Parallel Benchmark patterns.

| Benchmark | Description |
|-----------|-------------|
| `scientific/ep/tasks/N` | Embarrassingly Parallel (Monte Carlo) |
| `scientific/mg/grid/N` | Multi-Grid (stencil pattern) |
| `scientific/cg/size/N` | Conjugate Gradient (sparse matrix) |

---

### `transform.rs` — Game Engine Hierarchy

Simulates game engine transform hierarchy updates.

| Benchmark | Description |
|-----------|-------------|
| `transform_hierarchy/hierarchy/d4_b4` | Depth 4, branching 4 |
| `transform_hierarchy/hierarchy/d6_b3` | Depth 6, branching 3 |
| `transform_hierarchy/hierarchy/d8_b2` | Depth 8, branching 2 |

---

### `producer_consumer.rs` — Lock-Free Queue

Tests producer-consumer pattern with SegQueue.

| Benchmark | Description |
|-----------|-------------|
| `producer_consumer/items/10000` | 10K items |
| `producer_consumer/items/100000` | 100K items |
| `producer_consumer/items/500000` | 500K items |

---

### `allocation.rs` — Frame Allocator

Tests frame allocator allocation throughput.

| Benchmark | Description |
|-----------|-------------|
| `allocation/jobs/1000` | Allocate 1K jobs |
| `allocation/jobs/10000` | Allocate 10K jobs |
| `allocation/jobs/100000` | Allocate 100K jobs |
| `allocation/jobs/1000000` | Allocate 1M jobs |

---

### `startup.rs` — Startup Latency

Measures JobSystem initialization time.

| Benchmark | Description |
|-----------|-------------|
| `startup/config/minimal_4` | 4 fibers per worker |
| `startup/config/default_16` | 16 fibers per worker |
| `startup/config/large_64` | 64 fibers per worker |

---


## Legacy Benchmark Suite

Located in `src/benchmarks/`, run via the `benchmarks` binary.

### Available Benchmarks

| Benchmark | File | Description |
|-----------|------|-------------|
| **Empty Job Latency** | `latency.rs` | Measures scheduling latency for empty jobs at various batch sizes |
| **Throughput** | `throughput.rs` | Measures jobs/second throughput |
| **Producer-Consumer** | `producer_consumer.rs` | Tests synchronized producer-consumer patterns |
| **Work-Stealing Stress** | `stress.rs` | Stresses work-stealing with steal success/failure tracking |
| **Transform Hierarchy** | `transform.rs` | Simulates game engine transform hierarchy updates |
| **NAS Benchmarks** | `nas_benchmarks.rs` | Scientific computing patterns (CG, MG, EP) |
| **QuickSort** | `quicksort.rs` | Parallel divide-and-conquer sorting |
| **Allocation** | `allocation.rs` | Tests frame allocator performance |
| **Startup Latency** | `startup_latency.rs` | Measures JobSystem initialization time |

### Running the Legacy Suite

```bash
# Build release binary
cargo build --release --bin benchmarks

# Run all benchmarks
./target/release/benchmarks

# Run specific benchmark
./target/release/benchmarks --only "Empty Job Latency"

# Run with custom thread count
./target/release/benchmarks --threads 8

# Compare across core counts
./target/release/benchmarks --compare-cores
```

### Visualization

```bash
# Generate graphs from benchmark results
python scripts/run_benchmarks.py

# Graphs are saved to docs/benchmarks/
```

---

## Benchmark Architecture

### Criterion vs Custom Runner

| Feature | Criterion (`benches/`) | Custom (`src/benchmarks/`) |
|---------|------------------------|----------------------------|
| Statistical rigor | ✅ Automatic | Manual |
| Nanosecond precision | ✅ Yes | Millisecond |
| Regression tracking | ✅ Built-in | Manual comparison |
| Visualization | HTML reports | Python matplotlib |
| Scaling sweeps | Manual | ✅ Automatic |
| Real workloads | Micro only | ✅ Application patterns |

### When to Use Each

- **Criterion**: For micro-benchmarking specific operations (context switch, job spawn)
- **Custom Runner**: For application-level patterns (game loops, scientific computing)

---

## Interpreting Results

### Latency Benchmarks

- **< 100 ns**: Excellent raw performance
- **100 ns – 1 µs**: Good, includes some overhead
- **> 1 µs**: May indicate parking, contention, or allocation issues

### Throughput Benchmarks

- **> 10 M jobs/sec**: Excellent scaling
- **1 – 10 M jobs/sec**: Good for heavier workloads
- **< 1 M jobs/sec**: Check for bottlenecks

### Scaling Analysis

- **Linear scaling**: Efficiency = Speedup / Threads ≈ 1.0
- **Sub-linear**: Work-stealing overhead or contention
- **Flat/inverse**: Bottleneck identified

---

## Adding New Benchmarks

### Criterion Benchmark

1. Create `benches/my_benchmark.rs`
2. Add to `Cargo.toml`:
   ```toml
   [[bench]]
   name = "my_benchmark"
   harness = false
   ```
3. Use criterion macros:
   ```rust
   use criterion::{criterion_group, criterion_main, Criterion};
   
   fn my_bench(c: &mut Criterion) {
       c.bench_function("my_test", |b| {
           b.iter(|| { /* code */ })
       });
   }
   
   criterion_group!(benches, my_bench);
   criterion_main!(benches);
   ```

### Custom Benchmark

1. Create `src/benchmarks/my_benchmark.rs`
2. Add module to `src/benchmarks/mod.rs`
3. Register in `src/benchmarks/main.rs`
4. Return `BenchmarkResult` with data points

---

## Performance Targets

| Metric | Target | Rationale |
|--------|--------|-----------|
| Raw fiber switch | < 50 ns | Game engine frame budget |
| Job dispatch | < 1 µs | Sub-frame latency |
| 1M job throughput | > 10 M/sec | Entity update capacity |
| Memory per fiber | < 256 KB | Reasonable pool sizes |
| Startup time | < 100 ms | Acceptable cold start |

---

## Credits

Created by Nguyen Phi Hung.
