use crate::utils::{BenchmarkResult, DataPoint, SystemInfo, num_cpus};
use rustfiber::{JobSystem, PinningStrategy};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

// Embarrassingly Parallel (EP) - Pure throughput test with zero communication
fn run_ep_benchmark(job_system: &JobSystem, num_tasks: usize) -> f64 {
    let counter = Arc::new(AtomicUsize::new(0));
    let start = Instant::now();

    let mut jobs: Vec<Box<dyn FnOnce() + Send>> = Vec::new();
    for _ in 0..num_tasks {
        let counter_clone = counter.clone();
        jobs.push(Box::new(move || {
            // Independent computation - no communication
            let mut sum = 0.0;
            for i in 0..1000 {
                sum += (i as f64).sqrt();
            }
            counter_clone.fetch_add(1, Ordering::SeqCst);
            let _ = sum; // Use result
        }));
    }

    let job_counter = job_system.run_multiple(jobs);
    job_system.wait_for_counter(&job_counter);

    start.elapsed().as_secs_f64() * 1000.0
}

// Multi-Grid (MG) - Tests communication and memory bandwidth
fn run_mg_benchmark(job_system: &JobSystem, grid_size: usize, threads: usize) -> f64 {
    // Use a vector of per-chunk grids to reduce lock contention
    let num_chunks = threads;
    let rows_per_chunk = grid_size / num_chunks;

    // Each chunk gets its own grid section to reduce lock contention
    let grids: Vec<_> = (0..num_chunks)
        .map(|_| Arc::new(Mutex::new(vec![vec![0.0; grid_size]; rows_per_chunk + 2])))
        .collect();

    let start = Instant::now();

    // Simulate multi-grid operations with reduced communication
    let mut jobs: Vec<Box<dyn FnOnce() + Send>> = Vec::new();
    for grid_clone in grids.iter().take(num_chunks) {
        let grid_clone = grid_clone.clone();
        jobs.push(Box::new(move || {
            // Simulate stencil operations on local grid section
            for _ in 0..10 {
                let mut local_grid = grid_clone.lock().unwrap();
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
        }));
    }

    let counter = job_system.run_multiple(jobs);
    job_system.wait_for_counter(&counter);

    start.elapsed().as_secs_f64() * 1000.0
}

// Conjugate Gradient (CG) - Tests irregular memory access
fn run_cg_benchmark(job_system: &JobSystem, matrix_size: usize, threads: usize) -> f64 {
    let vector = Arc::new(Mutex::new(vec![1.0; matrix_size]));
    let result = Arc::new(Mutex::new(vec![0.0; matrix_size]));

    let start = Instant::now();

    // Simulate sparse matrix-vector multiplication with irregular access
    let num_chunks = threads;
    let chunk_size = matrix_size / num_chunks;

    let mut jobs: Vec<Box<dyn FnOnce() + Send>> = Vec::new();
    for chunk in 0..num_chunks {
        let vector_clone = vector.clone();
        let result_clone = result.clone();

        jobs.push(Box::new(move || {
            let start_idx = chunk * chunk_size;
            let end_idx = ((chunk + 1) * chunk_size).min(matrix_size);

            let vec = vector_clone.lock().unwrap();
            let mut res = result_clone.lock().unwrap();

            // Irregular memory access pattern
            for i in start_idx..end_idx {
                let mut sum = 0.0;
                // Simulate sparse matrix access
                for j in (i.saturating_sub(5)..=(i + 5).min(matrix_size - 1)).step_by(2) {
                    sum += vec[j] * ((i + j) as f64 * 0.01);
                }
                res[i] = sum;
            }
        }));
    }

    let counter = job_system.run_multiple(jobs);
    job_system.wait_for_counter(&counter);

    start.elapsed().as_secs_f64() * 1000.0
}

pub fn run_nas_ep_benchmark(strategy: PinningStrategy, threads: usize) -> BenchmarkResult {
    eprintln!("\n=== Benchmark 4a: NAS EP (Embarrassingly Parallel) ===");

    let system_info = SystemInfo::collect(strategy, threads);
    eprintln!(
        "System: {} CPU cores, {:.2} GB total RAM, Strategy: {:?}",
        system_info.cpu_cores, system_info.total_memory_gb, strategy
    );

    let job_system = JobSystem::new_with_strategy(threads, strategy);

    let ep_sizes = vec![1_000, 5_000, 10_000, 25_000, 50_000, 100_000, 200_000];
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
        let elapsed_ms = run_ep_benchmark(&job_system, size);
        eprintln!("  Completed in {:.2} ms", elapsed_ms);
        data_points.push(DataPoint {
            num_tasks: size,
            time_ms: elapsed_ms,
        });
    }

    job_system.shutdown().ok();

    BenchmarkResult {
        name: "Benchmark 4a: NAS EP (Embarrassingly Parallel)".to_string(),
        data_points,
        system_info,
        crashed: false,
        crash_point: None,
        timed_out,
    }
}

pub fn run_nas_mg_benchmark(strategy: PinningStrategy, threads: usize) -> BenchmarkResult {
    eprintln!("\n=== Benchmark 4b: NAS MG (Multi-Grid) ===");

    let system_info = SystemInfo::collect(strategy, threads);
    eprintln!(
        "System: {} CPU cores, {:.2} GB total RAM, Strategy: {:?}",
        system_info.cpu_cores, system_info.total_memory_gb, strategy
    );

    // Use specific strategy
    let job_system = JobSystem::new_with_strategy(threads, strategy);

    let mg_sizes = vec![50, 100, 150, 200, 250, 300];
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
        let elapsed_ms = run_mg_benchmark(&job_system, size, threads);
        eprintln!("  Completed in {:.2} ms", elapsed_ms);
        data_points.push(DataPoint {
            num_tasks: size,
            time_ms: elapsed_ms,
        });
    }

    job_system.shutdown().ok();

    BenchmarkResult {
        name: "Benchmark 4b: NAS MG (Multi-Grid)".to_string(),
        data_points,
        system_info,
        crashed: false,
        crash_point: None,
        timed_out,
    }
}

pub fn run_nas_cg_benchmark(strategy: PinningStrategy, threads: usize) -> BenchmarkResult {
    eprintln!("\n=== Benchmark 4c: NAS CG (Conjugate Gradient) ===");

    let system_info = SystemInfo::collect(strategy, threads);
    eprintln!(
        "System: {} CPU cores, {:.2} GB total RAM, Strategy: {:?}",
        system_info.cpu_cores, system_info.total_memory_gb, strategy
    );

    let job_system = JobSystem::new_with_strategy(threads, strategy);

    let cg_sizes = vec![1_000, 5_000, 10_000, 25_000, 50_000, 100_000];
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
        let elapsed_ms = run_cg_benchmark(&job_system, size, threads);
        eprintln!("  Completed in {:.2} ms", elapsed_ms);
        data_points.push(DataPoint {
            num_tasks: size,
            time_ms: elapsed_ms,
        });
    }

    job_system.shutdown().ok();

    BenchmarkResult {
        name: "Benchmark 4c: NAS CG (Conjugate Gradient)".to_string(),
        data_points,
        system_info,
        crashed: false,
        crash_point: None,
        timed_out,
    }
}
