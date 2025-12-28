use super::utils::{BenchmarkResult, DataPoint, SystemInfo};
use rustfiber::{JobSystem, PinningStrategy};
use std::time::Instant;

pub fn run_allocation_benchmark(strategy: PinningStrategy, threads: usize) -> BenchmarkResult {
    eprintln!("Running Allocation Throughput Benchmark...");
    let system = JobSystem::new_with_strategy(threads, strategy);
    let system_info = SystemInfo::collect(strategy, threads);
    let count = 1_000_000;
    
    // Warmup
    let root = system.run_with_context(|_ctx| {
        for _ in 0..1000 {
             let _ = rustfiber::Job::new(|| {});
        }
    });
    system.wait_for_counter(&root);

    // Benchmark: Creation Only
    let start = Instant::now();
    let root = system.run_with_context(move |_ctx| {
        let mut jobs = Vec::with_capacity(count);
        let t0 = Instant::now();
        for _ in 0..count {
            jobs.push(rustfiber::Job::new(|| { std::hint::black_box(()); }));
        }
        let t1 = t0.elapsed();
        
        eprintln!("Worker Inner Loop: {} jobs in {:?} ({:.2} ns/job)", 
             count, t1, (t1.as_nanos() as f64) / (count as f64));
             
        std::hint::black_box(&jobs);
    });
    system.wait_for_counter(&root);
    
    let duration = start.elapsed();
    let duration_ms = duration.as_secs_f64() * 1000.0;
    
    let data_points = vec![DataPoint {
        num_tasks: count,
        time_ms: duration_ms,
    }];

    system.shutdown().unwrap();

    BenchmarkResult {
        name: "Allocation Throughput".to_string(),
        system_info,
        data_points,
        crashed: false,
        crash_point: None,
        timed_out: false,
    }
}
