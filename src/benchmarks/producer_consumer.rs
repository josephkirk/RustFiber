use crate::utils::{BenchmarkResult, DataPoint, SystemInfo};
use crossbeam::queue::SegQueue;
use rustfiber::{JobSystem, PinningStrategy};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

/// Benchmark for Producer-Consumer pattern using a lock-free queue.
/// This tests the pure throughput of the job system and fiber switching
/// without being bottlenecked by global mutex contention.
pub fn run_producer_consumer_benchmark(
    job_system: &JobSystem,
    strategy: PinningStrategy,
    threads: usize,
) -> BenchmarkResult {
    eprintln!("\n=== Benchmark 3: Producer-Consumer (Lock-Free) ===");

    let system_info = SystemInfo::collect(strategy, threads);
    eprintln!(
        "System: {} CPU cores, {:.2} GB total RAM, Strategy: {:?}",
        system_info.cpu_cores, system_info.total_memory_gb, strategy
    );

    let test_sizes = vec![
        100, 1_000, 10_000, 25_000, 50_000, 100_000, 200_000, 300_000, 500_000,
    ];

    let mut data_points = Vec::new();
    let mut timed_out = false;
    let total_start = Instant::now();
    let timeout_duration = std::time::Duration::from_secs(crate::utils::DEFAULT_TIMEOUT_SECS);

    for &num_items in &test_sizes {
        if total_start.elapsed() > timeout_duration {
            eprintln!(
                "\n! Timeout reached ({}s), stopping benchmark.",
                crate::utils::DEFAULT_TIMEOUT_SECS
            );
            timed_out = true;
            break;
        }

        eprintln!("\nTesting with {} items...", num_items);

        // Shared queue for producer-consumer pattern
        let queue = Arc::new(SegQueue::new());
        let produced = Arc::new(AtomicUsize::new(0));
        let consumed = Arc::new(AtomicUsize::new(0));

        let start = Instant::now();

        // Create producer jobs
        let num_producers = (threads / 2).max(1);
        let items_per_producer = num_items / num_producers;
        let remainder = num_items % num_producers;

        let mut producer_jobs: Vec<Box<dyn FnOnce() + Send>> = Vec::new();
        for i in 0..num_producers {
            let queue_clone = queue.clone();
            let produced_clone = produced.clone();
            let my_items = if i == num_producers - 1 {
                items_per_producer + remainder
            } else {
                items_per_producer
            };

            producer_jobs.push(Box::new(move || {
                // Push items directly (SegQueue is lock-free)
                for j in 0..my_items {
                    queue_clone.push(j);
                }
                produced_clone.fetch_add(my_items, Ordering::SeqCst);
            }));
        }

        // Create consumer jobs
        let num_consumers = (threads / 2).max(1);
        let mut consumer_jobs: Vec<Box<dyn FnOnce() + Send>> = Vec::new();

        for _ in 0..num_consumers {
            let queue_clone = queue.clone();
            let consumed_clone = consumed.clone();
            let target = num_items;

            consumer_jobs.push(Box::new(move || {
                while consumed_clone.load(Ordering::SeqCst) < target {
                    if let Some(item) = queue_clone.pop() {
                        consumed_clone.fetch_add(1, Ordering::SeqCst);
                        // Simulate work
                        let _ = item * 2;
                    } else {
                        // Brief yield/spin if queue is empty
                        std::thread::yield_now();
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

        eprintln!(
            "  Produced: {}, Consumed: {}",
            produced.load(Ordering::SeqCst),
            consumed.load(Ordering::SeqCst)
        );
        eprintln!(
            "  Completed in {:.2} ms ({:.2} items/sec)",
            elapsed_ms,
            num_items as f64 / elapsed.as_secs_f64()
        );

        data_points.push(DataPoint {
            num_tasks: num_items,
            time_ms: elapsed_ms,
            metric_type: None,
        });
    }

    BenchmarkResult {
        name: "Benchmark 3: Producer-Consumer (Lock-Free)".to_string(),
        data_points,
        system_info,
        crashed: false,
        crash_point: None,
        #[cfg(feature = "metrics")]
        internal_metrics: crate::utils::capture_internal_metrics(job_system),
        timed_out,
    }
}
