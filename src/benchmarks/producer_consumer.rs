use rustfiber::JobSystem;
use std::sync::{Arc, Mutex};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;
use crate::utils::{BenchmarkResult, DataPoint, SystemInfo, num_cpus};

pub fn run_producer_consumer_benchmark() -> BenchmarkResult {
    eprintln!("\n=== Benchmark 3: Producer-Consumer Stress Test ===");
    
    let system_info = SystemInfo::collect();
    eprintln!("System: {} CPU cores, {:.2} GB total RAM", 
             system_info.cpu_cores, system_info.total_memory_gb);
    
    let job_system = JobSystem::new(num_cpus());
    
    let test_sizes = vec![
        1_000, 5_000, 10_000, 25_000, 50_000, 100_000, 200_000, 
        300_000, 500_000
    ];
    
    let mut data_points = Vec::new();
    
    for &num_items in &test_sizes {
        eprintln!("\nTesting with {} items...", num_items);
        
        // Shared queue for producer-consumer pattern
        let queue = Arc::new(Mutex::new(VecDeque::new()));
        let produced = Arc::new(AtomicUsize::new(0));
        let consumed = Arc::new(AtomicUsize::new(0));
        
        let start = Instant::now();
        
        // Create producer jobs
        let num_producers = num_cpus() / 2;
        let items_per_producer = num_items / num_producers;
        
        let mut producer_jobs: Vec<Box<dyn FnOnce() + Send>> = Vec::new();
        for _ in 0..num_producers {
            let queue_clone = queue.clone();
            let produced_clone = produced.clone();
            
            producer_jobs.push(Box::new(move || {
                for i in 0..items_per_producer {
                    let mut q = queue_clone.lock().unwrap();
                    q.push_back(i);
                    drop(q); // Release lock
                    produced_clone.fetch_add(1, Ordering::SeqCst);
                }
            }));
        }
        
        // Create consumer jobs
        let num_consumers = num_cpus() / 2;
        let mut consumer_jobs: Vec<Box<dyn FnOnce() + Send>> = Vec::new();
        
        for _ in 0..num_consumers {
            let queue_clone = queue.clone();
            let consumed_clone = consumed.clone();
            let target = num_items;
            
            consumer_jobs.push(Box::new(move || {
                while consumed_clone.load(Ordering::SeqCst) < target {
                    let item = {
                        let mut q = queue_clone.lock().unwrap();
                        q.pop_front()
                    };
                    
                    if item.is_some() {
                        consumed_clone.fetch_add(1, Ordering::SeqCst);
                        // Simulate some work
                        let _ = item.unwrap() * 2;
                    } else {
                        // Brief yield if queue is empty
                        std::thread::sleep(std::time::Duration::from_micros(1));
                    }
                }
            }));
        }
        
        // Start all jobs
        let mut all_jobs = producer_jobs;
        all_jobs.extend(consumer_jobs);
        
        let counter = job_system.run_multiple(all_jobs);
        job_system.wait_for_counter(&counter);
        
        let elapsed = start.elapsed();
        let elapsed_ms = elapsed.as_secs_f64() * 1000.0;
        
        eprintln!("  Produced: {}, Consumed: {}", 
                 produced.load(Ordering::SeqCst),
                 consumed.load(Ordering::SeqCst));
        eprintln!("  Completed in {:.2} ms ({:.2} items/sec)", 
                 elapsed_ms, num_items as f64 / elapsed.as_secs_f64());
        
        data_points.push(DataPoint {
            num_tasks: num_items,
            time_ms: elapsed_ms,
        });
    }
    
    job_system.shutdown().ok();
    
    BenchmarkResult {
        name: "Benchmark 3: Producer-Consumer Stress Test".to_string(),
        data_points,
        system_info,
        crashed: false,
        crash_point: None,
    }
}
