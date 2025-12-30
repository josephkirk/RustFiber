//! Work-stealing stress benchmark using criterion.
//!
//! Tests work-stealing under high contention with imbalanced workloads.
//!
//! Created by Nguyen Phi Hung.

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use rustfiber::{Counter, JobSystem};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

fn fibonacci(n: u64) -> u64 {
    if n <= 1 {
        return n;
    }
    let mut a = 0u64;
    let mut b = 1u64;
    for _ in 2..=n {
        let temp = a.wrapping_add(b);
        a = b;
        b = temp;
    }
    b
}

fn bench_work_stealing_stress(c: &mut Criterion) {
    let num_threads = num_cpus::get();
    let system = JobSystem::for_throughput();

    // Warmup
    let warmup = system.parallel_for_chunked_auto(0..num_threads * 100, |_| {
        std::hint::black_box(());
    });
    system.wait_for_counter(&warmup);

    let mut group = c.benchmark_group("work_stealing");
    group.sample_size(10);

    // Imbalanced workload: some jobs are heavy, some are light
    for total_jobs in [1_000, 10_000, 100_000] {
        group.throughput(Throughput::Elements(total_jobs as u64));

        group.bench_function(BenchmarkId::new("imbalanced", total_jobs), |b| {
            b.iter(|| {
                let completed = Arc::new(AtomicUsize::new(0));
                let batch_counter = Counter::new(total_jobs);
                let batch_clone = batch_counter.clone();
                let comp = completed.clone();

                let root = system.run_with_context(move |ctx| {
                    for i in 0..total_jobs {
                        let c = comp.clone();
                        let cnt = batch_clone.clone();

                        ctx.spawn_with_counter(
                            move |_| {
                                // Imbalanced: every 10th job is heavy
                                let work = if i % 10 == 0 { 1000 } else { 10 };
                                std::hint::black_box(fibonacci(work));
                                c.fetch_add(1, Ordering::Relaxed);
                            },
                            cnt,
                        );
                    }
                });

                system.wait_for_counter(&root);
                system.wait_for_counter(&batch_counter);

                std::hint::black_box(completed.load(Ordering::Relaxed));
            })
        });
    }

    group.finish();
}

criterion_group!(benches, bench_work_stealing_stress);
criterion_main!(benches);
