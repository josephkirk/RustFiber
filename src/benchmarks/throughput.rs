use crate::utils::{BenchmarkResult, DataPoint, SystemInfo};
use rustfiber::{JobSystem, PinningStrategy};
use std::time::Instant;

pub fn run_dependency_graph_benchmark(
    job_system: &JobSystem,
    strategy: PinningStrategy,
    threads: usize,
) -> BenchmarkResult {
    eprintln!("\n=== Benchmark: Dependency Graph (Fork-Join) ===");

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
        1, 10, 100, 1_000, 10_000, 100_000,
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

        eprintln!("\nTesting dependency graph with {} tasks...", num_tasks);

        let start = Instant::now();
        let test_counter = rustfiber::Counter::new(num_tasks);
        let test_counter_clone = test_counter.clone();

        // Create a simple dependency graph: root spawns children, each child does work
        let root_counter = job_system.run_with_context(move |ctx| {
            for _ in 0..num_tasks {
                let c = test_counter_clone.clone();
                ctx.spawn_with_counter(
                    move |_| {
                        // Simulate some work
                        std::hint::black_box(fibonacci(10));
                    },
                    c,
                );
            }
        });

        job_system.wait_for_counter(&root_counter);
        job_system.wait_for_counter(&test_counter);

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
            metric_type: Some("time".to_string()),
        });
    }

    BenchmarkResult {
        name: "Dependency Graph Throughput".to_string(),
        data_points,
        system_info,
        crashed: false,
        crash_point: None,
        timed_out,
        #[cfg(feature = "metrics")]
        internal_metrics: crate::utils::capture_internal_metrics(job_system),
    }
}

pub fn run_parallel_for_scaling_benchmark(
    job_system: &JobSystem,
    strategy: PinningStrategy,
    threads: usize,
) -> BenchmarkResult {
    eprintln!("\n=== Benchmark: ParallelFor Scaling ===");

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

    let array_size = 1_000_000;

    // Serial run
    let serial_start = Instant::now();
    for _ in 0..array_size {
        std::hint::black_box(fibonacci(25));
    }
    let serial_time = serial_start.elapsed().as_secs_f64() * 1000.0;

    eprintln!("Serial time: {:.2} ms", serial_time);

    // Parallel run
    let parallel_start = Instant::now();
    let counter = job_system.parallel_for_chunked_auto(0..array_size, |range| {
        for _ in range {
            std::hint::black_box(fibonacci(25));
        }
    });
    job_system.wait_for_counter(&counter);
    let parallel_time = parallel_start.elapsed().as_secs_f64() * 1000.0;

    let scaling_factor = serial_time / parallel_time;

    eprintln!("Scaling factor: {:.2}x", scaling_factor);

    // For now, return a single data point with scaling factor as time_ms (abuse, but for now)
    let data_points = vec![DataPoint {
        num_tasks: array_size,
        time_ms: scaling_factor, // Actually scaling factor
        metric_type: Some("scaling_factor".to_string()),
    }];

    BenchmarkResult {
        name: "ParallelFor Scaling".to_string(),
        data_points,
        system_info,
        crashed: false,
        crash_point: None,
        timed_out: false,
        #[cfg(feature = "metrics")]
        internal_metrics: None, // No metrics collected for this benchmark
    }
}

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