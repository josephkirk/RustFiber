pub mod fibonacci;
pub mod nas_benchmarks;
pub mod producer_consumer;
pub mod quicksort;
pub mod utils;

use fibonacci::run_fibonacci_benchmark;
use nas_benchmarks::{run_nas_cg_benchmark, run_nas_ep_benchmark, run_nas_mg_benchmark};
use producer_consumer::run_producer_consumer_benchmark;
use quicksort::run_quicksort_benchmark;

fn main() {
    eprintln!("=======================================================");
    eprintln!("           RustFiber Benchmark Suite");
    eprintln!("=======================================================");
    eprintln!("\nThis benchmark suite tests the RustFiber job system");
    eprintln!("with 4 different stress tests:\n");
    eprintln!("1. Million Tiny Tasks (Fibonacci)");
    eprintln!("2. Recursive Task Decomposition (QuickSort)");
    eprintln!("3. Producer-Consumer Stress Test");
    eprintln!("4. NAS Parallel Benchmarks (EP, MG, CG)");
    eprintln!("\n=======================================================\n");

    // Run all benchmarks and collect results
    let results = vec![
        run_fibonacci_benchmark(),
        run_quicksort_benchmark(),
        run_producer_consumer_benchmark(),
        run_nas_ep_benchmark(),
        run_nas_mg_benchmark(),
        run_nas_cg_benchmark(),
    ];

    eprintln!("\n=======================================================");
    eprintln!("         All Benchmarks Completed!");
    eprintln!("=======================================================\n");

    // Output results as JSON to stdout
    match serde_json::to_string_pretty(&results) {
        Ok(json) => println!("{}", json),
        Err(e) => eprintln!("Error serializing results: {}", e),
    }
}
