# Design: Fiber Switch Latency Benchmark

## Approach

Instead of adding a new `execute_raw_switch()` method, use the existing minimal-overhead path through `JobSystem`:

```rust
let counter = job_system.run(|| { /* empty */ });
job_system.wait_for_counter(&counter);
```

This measures the complete round-trip:
1. Job dispatch to injector
2. Worker steals job
3. Fiber switch (Caller → Fiber)
4. Execute empty closure
5. Complete and signal Counter
6. Wake main thread

## Benchmark Implementation

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rustfiber::JobSystem;

fn bench_fiber_switch(c: &mut Criterion) {
    // Single thread to isolate fiber switching from work-stealing overhead
    let system = JobSystem::new(1);
    
    // Warmup: ensure fiber pool is primed
    for _ in 0..1000 {
        let counter = system.run(|| {});
        system.wait_for_counter(&counter);
    }
    
    c.bench_function("fiber_switch_latency", |b| {
        b.iter(|| {
            let counter = system.run(black_box(|| {}));
            system.wait_for_counter(&counter);
        })
    });
}

criterion_group!(benches, bench_fiber_switch);
criterion_main!(benches);
```

## What This Measures

| Component | Included |
|-----------|----------|
| Job dispatch to injector | ✓ |
| Work stealing (local pop) | ✓ |
| Fiber pool acquire | ✓ |
| Context switch (corosensei) | ✓ |
| Closure execution | ✓ (empty) |
| Counter signaling | ✓ |
| Wait wakeup | ✓ |

## Expected Results

Based on `corosensei` benchmarks (~5ns raw switch) plus overhead:

| Scenario | Expected Latency |
|----------|------------------|
| Optimal (warm cache, pooled fiber) | 50–200 ns |
| Acceptable for full round-trip | 200–500 ns |
| Needs investigation | >1 μs |

> **Note**: This measures more than raw context switch - it includes job dispatch and counter signaling. For pure switch cost, would need direct fiber benchmarking (not part of this proposal).
