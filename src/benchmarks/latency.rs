use crate::utils::{BenchmarkResult, DataPoint, SystemInfo};
use rustfiber::{JobSystem, PinningStrategy};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

pub fn run_empty_job_latency_benchmark(
    job_system: &JobSystem,
    strategy: PinningStrategy,
    threads: usize,
) -> BenchmarkResult {
    eprintln!("\n=== Benchmark: Empty Job Latency ===");

    let system_info = SystemInfo::collect(strategy, threads);
    eprintln!(
        "System: {} CPU cores, {:.2} GB total RAM, Strategy: {:?}",
        system_info.cpu_cores, system_info.total_memory_gb, strategy
    );

    // Local Warmup
    eprintln!("Warming up workers locally...");
    let warmup_counter = job_system.parallel_for_chunked_auto(0..threads * 100, |_| {
        std::hint::black_box(());
    });
    job_system.wait_for_counter(&warmup_counter);

    let test_sizes = vec![
        1, 10, 100, 1_000, 10_000, 100_000, 1_000_000,
    ];

    let mut data_points = Vec::new();
    let mut timed_out = false;
    let total_start = Instant::now();
    let timeout_duration = Duration::from_secs(crate::utils::DEFAULT_TIMEOUT_SECS);

    for &num_tasks in &test_sizes {
        if total_start.elapsed() > timeout_duration {
            eprintln!(
                "\n! Timeout reached ({}s), stopping benchmark.",
                crate::utils::DEFAULT_TIMEOUT_SECS
            );
            timed_out = true;
            break;
        }

        eprintln!("\nTesting with {} empty jobs...", num_tasks);

        // Measure scheduling latency: time from dispatch to job execution start
        let dispatch_start = Instant::now();
        let job_latencies = Arc::new(AtomicU64::new(0));
        let jobs_completed = Arc::new(AtomicU64::new(0));

        // Clone for use after the closure
        let job_latencies_clone = Arc::clone(&job_latencies);
        let jobs_completed_clone = Arc::clone(&jobs_completed);

        let counter = job_system.run_with_context(move |ctx| {
            for _ in 0..num_tasks {
                let job_latencies = Arc::clone(&job_latencies_clone);
                let jobs_completed = Arc::clone(&jobs_completed_clone);

                ctx.spawn_job(move |_| {
                    // Record the latency: time from dispatch start to job execution start
                    let latency_ns = dispatch_start.elapsed().as_nanos() as u64;

                    job_latencies.fetch_add(latency_ns, Ordering::Relaxed);
                    jobs_completed.fetch_add(1, Ordering::Relaxed);

                    // Empty job body
                    std::hint::black_box(());
                });
            }
        });

        job_system.wait_for_counter(&counter);

        // Calculate average scheduling latency
        let total_latency_ns = job_latencies.load(Ordering::Relaxed);
        let num_jobs_completed = jobs_completed.load(Ordering::Relaxed);

        if num_jobs_completed == 0 {
            eprintln!("  Warning: No jobs recorded as completed");
            continue;
        }

        let avg_latency_ns = total_latency_ns / num_jobs_completed;
        let avg_latency_us = avg_latency_ns as f64 / 1000.0;

        eprintln!(
            "  Average scheduling latency: {:.0} ns ({:.2} μs) per job",
            avg_latency_ns,
            avg_latency_us
        );

        data_points.push(DataPoint {
            num_tasks,
            time_ms: avg_latency_ns as f64 / 1_000_000.0, // Convert ns to ms for consistency
            metric_type: Some("latency".to_string()),
        });
    }

    BenchmarkResult {
        name: "Empty Job Latency".to_string(),
        data_points,
        system_info,
        crashed: false,
        crash_point: None,
        timed_out,
        #[cfg(feature = "metrics")]
        internal_metrics: crate::utils::capture_internal_metrics(job_system),
    }
}