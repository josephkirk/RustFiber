use super::utils::BenchmarkResult;
use rustfiber::{JobSystem, PinningStrategy};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

pub fn run_batching_benchmark(strategy: PinningStrategy, threads: usize) -> BenchmarkResult {
    let job_system = JobSystem::new_with_strategy(threads, strategy);
    let system_info = crate::utils::SystemInfo::collect(strategy, threads);

    eprintln!("\n=== Parallel For Batching Benchmark ===");
    eprintln!(
        "System: {} CPU cores, {:.2} GB total RAM, Strategy: {:?}",
        system_info.cpu_cores, system_info.total_memory_gb, strategy
    );

    let test_sizes = vec![
        1_000, 5_000, 10_000, 25_000, 50_000, 100_000, 200_000, 300_000, 500_000, 750_000,
        1_000_000,
    ];

    let mut data_points = Vec::new();
    let mut timed_out = false;
    let total_start = Instant::now();
    let timeout_duration = std::time::Duration::from_secs(crate::utils::DEFAULT_TIMEOUT_SECS);

    // Synthetic workload to separate verify job overhead from atomic contention.
    // Without this, the benchmark measures cache coherence of a single atomic more than job overhead.
    fn heavy_work(n: u64) -> u64 {
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

    for &num_items in &test_sizes {
        if total_start.elapsed() > timeout_duration {
            eprintln!(
                "\n! Timeout reached ({}s), stopping benchmark.",
                crate::utils::DEFAULT_TIMEOUT_SECS
            );
            timed_out = true;
            break;
        }

        eprintln!("\nTesting with {} items...", num_items);
        let data = Arc::new(AtomicUsize::new(0));
        let start = Instant::now();

        // Use auto batching with chunked API for local accumulation
        let counter = job_system.parallel_for_chunked_auto(0..num_items, move |chunk_range| {
            let chunk_len = chunk_range.len();
            let mut local_sum = 0;
            // Iterate over the assigned chunk
            for _ in chunk_range {
                // Simulate work
                local_sum += heavy_work(15);
            }
            // Single atomic update per batch!
            data.fetch_add(chunk_len, Ordering::Relaxed);

            // To prevent heavy_work from being optimized out if we don't use local_sum:
            std::hint::black_box(local_sum);
        });

        job_system.wait_for_counter(&counter);

        let elapsed = start.elapsed();
        let elapsed_ms = elapsed.as_secs_f64() * 1000.0;

        eprintln!(
            "  Completed in {:.2} ms ({:.2} items/sec)",
            elapsed_ms,
            num_items as f64 / elapsed.as_secs_f64()
        );

        data_points.push(crate::utils::DataPoint {
            num_tasks: num_items,
            time_ms: elapsed_ms,
        });
    }

    job_system.shutdown().unwrap();

    BenchmarkResult {
        name: "Batching (Parallel For Auto)".to_string(),
        system_info,
        data_points,
        crashed: false,
        crash_point: None,
        timed_out,
    }
}
