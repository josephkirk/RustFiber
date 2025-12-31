# Proposal: Enforce Benchmark Timeout and Incremental Graph Generation

## Goal
The goal of this change is to prevent benchmarks from running indefinitely on powerful systems and to provide real-time feedback by generating graphs incrementally as each benchmark test completes.

## Problem Description
Currently, the benchmark suite runs all tests and then outputs a single JSON object containing all results. On powerful systems, these tests can take a long time or even appear to run "forever" because they scale with the system's capabilities. Additionally, users must wait for all tests to finish before they can see any graphical output.

## Proposed Solution
1. **Benchmark Control**: Introduce a 1-minute timeout per benchmark test. If a test exceeds this duration, it will stop collecting data points and finish gracefully.
2. **Incremental Feedback**: Update the Rust benchmark runner to output results for each benchmark as soon as it completes.
3. **Real-time Graphing**: Update the Python benchmark script to read the incremental output and generate the corresponding graph immediately, rather than waiting for the entire suite to finish.

## Changes
- **Rust (src/benchmarks)**:
    - Update `utils.rs` to include timeout logic.
    - Update `main.rs` to output results line-by-line.
    - Update individual benchmark files (fibonacci, quicksort, etc.) to check for the timeout.
- **Python (run_benchmarks.py)**:
    - Refactor `run_rust_benchmarks` to handle incremental JSON output.
    - Trigger `create_graph` for each benchmark result as it's parsed.
