use crate::utils::{BenchmarkResult, DataPoint, SystemInfo};
use rustfiber::{JobSystem, PinningStrategy};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

fn fibonacci(n: u64) -> u64 {
    if n <= 1 {
        return n;
    }
    let mut a = 0;
    let mut b = 1;
    for _ in 2..=n {
        let temp = a + b;
        a = b;
        b = temp;
    }
    b
}

pub fn run_fibonacci_benchmark(
    job_system: &JobSystem,
    strategy: PinningStrategy,
    threads: usize,
) -> BenchmarkResult {
    eprintln!("\n=== Benchmark 1: Million Tiny Tasks (Fibonacci) ===");

    let system_info = SystemInfo::collect(strategy, threads);
    eprintln!(
        "System: {} CPU cores, {:.2} GB total RAM, Strategy: {:?}",
        system_info.cpu_cores, system_info.total_memory_gb, strategy
    );

    // Local Warmup: Ensure all threads are started and have allocated local resources
    eprintln!("Warming up workers locally...");
    let warmup_jobs: Vec<Box<dyn FnOnce() + Send>> = (0..threads * 4)
        .map(|_| Box::new(|| {}) as Box<dyn FnOnce() + Send>)
        .collect();
    let counter = job_system.run_multiple(warmup_jobs);
    job_system.wait_for_counter(&counter);

    let test_sizes = vec![
        1, 10, 100, 1_000, 10_000, 50_000, 100_000, 250_000, 500_000, 750_000, 1_000_000,
        1_250_000, 1_500_000, 2_000_000,
    ];

    let mut data_points = Vec::new();
    let mut timed_out = false;
    let total_start = Instant::now();
    let timeout_duration = std::time::Duration::from_secs(crate::utils::DEFAULT_TIMEOUT_SECS);

    for &num_tasks in &test_sizes {
        if total_start.elapsed() > timeout_duration {
            eprintln!(
                "\n! Timeout reached ({}s), stopping benchmark.",
                crate::utils::DEFAULT_TIMEOUT_SECS
            );
            timed_out = true;
            break;
        }

        eprintln!("\nTesting with {} tasks...", num_tasks);

        let start = Instant::now();
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let root_job = job_system.run_with_context(move |ctx| {
            for _ in 0..num_tasks {
                let c_inner = counter_clone.clone();
                ctx.spawn_detached(move |_| {
                    // Calculate fibonacci(20) for consistent small workload
                    let _ = fibonacci(20);
                    c_inner.fetch_add(1, Ordering::Relaxed);
                });
            }
        });

        job_system.wait_for_counter(&root_job);

        // Wait for all detached jobs to complete
        while counter.load(Ordering::Relaxed) < num_tasks {
            std::thread::yield_now();
        }

        let elapsed = start.elapsed();
        let elapsed_ms = elapsed.as_secs_f64() * 1000.0;

        eprintln!(
            "  Completed in {:.2} ms ({:.2} tasks/sec)",
            elapsed_ms,
            num_tasks as f64 / elapsed.as_secs_f64()
        );

        data_points.push(DataPoint {
            num_tasks,
            time_ms: elapsed_ms,
        });
    }

    BenchmarkResult {
        name: "Benchmark 1: Million Tiny Tasks (Fibonacci)".to_string(),
        data_points,
        system_info,
        crashed: false,
        crash_point: None,
        timed_out,
    }
}
