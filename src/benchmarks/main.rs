pub mod allocation;
pub mod batching_benchmark;
pub mod fibonacci;
pub mod nas_benchmarks;
pub mod producer_consumer;
pub mod quicksort;
pub mod utils;

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

    eprintln!("=======================================================");
    eprintln!("           RustFiber Benchmark Suite");
    eprintln!("=======================================================");
    eprintln!("\nStrategy: {:?}", strategy);
    eprintln!("Threads:  {}", threads);

    // One-time Global Initialization (OS Settle + Warmup)
    eprintln!("\n[STARTUP] Initializing Global Job System...");
    let job_system =
        std::sync::Arc::new(rustfiber::JobSystem::new_with_strategy(threads, strategy));

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
        fibonacci::run_fibonacci_benchmark,
        quicksort::run_quicksort_benchmark,
        producer_consumer::run_producer_consumer_benchmark,
        nas_benchmarks::run_nas_ep_benchmark,
        nas_benchmarks::run_nas_mg_benchmark,
        nas_benchmarks::run_nas_cg_benchmark,
        batching_benchmark::run_batching_benchmark,
        allocation::run_allocation_benchmark,
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

    eprintln!("\n=======================================================");
    eprintln!("         All Benchmarks Completed!");
    eprintln!("=======================================================\n");
}
