use rustfiber::JobSystem;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;
use crate::utils::{BenchmarkResult, DataPoint, SystemInfo, num_cpus};

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
fn run_mg_benchmark(job_system: &JobSystem, grid_size: usize) -> f64 {
    let grid = Arc::new(Mutex::new(vec![vec![0.0; grid_size]; grid_size]));
    let start = Instant::now();
    
    // Simulate multi-grid operations with communication between neighboring cells
    let num_chunks = num_cpus();
    let rows_per_chunk = grid_size / num_chunks;
    
    let mut jobs: Vec<Box<dyn FnOnce() + Send>> = Vec::new();
    for chunk in 0..num_chunks {
        let grid_clone = grid.clone();
        jobs.push(Box::new(move || {
            let start_row = chunk * rows_per_chunk;
            let end_row = ((chunk + 1) * rows_per_chunk).min(grid_size);
            
            // Simulate stencil operations (requires neighboring data)
            for _ in 0..10 {
                let mut local_grid = grid_clone.lock().unwrap();
                for i in start_row..end_row {
                    for j in 0..grid_size {
                        let val = if i > 0 && i < grid_size - 1 && j > 0 && j < grid_size - 1 {
                            (local_grid[i-1][j] + local_grid[i+1][j] + 
                             local_grid[i][j-1] + local_grid[i][j+1]) / 4.0
                        } else {
                            0.0
                        };
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
fn run_cg_benchmark(job_system: &JobSystem, matrix_size: usize) -> f64 {
    let vector = Arc::new(Mutex::new(vec![1.0; matrix_size]));
    let result = Arc::new(Mutex::new(vec![0.0; matrix_size]));
    
    let start = Instant::now();
    
    // Simulate sparse matrix-vector multiplication with irregular access
    let num_chunks = num_cpus();
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
                for j in (i.saturating_sub(5)..=(i+5).min(matrix_size-1)).step_by(2) {
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

pub fn run_nas_ep_benchmark() -> BenchmarkResult {
    eprintln!("\n=== Benchmark 4a: NAS EP (Embarrassingly Parallel) ===");
    
    let system_info = SystemInfo::collect();
    eprintln!("System: {} CPU cores, {:.2} GB total RAM", 
             system_info.cpu_cores, system_info.total_memory_gb);
    
    let job_system = JobSystem::new(num_cpus());
    
    let ep_sizes = vec![1_000, 5_000, 10_000, 25_000, 50_000, 100_000, 200_000];
    let mut data_points = Vec::new();
    
    for &size in &ep_sizes {
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
    }
}

pub fn run_nas_mg_benchmark() -> BenchmarkResult {
    eprintln!("\n=== Benchmark 4b: NAS MG (Multi-Grid) ===");
    
    let system_info = SystemInfo::collect();
    eprintln!("System: {} CPU cores, {:.2} GB total RAM", 
             system_info.cpu_cores, system_info.total_memory_gb);
    
    let job_system = JobSystem::new(num_cpus());
    
    let mg_sizes = vec![50, 100, 150, 200, 250, 300];
    let mut data_points = Vec::new();
    
    for &size in &mg_sizes {
        eprintln!("Testing MG with {}x{} grid...", size, size);
        let elapsed_ms = run_mg_benchmark(&job_system, size);
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
    }
}

pub fn run_nas_cg_benchmark() -> BenchmarkResult {
    eprintln!("\n=== Benchmark 4c: NAS CG (Conjugate Gradient) ===");
    
    let system_info = SystemInfo::collect();
    eprintln!("System: {} CPU cores, {:.2} GB total RAM", 
             system_info.cpu_cores, system_info.total_memory_gb);
    
    let job_system = JobSystem::new(num_cpus());
    
    let cg_sizes = vec![1_000, 5_000, 10_000, 25_000, 50_000, 100_000];
    let mut data_points = Vec::new();
    
    for &size in &cg_sizes {
        eprintln!("Testing CG with matrix size {}...", size);
        let elapsed_ms = run_cg_benchmark(&job_system, size);
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
    }
}
