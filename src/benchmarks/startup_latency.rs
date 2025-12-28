use crate::utils::{BenchmarkResult, DataPoint, SystemInfo};
use rustfiber::job_system::FiberConfig;
use rustfiber::{JobSystem, PinningStrategy};

pub fn run_startup_latency_benchmark(
    _job_system: &JobSystem,
    strategy: PinningStrategy,
    threads: usize,
) -> BenchmarkResult {
    eprintln!("\n=== Benchmark: Startup Latency ===");

    let system_info = SystemInfo::collect(strategy, threads);
    eprintln!(
        "System: {} CPU cores, {:.2} GB total RAM, Strategy: {:?}",
        system_info.cpu_cores, system_info.total_memory_gb, strategy
    );

    // Measure startup time for different fiber configurations
    let configs = vec![
        ("Default (16 fibers)", FiberConfig::default()),
        (
            "Minimal (4 fibers)",
            FiberConfig {
                initial_pool_size: 4,
                ..Default::default()
            },
        ),
        (
            "Large (64 fibers)",
            FiberConfig {
                initial_pool_size: 64,
                ..Default::default()
            },
        ),
    ];

    let mut data_points = Vec::new();

    for (name, config) in configs {
        // Measure startup time
        let start = std::time::Instant::now();

        // Create JobSystem with config
        let job_system = JobSystem::new_with_config(threads, config);

        // Wait for system to stabilize
        std::thread::sleep(std::time::Duration::from_millis(10));

        let startup_time = start.elapsed();

        // Clean up
        job_system.shutdown().ok();

        let startup_ms = startup_time.as_millis() as f64;
        eprintln!("  {}: {:.2}ms", name, startup_ms);

        data_points.push(DataPoint {
            num_tasks: 0, // Not applicable for startup benchmark
            time_ms: startup_ms,
        });
    }

    BenchmarkResult {
        name: "Startup Latency".to_string(),
        data_points,
        system_info,
        crashed: false,
        crash_point: None,
        timed_out: false,
    }
}
