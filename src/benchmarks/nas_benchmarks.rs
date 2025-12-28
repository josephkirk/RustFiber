use crate::utils::{BenchmarkResult, DataPoint, SystemInfo};
use rustfiber::{JobSystem, PinningStrategy};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

// Embarrassingly Parallel (EP) - Pure throughput test with zero communication
fn run_ep_benchmark(job_system: &JobSystem, num_tasks: usize) -> f64 {
    let counter = Arc::new(AtomicUsize::new(0));
    let start = Instant::now();
    let counter_clone = counter.clone();

    let root_job = job_system.run_with_context(move |ctx| {
        for _ in 0..num_tasks {
            let c_inner = counter_clone.clone();
            ctx.spawn_detached(move |_| {
                // Independent computation - no communication
                let mut sum = 0.0;
                for i in 0..1000 {
                    sum += (i as f64).sqrt();
                }
                c_inner.fetch_add(1, Ordering::SeqCst);
                let _ = sum; // Use result
            });
        }
    });

    job_system.wait_for_counter(&root_job);

    while counter.load(Ordering::SeqCst) < num_tasks {
        std::thread::yield_now();
    }

    start.elapsed().as_secs_f64() * 1000.0
}

// Multi-Grid (MG) - Tests communication and memory bandwidth
fn run_mg_benchmark(job_system: &JobSystem, grid_size: usize, threads: usize) -> f64 {
    // Use separate grids for each chunk to eliminate mutex contention
    let num_chunks = threads;
    let rows_per_chunk = grid_size / num_chunks;

    let start = Instant::now();

    // Create independent jobs, each with its own grid
    let mut jobs: Vec<Box<dyn FnOnce() + Send>> = Vec::new();
    for _ in 0..num_chunks {
        // Each job gets its own grid - no sharing, no mutexes
        let mut local_grid = vec![vec![0.0; grid_size]; rows_per_chunk + 2];

        jobs.push(Box::new(move || {
            // Simulate stencil operations on local grid
            for _ in 0..10 {
                let local_size = local_grid.len();
                for i in 1..local_size - 1 {
                    for j in 1..grid_size - 1 {
                        let val = (local_grid[i - 1][j]
                            + local_grid[i + 1][j]
                            + local_grid[i][j - 1]
                            + local_grid[i][j + 1])
                            / 4.0;
                        local_grid[i][j] = val + 0.1;
                    }
                }
            }
            // Prevent optimization of unused result
            std::hint::black_box(local_grid);
        }));
    }

    let counter = job_system.run_multiple(jobs);
    job_system.wait_for_counter(&counter);

    start.elapsed().as_secs_f64() * 1000.0
}

// Conjugate Gradient (CG) - Tests irregular memory access
fn run_cg_benchmark(job_system: &JobSystem, matrix_size: usize, threads: usize) -> f64 {
    let start = Instant::now();

    // Simulate sparse matrix-vector multiplication with irregular access
    let num_chunks = threads;
    let chunk_size = matrix_size / num_chunks;

    let mut jobs: Vec<Box<dyn FnOnce() + Send>> = Vec::new();
    for chunk in 0..num_chunks {
        // Each job gets its own copy of input and output - no sharing
        let input_vec = vec![1.0; matrix_size];
        let mut output_vec = vec![0.0; chunk_size];

        jobs.push(Box::new(move || {
            let start_idx = chunk * chunk_size;
            let end_idx = ((chunk + 1) * chunk_size).min(matrix_size);

            // Irregular memory access pattern
            for (local_i, i) in (start_idx..end_idx).enumerate() {
                let mut sum = 0.0;
                // Simulate sparse matrix access
                for j in (i.saturating_sub(5)..=(i + 5).min(matrix_size - 1)).step_by(2) {
                    sum += input_vec[j] * ((i + j) as f64 * 0.01);
                }
                output_vec[local_i] = sum;
            }
            // Prevent optimization of unused result
            std::hint::black_box(output_vec);
        }));
    }

    let counter = job_system.run_multiple(jobs);
    job_system.wait_for_counter(&counter);

    start.elapsed().as_secs_f64() * 1000.0
}

pub fn run_nas_ep_benchmark(
    job_system: &JobSystem,
    strategy: PinningStrategy,
    threads: usize,
) -> BenchmarkResult {
    eprintln!("\n=== Benchmark 4a: NAS EP (Embarrassingly Parallel) ===");

    let system_info = SystemInfo::collect(strategy, threads);
    eprintln!(
        "System: {} CPU cores, {:.2} GB total RAM, Strategy: {:?}",
        system_info.cpu_cores, system_info.total_memory_gb, strategy
    );

    let ep_sizes = vec![
        1, 10, 100, 1_000, 10_000, 25_000, 50_000, 100_000, 200_000, 300_000, 500_000, 1_000_000,
    ];
    let mut data_points = Vec::new();
    let mut timed_out = false;
    let total_start = Instant::now();
    let timeout_duration = std::time::Duration::from_secs(crate::utils::DEFAULT_TIMEOUT_SECS);

    for &size in &ep_sizes {
        if total_start.elapsed() > timeout_duration {
            eprintln!(
                "\n! Timeout reached ({}s), stopping benchmark.",
                crate::utils::DEFAULT_TIMEOUT_SECS
            );
            timed_out = true;
            break;
        }
        eprintln!("Testing EP with {} tasks...", size);
        let elapsed_ms = run_ep_benchmark(job_system, size);
        eprintln!(
            "  Completed in {:.2} ms ({:.2} ns/item)",
            elapsed_ms,
            (elapsed_ms * 1_000_000.0) / (size as f64)
        );
        data_points.push(DataPoint {
            num_tasks: size,
            time_ms: elapsed_ms,
            metric_type: None,
        });
    }

    BenchmarkResult {
        name: "Benchmark 4a: NAS EP (Embarrassingly Parallel)".to_string(),
        data_points,
        system_info,
        crashed: false,
        crash_point: None,
        timed_out,
        #[cfg(feature = "metrics")]
        internal_metrics: crate::utils::capture_internal_metrics(job_system),
    }
}

pub fn run_nas_mg_benchmark(
    job_system: &JobSystem,
    strategy: PinningStrategy,
    threads: usize,
) -> BenchmarkResult {
    eprintln!("\n=== Benchmark 4b: NAS MG (Multi-Grid) ===");

    let system_info = SystemInfo::collect(strategy, threads);
    eprintln!(
        "System: {} CPU cores, {:.2} GB total RAM, Strategy: {:?}",
        system_info.cpu_cores, system_info.total_memory_gb, strategy
    );

    let mg_sizes = vec![16, 32, 64, 128, 256, 512, 1024];
    let mut data_points = Vec::new();
    let mut timed_out = false;
    let total_start = Instant::now();
    let timeout_duration = std::time::Duration::from_secs(crate::utils::DEFAULT_TIMEOUT_SECS);

    for &size in &mg_sizes {
        if total_start.elapsed() > timeout_duration {
            eprintln!(
                "\n! Timeout reached ({}s), stopping benchmark.",
                crate::utils::DEFAULT_TIMEOUT_SECS
            );
            timed_out = true;
            break;
        }
        eprintln!("Testing MG with {}x{} grid...", size, size);
        let elapsed_ms = run_mg_benchmark(job_system, size, threads);
        eprintln!("  Completed in {:.2} ms", elapsed_ms);
        data_points.push(DataPoint {
            num_tasks: size,
            time_ms: elapsed_ms,
            metric_type: None,
        });
    }

    BenchmarkResult {
        name: "Benchmark 4b: NAS MG (Multi-Grid)".to_string(),
        data_points,
        system_info,
        crashed: false,
        crash_point: None,
        timed_out,
        #[cfg(feature = "metrics")]
        internal_metrics: crate::utils::capture_internal_metrics(job_system),
    }
}

pub fn run_nas_cg_benchmark(
    job_system: &JobSystem,
    strategy: PinningStrategy,
    threads: usize,
) -> BenchmarkResult {
    eprintln!("\n=== Benchmark 4c: NAS CG (Conjugate Gradient) ===");

    let system_info = SystemInfo::collect(strategy, threads);
    eprintln!(
        "System: {} CPU cores, {:.2} GB total RAM, Strategy: {:?}",
        system_info.cpu_cores, system_info.total_memory_gb, strategy
    );

    let cg_sizes = vec![1_020_000, 10_050, 250_001, 50_000, 100_000];
    let mut data_points = Vec::new();
    let mut timed_out = false;
    let total_start = Instant::now();
    let timeout_duration = std::time::Duration::from_secs(crate::utils::DEFAULT_TIMEOUT_SECS);

    for &size in &cg_sizes {
        if total_start.elapsed() > timeout_duration {
            eprintln!(
                "\n! Timeout reached ({}s), stopping benchmark.",
                crate::utils::DEFAULT_TIMEOUT_SECS
            );
            timed_out = true;
            break;
        }
        eprintln!("Testing CG with matrix size {}...", size);
        let elapsed_ms = run_cg_benchmark(job_system, size, threads);
        eprintln!("  Completed in {:.2} ms", elapsed_ms);
        data_points.push(DataPoint {
            num_tasks: size,
            time_ms: elapsed_ms,
            metric_type: None,
        });
    }

    BenchmarkResult {
        name: "Benchmark 4c: NAS CG (Conjugate Gradient)".to_string(),
        data_points,
        system_info,
        crashed: false,
        crash_point: None,
        timed_out,
        #[cfg(feature = "metrics")]
        internal_metrics: crate::utils::capture_internal_metrics(job_system),
    }
}
