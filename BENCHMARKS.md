# RustFiber Benchmarks

This directory contains benchmark scripts that test the RustFiber job system with various stress tests, now optimized with thread pinning strategies.

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

The benchmark runner supports selecting a **Pinning Strategy** to optimize for different CPU architectures (e.g., AMD Ryzen CCD isolation).

```bash
# Using uv (recommended)
uv run run_benchmarks.py [strategy]

# Examples:
uv run run_benchmarks.py linear         # Worker i -> Core i (Default)
uv run run_benchmarks.py avoid-smt     # Worker i -> Physical Core i (Even logic processors)
uv run run_benchmarks.py ccd-isolation # Restricted to CCD1 (Logical 0-14, step 2)
uv run run_benchmarks.py none          # Standard OS scheduling
```

The script will:
1. Build and run the Rust benchmark binary with the selected strategy.
2. **Stream results**: JSON data is processed line-by-line as benchmarks complete.
3. **Incremental Graphing**: PNG graphs are generated immediately after each test.
4. **Timeout**: Each benchmark has a strict **1-minute timeout** to prevent hangs.

### Manual Execution

You can also run the Rust benchmarks directly:

```bash
cargo run --bin benchmarks --release -- [strategy]
```

## Output

The benchmarks generate PNG graphs in `docs/` with descriptive filenames:
Format: `benchmark_[NAME]_[CORES]c_[RAM]gb_[STRATEGY].png`

Example: `benchmark_1_fibonacci_32c_95gb_ccdisolation.png`

Each graph shows:
- X-axis: Number of tasks
- Y-axis: Execution time in milliseconds
- **Title**: Benchmark Name + Selected Pinning Strategy
- **Info Box**: CPU cores, RAM, and pinning strategy details
- **Status Overlay**: "Benchmark timed out" or "System crashed" if applicable

## Requirements

- Rust (for building the benchmark binary)
- Python 3.8+ (for graph generation)
- matplotlib (Python package)
- sysinfo (Rust crate, for hardware detection)

Install Python dependencies:

```bash
# With uv
uv pip install matplotlib

# Or with pip
pip install matplotlib
```
