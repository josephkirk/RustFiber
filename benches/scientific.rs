//! Scientific computing benchmarks using criterion.
//!
//! Implements NAS Parallel Benchmark patterns: EP, MG, CG.
//!
//! Created by Nguyen Phi Hung.

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use rustfiber::JobSystem;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Embarrassingly Parallel (EP) - Pure throughput with zero communication
fn bench_ep(c: &mut Criterion) {
    // let num_threads = num_cpus::get();
    let system = JobSystem::for_throughput();

    let mut group = c.benchmark_group("scientific/ep");
    group.sample_size(10);

    for num_tasks in [10_000, 100_000, 1_000_000] {
        group.throughput(Throughput::Elements(num_tasks as u64));

        group.bench_function(BenchmarkId::new("tasks", num_tasks), |b| {
            b.iter(|| {
                let counter = system.parallel_for_chunked_auto(0..num_tasks, |range| {
                    for i in range {
                        // Monte Carlo-style computation
                        let x = (i as f64 * 0.0001).sin();
                        let y = (i as f64 * 0.0001).cos();
                        std::hint::black_box(x * x + y * y);
                    }
                });
                system.wait_for_counter(&counter);
            })
        });
    }

    group.finish();
}

/// Multi-Grid (MG) - Tests communication and memory bandwidth
fn bench_mg(c: &mut Criterion) {
    // let num_threads = num_cpus::get();
    let system = JobSystem::for_throughput();

    let mut group = c.benchmark_group("scientific/mg");
    group.sample_size(10);

    for grid_size in [64, 128, 256] {
        let total_cells = grid_size * grid_size * grid_size;
        group.throughput(Throughput::Elements(total_cells as u64));

        group.bench_function(BenchmarkId::new("grid", grid_size), |b| {
            let grid = Arc::new(vec![0.0f64; total_cells]);

            b.iter(|| {
                let g = grid.clone();
                let counter = system.parallel_for_chunked_auto(0..total_cells, move |range| {
                    for i in range {
                        // Stencil-like access pattern
                        let val = g[i];
                        let left = if i > 0 { g[i - 1] } else { 0.0 };
                        let right = if i < total_cells - 1 { g[i + 1] } else { 0.0 };
                        std::hint::black_box(val + left + right);
                    }
                });
                system.wait_for_counter(&counter);
            })
        });
    }

    group.finish();
}

/// Conjugate Gradient (CG) - Tests irregular memory access
fn bench_cg(c: &mut Criterion) {
    // let num_threads = num_cpus::get();
    let system = JobSystem::for_throughput();

    let mut group = c.benchmark_group("scientific/cg");
    group.sample_size(10);

    for matrix_size in [1_000, 10_000, 100_000] {
        group.throughput(Throughput::Elements(matrix_size as u64));

        group.bench_function(BenchmarkId::new("size", matrix_size), |b| {
            let vector = Arc::new(vec![1.0f64; matrix_size]);
            let result = Arc::new(AtomicUsize::new(0));

            b.iter(|| {
                let v = vector.clone();
                let r = result.clone();

                let counter = system.parallel_for_chunked_auto(0..matrix_size, move |range| {
                    for i in range {
                        // Sparse matrix-vector multiply pattern
                        let mut sum = 0.0;
                        for j in 0..5.min(matrix_size - i) {
                            sum += v[(i + j) % matrix_size];
                        }
                        r.fetch_add(sum as usize, Ordering::Relaxed);
                        std::hint::black_box(sum);
                    }
                });
                system.wait_for_counter(&counter);

                std::hint::black_box(result.load(Ordering::Relaxed));
            })
        });
    }

    group.finish();
}

criterion_group!(benches, bench_ep, bench_mg, bench_cg);
criterion_main!(benches);
