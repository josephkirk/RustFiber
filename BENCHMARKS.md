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
5. **Allocation Throughput** - Baseline test of the `FrameAllocator` and `Job::new` logic, measuring raw per-task allocation cost (~10ns/job).
6. **Startup Latency** - Measures job system initialization time across different fiber configurations, validating the incremental allocation optimization that reduced startup from 4-5ms to <1ms.

## Running Benchmarks

### With Python (uv recommended)

The benchmark runner supports selecting a **Pinning Strategy** and automatically iterates through common core counts (**1, 4, 16, 32, 64, 96**), skipping any that exceed the system's capacity.

```bash
# Using uv (recommended)
uv run run_benchmarks.py [strategy]

# Examples:
uv run run_benchmarks.py linear          # Worker i -> Core i (Default)
uv run run_benchmarks.py --compare-cores # Compare Linear across core counts
uv run run_benchmarks.py --compare-strategies # Compare strategies on high cores (>=16)
uv run run_benchmarks.py avoid-smt      # Physical cores only
```

### Modes

1.  **Single Run**: `uv run run_benchmarks.py [strategy] [thread_count]`
    - Generates a standard graph for a specific configuration.
2.  **Core Comparison**: `uv run run_benchmarks.py --compare-cores`
    - Uses the `Linear` strategy.
    - Compares 1, 4, 16, 32, 64, 96 cores (skipping those not supported).
    - Generates `comparison_cores_[NAME].png` with a line for each core count.
3.  **Strategy Comparison**: `uv run run_benchmarks.py --compare-strategies`
    - Runs only on systems with 16+ cores.
    - Uses the maximum physical core count.
    - Compares `none`, `linear`, `avoid-smt`, `ccd-isolation`, `tiered-spillover`.
    - Generates `comparison_strategies_[NAME].png` with a line for each strategy.

The script will:
1. Build and run the Rust benchmark binary for each target core count.
2. **Core Safety**: Only runs core counts supported by the current hardware.
3. **Stream results**: JSON data is processed line-by-line as benchmarks complete.
4. **Incremental Graphing**: PNG graphs are generated immediately after each test.
5. **Timeout**: Each benchmark has a strict **1-minute timeout** to prevent hangs.

### Manual Execution

You can also run the Rust benchmarks directly for a specific strategy and thread count:

```bash
cargo run --bin benchmarks --release -- [strategy] [thread_count]
```

## Output

The benchmarks generate PNG graphs in `docs/` with descriptive filenames:
Format: `benchmark_[NAME]_[CORES]c_[RAM]gb_[STRATEGY].png`
Comparison Format: `comparison_[MODE]_[NAME].png`

## Comparison Results

### Core Scaling Comparison (Linear Strategy)
These graphs show how performance scales from 1 to 32 cores.

| Fibonacci | QuickSort |
|-----------|-----------|
| ![Fibonacci Cores](docs/comparison_cores_benchmark_1_million_tiny_tasks_fibonacci.png) | ![QuickSort Cores](docs/comparison_cores_benchmark_2_recursive_task_decomposition_quicksort.png) |

| Producer-Consumer | NAS EP |
|-------------------|--------|
| ![Producer-Consumer Cores](docs/comparison_cores_benchmark_3_producer-consumer_lock-free.png) | ![NAS EP Cores](docs/comparison_cores_benchmark_4a_nas_ep_embarrassingly_parallel.png) |

| NAS MG | NAS CG |
|--------|--------|
| ![NAS MG Cores](docs/comparison_cores_benchmark_4b_nas_mg_multi-grid.png) | ![NAS CG Cores](docs/comparison_cores_benchmark_4c_nas_cg_conjugate_gradient.png) |

| Batching (Parallel For) | Allocation Throughput |
|-------------------------|-----------------------|
| ![Batching Cores](docs/comparison_cores_batching_parallel_for_auto.png) | ![Allocation Cores](docs/comparison_cores_allocation_throughput.png) |

### Strategy Efficiency Comparison (32 Cores)
These graphs compare different pinning strategies on a multi-core system.

| Fibonacci | QuickSort |
|-----------|-----------|
| ![Fibonacci Strategies](docs/comparison_strategies_benchmark_1_million_tiny_tasks_fibonacci.png) | ![QuickSort Strategies](docs/comparison_strategies_benchmark_2_recursive_task_decomposition_quicksort.png) |

| Producer-Consumer | NAS EP |
|-------------------|--------|
| ![Producer-Consumer Strategies](docs/comparison_strategies_benchmark_3_producer-consumer_lock-free.png) | ![NAS EP Strategies](docs/comparison_strategies_benchmark_4a_nas_ep_embarrassingly_parallel.png) |

| NAS MG | NAS CG |
|--------|--------|
| ![NAS MG Strategies](docs/comparison_strategies_benchmark_4b_nas_mg_multi-grid.png) | ![NAS CG Strategies](docs/comparison_strategies_benchmark_4c_nas_cg_conjugate_gradient.png) |

| Batching (Parallel For) | Allocation Throughput |
|-------------------------|-----------------------|
| ![Batching Strategies](docs/comparison_strategies_batching_parallel_for_auto.png) | ![Allocation Strategies](docs/comparison_strategies_allocation_throughput.png) |

## Scaling Analysis (ns/task)
RustFiber now uses **Nanoseconds per Task/Item** as its primary performance metric for most benchmarks. 

*   **O(1) Visualization**: A perfectly horizontal line on these graphs indicates that the work system has zero scaling overhead—the cost of a millionth task is identical to the first.
*   **Zero-Based Y-Axis**: All scaling graphs are forced to start at `0ns`. This provides a non-deceptive view of whether the system overhead is small relative to the work.
*   **Amortized Overhead**: Small task sizes (1,000 and 5,000) are excluded from the results. At these scales, constant overheads like root job submission dominate the metric, creating $1/n$ noise that obscures the actual runtime efficiency.

## Batching (Parallel For) Performance
The "Batching" benchmark tests the `parallel_for_chunked` API.
*   **Without Batching**: 1 million tiny jobs overwhelm the scheduler (~100M instructions).
*   **With Batching (Chunked)**: Work is split into ~128 large chunks (4x core count).
*   **Optimization**: Critically, it allows **Batch-Local Accumulation**, reducing atomic synchronization from 1,000,000 ops to just ~128 ops.
*   **Result**: Linear scaling up to memory bandwidth limits (>1B items/sec effective throughput).

## Warmup & Cold-Start Prevention
To ensure measurements reflect the high-frequency steady state of the job system, the following optimizations are applied:

- **OS Settle Period**: Each benchmark waits **20ms** after initializing the `JobSystem`. This allows worker threads, core affinity masks, and CPU frequency scaling to stabilize.
- **Substantial Warmup**: A massive parallel task (e.g., 100,000 items) is executed before recording. This primes the `FrameAllocator` arenas and wakes up all worker fibers.
- **Worker-Local Measurement**: Benchmarks like "Allocation Throughput" measure the **inner worker loop duration** directly via atomics, ensuring high-level scheduling overhead does not skew the raw per-task performance data.

## Startup Latency Benchmark
The startup latency benchmark specifically measures the time required to initialize the job system with different fiber configurations:
- **Default Configuration**: Standard fiber pool size for balanced performance
- **Minimal Configuration**: Reduced pool size for fastest possible startup
- **Large Configuration**: Increased pool size for high-throughput workloads

This benchmark validates the incremental fiber allocation optimization that reduced startup time from 4-5ms to <1ms, ensuring no runtime allocation glitches while maintaining fast initialization.

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
