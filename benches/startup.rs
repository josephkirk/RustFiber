//! Startup latency benchmark using criterion.
//!
//! Measures JobSystem initialization time with different configurations.
//!
//! Created by Nguyen Phi Hung.

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use rustfiber::JobSystem;
use rustfiber::job_system::FiberConfig;

fn bench_startup(c: &mut Criterion) {
    let num_threads = num_cpus::get();

    let mut group = c.benchmark_group("startup");
    group.sample_size(10);

    let configs = [
        (
            "minimal_4",
            FiberConfig {
                initial_pool_size: 4,
                ..Default::default()
            },
        ),
        ("default_16", FiberConfig::default()),
        (
            "large_64",
            FiberConfig {
                initial_pool_size: 64,
                ..Default::default()
            },
        ),
    ];

    for (name, config) in configs {
        group.bench_function(BenchmarkId::new("config", name), |b| {
            b.iter(|| {
                let system = JobSystem::new_with_config(num_threads, config.clone());

                // Do minimal work to ensure system is operational
                let counter = system.run(|| {
                    std::hint::black_box(());
                });
                system.wait_for_counter(&counter);

                // Shutdown
                let _ = system.shutdown();

                std::hint::black_box(());
            })
        });
    }

    group.finish();
}

criterion_group!(benches, bench_startup);
criterion_main!(benches);
