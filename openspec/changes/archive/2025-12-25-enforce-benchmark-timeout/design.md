# Design: Benchmark Timeout and Incremental Graph Generation

## Architecture Reasoning
The current implementation uses a "wait-all" pattern where the Rust binary collects all results in a `Vec<BenchmarkResult>` and then serializes it at the end. The Python script then reads the entire output at once.

To achieve incremental feedback, we will move to a "stream" pattern:
1. **Rust**: Instead of building a `Vec`, each benchmark will be executed and its result will be immediately printed to `stdout` as a JSON line.
2. **Python**: The Python script will use `subprocess.Popen` with a pipe for `stdout` and iterate over the lines.

## Timeout Mechanism
Each benchmark iterates through a list of `test_sizes`. We will add an `Instant` at the start of each benchmark and check the elapsed time after each data point is collected. If it exceeds 60 seconds (configurable via flag), we stop the iteration for that benchmark.

## Incremental Graphing
The Python script will maintain its logic for graph generation but will trigger it per line of JSON received from the Rust binary. This ensures that even if later benchmarks take longer, earlier ones are already visualized.
