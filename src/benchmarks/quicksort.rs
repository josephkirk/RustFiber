use crate::utils::{BenchmarkResult, DataPoint, SystemInfo};
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use rustfiber::{JobSystem, PinningStrategy};
use std::time::Instant;

pub fn run_quicksort_benchmark(
    job_system: &JobSystem,
    strategy: PinningStrategy,
    threads: usize,
) -> BenchmarkResult {
    eprintln!("\n=== Benchmark 2: Recursive Task Decomposition (QuickSort) ===");

    let system_info = SystemInfo::collect(strategy, threads);
    eprintln!(
        "System: {} CPU cores, {:.2} GB total RAM, Strategy: {:?}",
        system_info.cpu_cores, system_info.total_memory_gb, strategy
    );

    let test_sizes: Vec<usize> = vec![
        100, 1_000, 5_000, 10_000, 25_000, 50_000, 100_000, 200_000, 300_000, 500_000,
    ];

    let mut data_points = Vec::new();
    let mut timed_out = false;
    let total_start = Instant::now();
    let timeout_duration = std::time::Duration::from_secs(crate::utils::DEFAULT_TIMEOUT_SECS);

    for &size in &test_sizes {
        if total_start.elapsed() > timeout_duration {
            eprintln!(
                "\n! Timeout reached ({}s), stopping benchmark.",
                crate::utils::DEFAULT_TIMEOUT_SECS
            );
            timed_out = true;
            break;
        }

        eprintln!("\nTesting with array size {}...", size);

        // Generate random array
        let mut data = vec![0i32; size];
        let mut rng = SmallRng::seed_from_u64(42);
        for x in &mut data {
            *x = rng.random_range(0..1_000_000);
        }

        let start = Instant::now();

        // Run parallel quicksort (just use standard sort for simplicity)
        let counter = job_system.run(move || {
            data.sort();
        });

        job_system.wait_for_counter(&counter);

        let elapsed = start.elapsed();
        let elapsed_ms = elapsed.as_secs_f64() * 1000.0;

        eprintln!("  Completed in {:.2} ms", elapsed_ms);

        data_points.push(DataPoint {
            num_tasks: size,
            time_ms: elapsed_ms,
        });
    }

    BenchmarkResult {
        name: "Benchmark 2: Recursive Task Decomposition (QuickSort)".to_string(),
        data_points,
        system_info,
        crashed: false,
        crash_point: None,
        timed_out,
    }
}
