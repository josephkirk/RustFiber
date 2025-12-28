use super::utils::{BenchmarkResult, DataPoint, SystemInfo};
use rustfiber::{JobSystem, PinningStrategy};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

pub fn run_allocation_benchmark(
    system: &JobSystem,
    strategy: PinningStrategy,
    threads: usize,
) -> BenchmarkResult {
    let system_info = SystemInfo::collect(strategy, threads);

    eprintln!("\n=== Allocation Throughput Benchmark ===");
    eprintln!(
        "System: {} CPU cores, {:.2} GB total RAM, Strategy: {:?}",
        system_info.cpu_cores, system_info.total_memory_gb, strategy
    );

    let test_sizes = vec![
        1, 100, 1_000, 10_000, 25_000, 50_000, 100_000, 200_000, 300_000, 500_000, 750_000,
        1_000_000,
    ];

    // Warmup: Local allocation priming
    // let root = system.run_with_context(|_ctx| {
    //     for _ in 0..10_000 {
    //         let _ = rustfiber::Job::new(|| {});
    //     }
    // });
    // system.wait_for_counter(&root);

    let mut data_points = Vec::new();
    let mut timed_out = false;
    let total_start = Instant::now();
    let timeout_duration = std::time::Duration::from_secs(super::utils::DEFAULT_TIMEOUT_SECS);

    for &count in &test_sizes {
        if total_start.elapsed() > timeout_duration {
            eprintln!(
                "\n! Timeout reached ({}s), stopping benchmark.",
                super::utils::DEFAULT_TIMEOUT_SECS
            );
            timed_out = true;
            break;
        }

        eprintln!("\nTesting allocation with {} items...", count);

        let inner_nanos = Arc::new(AtomicU64::new(0));
        let inner_nanos_clone = inner_nanos.clone();

        let root = system.run_with_context(move |_ctx| {
            let mut jobs = Vec::with_capacity(count);
            let t0 = Instant::now();
            for _ in 0..count {
                // Trigger FrameAllocator logic via Job::new optimization
                jobs.push(rustfiber::Job::new(|| {
                    std::hint::black_box(());
                }));
            }
            let t1 = t0.elapsed();
            inner_nanos_clone.store(t1.as_nanos() as u64, Ordering::SeqCst);

            eprintln!(
                "  Worker Inner Loop: {} jobs in {:?} ({:.2} ns/job)",
                count,
                t1,
                (t1.as_nanos() as f64) / (count as f64)
            );

            std::hint::black_box(&jobs);
        });
        system.wait_for_counter(&root);

        let duration_ns = inner_nanos.load(Ordering::SeqCst);
        let duration_ms = (duration_ns as f64) / 1_000_000.0;

        eprintln!(
            "  Completed in {:.2} ms ({:.2} ns/item)",
            duration_ms,
            (duration_ns as f64) / (count as f64)
        );

        data_points.push(DataPoint {
            num_tasks: count,
            time_ms: duration_ms,
        });
    }

    BenchmarkResult {
        name: "Allocation Throughput".to_string(),
        system_info,
        data_points,
        crashed: false,
        crash_point: None,
        timed_out,
    }
}
