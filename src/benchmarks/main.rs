pub mod fibonacci;
pub mod nas_benchmarks;
pub mod producer_consumer;
pub mod quicksort;
pub mod utils;

use fibonacci::run_fibonacci_benchmark;
use nas_benchmarks::{run_nas_cg_benchmark, run_nas_ep_benchmark, run_nas_mg_benchmark};
use producer_consumer::run_producer_consumer_benchmark;
use quicksort::run_quicksort_benchmark;

use rustfiber::PinningStrategy;

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
    eprintln!("\nThis benchmark suite tests the RustFiber job system");
    eprintln!("with 4 different stress tests:\n");
    eprintln!("1. Million Tiny Tasks (Fibonacci)");
    eprintln!("2. Recursive Task Decomposition (QuickSort)");
    eprintln!("3. Producer-Consumer Stress Test");
    eprintln!("4. NAS Parallel Benchmarks (EP, MG, CG)");
    eprintln!("\n=======================================================\n");

    // Run benchmarks one by one and output JSON for each immediately
    let runs: Vec<fn(PinningStrategy, usize) -> utils::BenchmarkResult> = vec![
        run_fibonacci_benchmark,
        run_quicksort_benchmark,
        run_producer_consumer_benchmark,
        run_nas_ep_benchmark,
        run_nas_mg_benchmark,
        run_nas_cg_benchmark,
    ];

    for run in runs {
        let result = run(strategy, threads);
        match serde_json::to_string(&result) {
            Ok(json) => println!("{}", json),
            Err(e) => eprintln!("Error serializing result: {}", e),
        }
    }

    eprintln!("\n=======================================================");
    eprintln!("         All Benchmarks Completed!");
    eprintln!("=======================================================\n");
}
