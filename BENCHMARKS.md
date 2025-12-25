# RustFiber Benchmarks

This directory contains benchmark scripts that test the RustFiber job system with various stress tests.

## Benchmarks

1. **Million Tiny Tasks (Fibonacci)** - Tests task creation latency and scheduler efficiency with 1M+ tiny jobs
2. **Recursive Task Decomposition (QuickSort)** - Tests work-stealing efficiency with recursive job spawning
3. **Producer-Consumer Stress Test** - Tests throughput under high lock contention
4. **NAS Parallel Benchmarks** - Real-world computational efficiency tests:
   - EP (Embarrassingly Parallel) - Pure throughput with zero communication
   - MG (Multi-Grid) - Communication and memory bandwidth
   - CG (Conjugate Gradient) - Irregular memory access patterns

## Running Benchmarks

### With Python (uv recommended)

```bash
# Using uv (recommended)
uv run run_benchmarks.py

# Or with regular Python
python3 run_benchmarks.py
```

The script will:
1. Build and run the Rust benchmark binary
2. Capture JSON output with timing data
3. Generate PNG graphs in the `docs/` folder

### Manual Execution

You can also run the Rust benchmarks directly:

```bash
cargo run --bin benchmarks --release > benchmark_results.json
```

Then process the JSON output manually.

## Output

The benchmarks generate the following PNG graphs in `docs/`:

- `benchmark_1_fibonacci.png`
- `benchmark_2_quicksort.png`
- `benchmark_3_producer_consumer.png`
- `benchmark_4a_nas_ep.png`
- `benchmark_4b_nas_mg.png`
- `benchmark_4c_nas_cg.png`

Each graph shows:
- X-axis: Number of tasks
- Y-axis: Execution time in milliseconds
- System information (CPU cores, RAM)
- Crash point (if system crashed during testing)

## Requirements

- Rust (for building the benchmark binary)
- Python 3.8+ (for graph generation)
- matplotlib (Python package)

Install Python dependencies:

```bash
# With uv
uv pip install matplotlib

# Or with pip
pip install matplotlib
```
