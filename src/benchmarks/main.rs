pub mod fibonacci;
pub mod quicksort;
pub mod producer_consumer;
pub mod nas_benchmarks;
pub mod utils;

use fibonacci::run_fibonacci_benchmark;
use quicksort::run_quicksort_benchmark;
use producer_consumer::run_producer_consumer_benchmark;
use nas_benchmarks::{run_nas_ep_benchmark, run_nas_mg_benchmark, run_nas_cg_benchmark};

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
    let mut results = Vec::new();
    
    results.push(run_fibonacci_benchmark());
    results.push(run_quicksort_benchmark());
    results.push(run_producer_consumer_benchmark());
    results.push(run_nas_ep_benchmark());
    results.push(run_nas_mg_benchmark());
    results.push(run_nas_cg_benchmark());
    
    eprintln!("\n=======================================================");
    eprintln!("         All Benchmarks Completed!");
    eprintln!("=======================================================\n");
    
    // Output results as JSON to stdout
    match serde_json::to_string_pretty(&results) {
        Ok(json) => println!("{}", json),
        Err(e) => eprintln!("Error serializing results: {}", e),
    }
}
