use rustfiber::JobSystem;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

fn main() {
    println!("RustFiber - High-Performance Fiber-Based Job System\n");

    // Create a job system with 4 worker threads
    let num_threads = 4;
    let job_system = JobSystem::new(num_threads);
    println!("Initialized job system with {} worker threads\n", num_threads);

    // Example 1: Simple job execution
    println!("Example 1: Simple job execution");
    let counter = job_system.run(|| {
        println!("  Hello from a fiber job!");
    });
    job_system.wait_for_counter(&counter);
    println!("  Job completed\n");

    // Example 2: Multiple parallel jobs
    println!("Example 2: Parallel computation");
    let sum = Arc::new(AtomicUsize::new(0));
    let num_jobs = 100;
    
    let start = Instant::now();
    let mut jobs: Vec<Box<dyn FnOnce() + Send>> = Vec::new();
    
    for i in 0..num_jobs {
        let sum_clone = sum.clone();
        jobs.push(Box::new(move || {
            // Simulate some work
            let mut _local_sum = 0;
            for j in 0..1000 {
                _local_sum += j;
            }
            sum_clone.fetch_add(i, Ordering::SeqCst);
        }));
    }

    let counter = job_system.run_multiple(jobs);
    job_system.wait_for_counter(&counter);
    
    let duration = start.elapsed();
    let expected_sum: usize = (0..num_jobs).sum();
    println!("  Executed {} jobs in {:?}", num_jobs, duration);
    println!("  Sum result: {} (expected: {})\n", sum.load(Ordering::SeqCst), expected_sum);

    // Example 3: High-throughput test
    println!("Example 3: High-throughput benchmark");
    let num_jobs = 10000;
    let mut jobs: Vec<Box<dyn FnOnce() + Send>> = Vec::new();
    
    let start = Instant::now();
    for _ in 0..num_jobs {
        jobs.push(Box::new(|| {
            // Minimal work
            let mut _x = 0;
            for i in 0..10 {
                _x += i;
            }
        }));
    }

    let counter = job_system.run_multiple(jobs);
    job_system.wait_for_counter(&counter);
    
    let duration = start.elapsed();
    let jobs_per_second = num_jobs as f64 / duration.as_secs_f64();
    println!("  Executed {} jobs in {:?}", num_jobs, duration);
    println!("  Throughput: {:.2} jobs/second\n", jobs_per_second);

    // Shutdown the system
    println!("Shutting down job system...");
    match job_system.shutdown() {
        Ok(_) => println!("Done!"),
        Err(e) => eprintln!("Shutdown error: {}", e),
    }
}
