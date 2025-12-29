//! Throughput benchmark using criterion.
//!
//! Measures job throughput when spawning 1,000,000 tiny tasks.
//! Tests Chase-Lev work-stealing deque saturation and helping protocol.
//!
//! Created by Nguyen Phi Hung.

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use rustfiber::{Counter, JobSystem};

const JOB_COUNT: usize = 1_000_000;

/// Benchmark spawning 1M jobs with shared Counter.
fn bench_spawn_1m_jobs(c: &mut Criterion) {
    let num_threads = num_cpus::get();
    let system = JobSystem::new(num_threads);

    // Warmup
    for _ in 0..100 {
        let counter = system.run(|| {});
        system.wait_for_counter(&counter);
    }

    let mut group = c.benchmark_group("throughput");
    group.throughput(Throughput::Elements(JOB_COUNT as u64));
    group.sample_size(10); // Reduce samples since each iteration is expensive

    group.bench_function(BenchmarkId::new("spawn_1m_jobs", num_threads), |b| {
        b.iter(|| {
            // Create shared counter for all jobs
            let batch_counter = Counter::new(JOB_COUNT);
            let counter_clone = batch_counter.clone();

            // Spawn 1M jobs from a root job
            let root = system.run_with_context(move |ctx| {
                for _ in 0..JOB_COUNT {
                    let cnt = counter_clone.clone();
                    ctx.spawn_with_counter(
                        |_| {
                            // Minimal work
                            std::hint::black_box(1 + 1);
                        },
                        cnt,
                    );
                }
            });

            // Wait for root job to finish spawning
            system.wait_for_counter(&root);

            // Wait for all 1M jobs to complete (triggers helping)
            system.wait_for_counter(&batch_counter);
        })
    });

    group.finish();
}

/// Benchmark at different thread counts for scaling analysis.
fn bench_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("throughput_scaling");
    group.throughput(Throughput::Elements(JOB_COUNT as u64));
    group.sample_size(10);

    for threads in [1, 2, 4, 8, 16, 24, 32]
        .iter()
        .filter(|&&t| t <= num_cpus::get())
    {
        // Use AvoidSMT strategy to reduce contention on logical siblings
        let system = JobSystem::new_with_strategy(*threads, rustfiber::PinningStrategy::AvoidSMT);

        // Warmup
        for _ in 0..100 {
            let counter = system.run(|| {});
            system.wait_for_counter(&counter);
        }

        group.bench_function(BenchmarkId::new("spawn_1m", threads), |b| {
            b.iter(|| {
                let batch_counter = Counter::new(JOB_COUNT);
                let counter_clone = batch_counter.clone();

                let root = system.run_with_context(move |ctx| {
                    for _ in 0..JOB_COUNT {
                        let cnt = counter_clone.clone();
                        ctx.spawn_with_counter(
                            |_| {
                                std::hint::black_box(1 + 1);
                            },
                            cnt,
                        );
                    }
                });

                system.wait_for_counter(&root);
                system.wait_for_counter(&batch_counter);
            })
        });
    }

    group.finish();
}

/// Benchmark different pinning strategies.
fn bench_strategies(c: &mut Criterion) {
    let mut group = c.benchmark_group("strategy_comparison");
    group.throughput(Throughput::Elements(JOB_COUNT as u64));
    group.sample_size(10);

    let strategies = [
        rustfiber::PinningStrategy::None,
        rustfiber::PinningStrategy::Linear,
        rustfiber::PinningStrategy::AvoidSMT,
        rustfiber::PinningStrategy::TieredSpillover,
    ];

    let num_threads = num_cpus::get();

    for strategy in strategies {
        let system = JobSystem::new_with_strategy(num_threads, strategy);

        // Warmup
        for _ in 0..10 {
            let counter = system.run(|| {});
            system.wait_for_counter(&counter);
        }

        group.bench_function(BenchmarkId::new("spawn_1m", format!("{:?}", strategy)), |b| {
            b.iter(|| {
                let batch_counter = Counter::new(JOB_COUNT);
                let counter_clone = batch_counter.clone();

                let root = system.run_with_context(move |ctx| {
                    for _ in 0..JOB_COUNT {
                        let cnt = counter_clone.clone();
                        ctx.spawn_with_counter(
                            |_| {
                                std::hint::black_box(1 + 1);
                            },
                            cnt,
                        );
                    }
                });

                system.wait_for_counter(&root);
                system.wait_for_counter(&batch_counter);
            })
        });
    }

    group.finish();
}

criterion_group!(benches, bench_spawn_1m_jobs, bench_scaling, bench_strategies);
criterion_main!(benches);
