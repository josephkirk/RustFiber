# Performance Analysis: Job Batching (`parallel_for`)

## 1. Observations
Based on the benchmark results (`comparison_cores_batching_parallel_for_auto.png` and `comparison_strategies_batching_parallel_for_auto.png`):

*   **Scaling Plateau**: Performance scaling stops or diminishes significantly after **8-16 cores**.
    *   8 Cores: ~9.9 ms
    *   16 Cores: ~9.9 ms
    *   32 Cores: ~10-16 ms (Regression in some strategies)
*   **Throughput Cap**: Max throughput caps around ~100M items/sec.
*   **Strategy Impact**: `TieredSpillover` performs significantly worse (~60M items/sec), likely due to cross-CCD communication usage matching the memory contention pattern.

## 2. Root Cause Analysis
The bottleneck is **False Sharing and Atomic Contention**, not job scheduling overhead.

### Deep Dive: Why is 1 Core faster than 16 Cores?
The benchmark shows "Inverse Scaling" (1 Core: ~6.5ms vs 16 Cores: ~12ms). This is a classic symptom of **Cache Line Ping-Pong** (MESI Protocol Contention).

1.  **Single Core**: The `AtomicUsize` resides in the L1 cache of Core 0. `fetch_add` operations are extremely fast (~1-3 cycles) as the line is always in "Modified" state.
2.  **Multi-Core (16 Cores)**: The same `AtomicUsize` is shared.
    *   Core A wants to write (`fetch_add`). It must invalidate copies in Cores B-P and acquire "Exclusive" ownership.
    *   Core B wants to write. It must steal the line from Core A.
    *   This transfer of ownership via the L3/Interconnect takes ~100-300 cycles.
    *   Result: **Latency increases by 100x per operation**, creating a massive serial bottleneck that completely swamps the parallel speedup.

```rust
// Current Benchmark Kernel
let counter = job_system.parallel_for_auto(0..num_items, move |_| {
    std::hint::black_box(heavy_work(15));
    data.fetch_add(1, Ordering::Relaxed); // <--- CRITICAL BOTTLENECK: 1M global atomic writes
});
```

*   **Cache Line Bouncing**: Every single iteration (1 million times) performs an atomic RMW on the *exact same* memory address (`data`).
*   **Bus Saturation**: As core count increases, the cache coherency protocol is overwhelmed.
*   **Serialized Execution**: Effectively, the global `fetch_add` serializes execution.

## 3. Optimization Strategies

### A. Immediate Benchmark Optimization: Batch-Local Accumulation
To demonstrate the true throughput of the `parallel_for` *mechanism* (job dispatch), we must decouple it from the shared memory bottleneck.

**Proposed Change:**
Accumulate results locally within the loop and update the global atomic only once per batch.

```rust
// Optimized Benchmark Kernel (Conceptual)
// Note: parallel_for currently only supports Fn(usize), so we need inner loops or a new API.
// Manually simulated in benchmark:
job_system.parallel_for_chunks(0..num_items, batch_size, move |chunk_range| {
    let mut local_sum = 0;
    for i in chunk_range {
        local_sum += heavy_work(15);
    }
    data.fetch_add(local_sum, Ordering::Relaxed);
});
```

### B. Library Feature: `parallel_reduce`
Implement a dedicated reduction API in `JobSystem` to handle this pattern efficiently and safely.

*   **API**: `parallel_reduce<T>(range, identity, op, reducer) -> T`
*   **Implementation**: 
    1.  Each job computes a local reduction `T`.
    2.  Jobs return their local result (or store in a side-vector).
    3.  A final serial reduction combines these results.
*   **Benefit**: Reduces 1,000,000 atomic writes to `num_batches` (e.g., 128) writes.

### C. Data Partitioning
For non-reduction workloads (e.g., writing to an array), ensuring that writes are to distinct, cache-aligned memory regions is crucial. The current benchmark uses a shared counter, which is the worst case.

## 4. Action Plan
1.  **Refactor Benchmark**: Modify `batching_benchmark.rs` to use a "local sum" pattern manually within the closure (to the extent possible with current API) or prove the bottleneck by removing the atomic.
2.  **Verify**: Confirm that removing the atomic write per iteration restores linear scaling.
3.  **Proposal**: Draft an OpenSpec for `parallel_reduce` implementation.
