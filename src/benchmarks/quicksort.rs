use crate::utils::{BenchmarkResult, DataPoint, SystemInfo};
use rustfiber::{JobSystem, PinningStrategy, counter::Counter};
use std::time::Instant;

fn sequential_quicksort(arr: &mut [i32]) {
    if arr.len() <= 1 {
        return;
    }
    arr.sort();
}

pub fn run_quicksort_benchmark(strategy: PinningStrategy, threads: usize) -> BenchmarkResult {
    eprintln!("\n=== Benchmark 2: Recursive Task Decomposition (QuickSort) ===");

    let system_info = SystemInfo::collect(strategy, threads);
    eprintln!(
        "System: {} CPU cores, {:.2} GB total RAM, Strategy: {:?}",
        system_info.cpu_cores, system_info.total_memory_gb, strategy
    );

    let job_system = JobSystem::new_with_strategy(threads, strategy);

    let test_sizes: Vec<usize> = vec![
        1_000, 5_000, 10_000, 25_000, 50_000, 100_000, 200_000, 300_000, 500_000,
    ];

    let mut data_points = Vec::new();
    let mut timed_out = false;
    let total_start = Instant::now();
    let timeout_duration = std::time::Duration::from_secs(crate::utils::DEFAULT_TIMEOUT_SECS);

    for &array_size in &test_sizes {
        if total_start.elapsed() > timeout_duration {
            eprintln!(
                "\n! Timeout reached ({}s), stopping benchmark.",
                crate::utils::DEFAULT_TIMEOUT_SECS
            );
            timed_out = true;
            break;
        }

        eprintln!("\nTesting with array size {}...", array_size);

        // Generate array to sort
        let arr: Vec<i32> = (0..array_size)
            .map(|i| (i as i32 * 17) % 1000)
            .rev()
            .collect();

        let start = Instant::now();

        // Simulate recursive task decomposition by creating jobs for chunks
        let chunk_size = 1000_usize;
        let num_chunks = array_size.div_ceil(chunk_size);

        let root_job = job_system.run_with_context(move |ctx| {
            let group = Counter::new(num_chunks);

            for chunk_idx in 0..num_chunks {
                let start_idx = chunk_idx * chunk_size;
                let end_idx = ((chunk_idx + 1) * chunk_size).min(array_size);
                // Note: Allocating vector here is fine as it simulates data preparation
                let mut chunk: Vec<i32> = arr[start_idx..end_idx].to_vec();
                let group_clone = group.clone();

                ctx.spawn_with_counter(
                    move |_| {
                        sequential_quicksort(&mut chunk);
                    },
                    group_clone,
                );
            }

            ctx.wait_for(&group);
        });

        job_system.wait_for_counter(&root_job);

        let elapsed = start.elapsed();
        let elapsed_ms = elapsed.as_secs_f64() * 1000.0;

        eprintln!("  Completed in {:.2} ms", elapsed_ms);

        data_points.push(DataPoint {
            num_tasks: array_size,
            time_ms: elapsed_ms,
        });
    }

    job_system.shutdown().ok();

    BenchmarkResult {
        name: "Benchmark 2: Recursive Task Decomposition (QuickSort)".to_string(),
        data_points,
        system_info,
        crashed: false,
        crash_point: None,
        timed_out,
    }
}
