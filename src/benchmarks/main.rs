pub mod utils;
pub mod latency;
pub mod throughput;
pub mod stress;
pub mod transform;

use rustfiber::PinningStrategy;
use std::sync::Arc;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let strategy = if args.len() > 1 {
        match args[1].to_lowercase().as_str() {
            "none" => PinningStrategy::None,
            "linear" => PinningStrategy::Linear,
            "avoid-smt" | "avoid_smt" => PinningStrategy::AvoidSMT,
            "ccd-isolation" | "ccd_isolation" => PinningStrategy::CCDIsolation,
            "tiered-spillover" => PinningStrategy::TieredSpillover,
            _ => {
                eprintln!("Unknown strategy: {}. Using Linear.", args[1]);
                PinningStrategy::Linear
            }
        }
    } else {
        PinningStrategy::Linear
    };

    let threads = if args.len() > 2 {
        args[2]
            .parse::<usize>()
            .unwrap_or_else(|_| utils::num_cpus())
    } else {
        utils::num_cpus()
    };

    #[cfg(feature = "metrics")]
    let enable_metrics = if args.len() > 3 && args[3] == "metrics" {
        true
    } else {
        false
    };

    #[cfg(feature = "tracing")]
    let _collector = rustfiber::tracing::CollectorGuard;
    #[cfg(feature = "tracing")]
    rustfiber::worker::WORKER_ID.with(|id| id.set(Some(99)));

    eprintln!("=======================================================");
    eprintln!("           RustFiber Benchmark Suite");
    eprintln!("=======================================================");
    eprintln!("\nStrategy: {:?}", strategy);
    eprintln!("Threads:  {}", threads);
    #[cfg(feature = "metrics")]
    eprintln!("Metrics:  {}", if enable_metrics { "Enabled" } else { "Disabled" });

    // One-time Global Initialization (OS Settle + Warmup)
    eprintln!("\n[STARTUP] Initializing Global Job System...");
    let mut job_system_builder = rustfiber::JobSystem::builder()
        .thread_count(threads)
        .pinning_strategy(strategy);

    #[cfg(feature = "metrics")]
    {
        job_system_builder = job_system_builder.enable_metrics(enable_metrics);
    }

    let job_system = std::sync::Arc::new(job_system_builder.build());

    eprintln!("[STARTUP] Waiting 20ms for OS thread stabilization...");
    std::thread::sleep(std::time::Duration::from_millis(20));

    eprintln!("[STARTUP] Performing global first-touch warmup...");
    let warmup_counter = job_system.parallel_for_chunked_auto(0..1_000_000, |_| {
        std::hint::black_box(());
    });
    job_system.wait_for_counter(&warmup_counter);
    eprintln!("[STARTUP] Job system is hot. Starting benchmarks.\n");

    eprintln!("=======================================================");

    let runs: Vec<fn(&rustfiber::JobSystem, PinningStrategy, usize) -> utils::BenchmarkResult> = vec![
        latency::run_empty_job_latency_benchmark,
        throughput::run_dependency_graph_benchmark,
        throughput::run_parallel_for_scaling_benchmark,
        stress::run_work_stealing_stress_benchmark,
        transform::run_transform_hierarchy_benchmark,
    ];

    for run in runs {
        let result = run(&job_system, strategy, threads);
        match serde_json::to_string(&result) {
            Ok(json) => println!("{}", json),
            Err(e) => eprintln!("Error serializing result: {}", e),
        }
    }

    // Explicit shutdown after all benchmarks are done
    eprintln!("\n[SHUTDOWN] Cleaning up job system...");
    if let Ok(js) = Arc::try_unwrap(job_system) {
        js.shutdown().ok();
    } else {
        eprintln!("Warning: Could not shutdown job system cleanly");
    }

    #[cfg(feature = "tracing")]
    {
        rustfiber::tracing::collect_local_trace();
        if let Err(e) = rustfiber::tracing::export_to_file("trace.json") {
            eprintln!("Error exporting trace: {}", e);
        } else {
            eprintln!("Trace exported to trace.json");
        }
    }

    eprintln!("\n=======================================================");
    eprintln!("         All Benchmarks Completed!");
    eprintln!("=======================================================\n");
}
