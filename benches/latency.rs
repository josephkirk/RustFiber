//! Scheduling latency benchmark using criterion.
//!
//! Measures per-job scheduling latency at various batch sizes.
//!
//! Created by Nguyen Phi Hung.

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use rustfiber::{Counter, JobSystem};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

fn bench_scheduling_latency(c: &mut Criterion) {
    let num_threads = num_cpus::get();
    let system = JobSystem::new(num_threads);

    // Warmup
    let warmup = system.parallel_for_chunked_auto(0..num_threads * 100, |_| {
        std::hint::black_box(());
    });
    system.wait_for_counter(&warmup);

    let mut group = c.benchmark_group("scheduling_latency");

    for batch_size in [100, 1_000, 10_000, 100_000] {
        group.throughput(Throughput::Elements(batch_size as u64));

        group.bench_function(BenchmarkId::new("batch", batch_size), |b| {
            b.iter(|| {
                let job_latencies = Arc::new(AtomicU64::new(0));
                let jobs_completed = Arc::new(AtomicU64::new(0));
                let dispatch_start = std::time::Instant::now();

                let batch_counter = Counter::new(batch_size);
                let batch_counter_clone = batch_counter.clone();
                let latencies = job_latencies.clone();
                let completed = jobs_completed.clone();

                let root = system.run_with_context(move |ctx| {
                    for _ in 0..batch_size {
                        let lat = latencies.clone();
                        let comp = completed.clone();
                        let cnt = batch_counter_clone.clone();
                        let start = dispatch_start;

                        ctx.spawn_with_counter(
                            move |_| {
                                let latency_ns = start.elapsed().as_nanos() as u64;
                                lat.fetch_add(latency_ns, Ordering::Relaxed);
                                comp.fetch_add(1, Ordering::Relaxed);
                                std::hint::black_box(());
                            },
                            cnt,
                        );
                    }
                });

                system.wait_for_counter(&root);
                system.wait_for_counter(&batch_counter);

                // Return average latency for criterion to track
                let total = job_latencies.load(Ordering::Relaxed);
                let count = jobs_completed.load(Ordering::Relaxed);
                if count > 0 {
                    std::hint::black_box(total / count);
                }
            })
        });
    }

    group.finish();
}

criterion_group!(benches, bench_scheduling_latency);
criterion_main!(benches);
