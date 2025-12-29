//! Allocation throughput benchmark using criterion.
//!
//! Tests frame allocator allocation performance.
//!
//! Created by Nguyen Phi Hung.

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use rustfiber::{Job, JobSystem};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

fn bench_allocation(c: &mut Criterion) {
    let num_threads = num_cpus::get();
    let system = JobSystem::new(num_threads);

    let mut group = c.benchmark_group("allocation");
    group.sample_size(10);

    for count in [1_000, 10_000, 100_000, 1_000_000] {
        group.throughput(Throughput::Elements(count as u64));

        group.bench_function(BenchmarkId::new("jobs", count), |b| {
            b.iter(|| {
                let inner_nanos = Arc::new(AtomicU64::new(0));
                let nanos_clone = inner_nanos.clone();

                let root = system.run_with_context(move |_ctx| {
                    let mut jobs = Vec::with_capacity(count);
                    let t0 = Instant::now();

                    for _ in 0..count {
                        // Trigger FrameAllocator via Job::new
                        jobs.push(Job::new(|| {
                            std::hint::black_box(());
                        }));
                    }

                    let t1 = t0.elapsed();
                    nanos_clone.store(t1.as_nanos() as u64, Ordering::SeqCst);

                    std::hint::black_box(&jobs);
                });

                system.wait_for_counter(&root);

                std::hint::black_box(inner_nanos.load(Ordering::SeqCst));
            })
        });
    }

    group.finish();
}

criterion_group!(benches, bench_allocation);
criterion_main!(benches);
