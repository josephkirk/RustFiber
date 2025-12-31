# Startup Latency Analysis

## Observation
Linear benchmarks show a consistent ~4-5ms startup latency before tasks begin execution.

## Root Cause
The latency is caused by the **Eager Pre-allocation** of fiber stacks in `FiberPool::new()`.
- **Default Config**: `initial_pool_size = 128`, `stack_size = 512KB`.
- **Per-Thread Cost**: 128 * 512KB = **64MB** of virtual memory allocated and initialized (vectors created, potential mmap/fault overhead).
- **System Cost (16 Threads)**: 16 * 64MB = **1GB** total virtual memory reserve.

This initialization happens serially inside each `Worker` thread upon startup. The 4-5ms is the time taken to allocate these OS resources.

## Implication
- **For Games/Servers**: This is **Beneficial**. Pre-allocating fibers prevents unpredictable allocation spikes ("glitches") during the first few frames of gameplay. 5ms startup time is negligible for a long-running process.
- **For CLI/Benchmarks**: This appears as "high overhead" for execution, but is actually just initialization.

## Recommendations
1.  **Keep as Default**: For the target use-case (Game Engine), pre-allocation is correct.
2.  **For Fast Startup**: If using `RustFiber` for short-lived CLI tools, reduce the initial pool size:
    ```rust
    let config = FiberConfig {
        initial_pool_size: 16, // Reduce from 128
        ..FiberConfig::default()
    };
    let system = JobSystem::new_with_config(threads, config);
    ```
    This will reduce startup latency to <1ms, at the cost of potential allocations during the first complex frame.

## Conclusion
The 4-5ms is not runtime locking or contention "overhead". It is a fixed initialization cost that amortizes to zero for any run longer than a few frames.
