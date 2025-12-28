//! Integration tests for the fiber-based job system.

use crate::{Counter, JobSystem};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::Duration;

#[test]
fn test_basic_job_execution() {
    let job_system = JobSystem::new(2);
    let value = Arc::new(AtomicUsize::new(0));
    let value_clone = value.clone();

    let counter = job_system.run(move || {
        value_clone.store(42, Ordering::SeqCst);
    });

    job_system.wait_for_counter(&counter);
    assert_eq!(value.load(Ordering::SeqCst), 42);
    job_system.shutdown().expect("Shutdown failed");
}

#[test]
fn test_parallel_job_execution() {
    let job_system = JobSystem::new(4);
    let sum = Arc::new(AtomicUsize::new(0));

    let num_jobs = 100;
    let mut jobs: Vec<Box<dyn FnOnce() + Send>> = Vec::new();

    for i in 0..num_jobs {
        let sum_clone = sum.clone();
        jobs.push(Box::new(move || {
            sum_clone.fetch_add(i, Ordering::SeqCst);
        }));
    }

    let counter = job_system.run_multiple(jobs);
    job_system.wait_for_counter(&counter);

    let expected_sum: usize = (0..num_jobs).sum();
    assert_eq!(sum.load(Ordering::SeqCst), expected_sum);
    job_system.shutdown().expect("Shutdown failed");
}

#[test]
fn test_counter_synchronization() {
    let job_system = JobSystem::new(4);
    let counter = Counter::new(10);

    for _ in 0..10 {
        let counter_clone = counter.clone();
        job_system.run(move || {
            thread::sleep(Duration::from_millis(10));
            // Use dummy injector for testing (no waiters expected)
            let injector = crossbeam::deque::Injector::<crate::job::Job>::new();
            counter_clone.decrement(&injector);
        });
    }

    // Wait for all jobs
    while !counter.is_complete() {
        thread::sleep(Duration::from_millis(5));
    }

    assert!(counter.is_complete());
    job_system.shutdown().expect("Shutdown failed");
}

#[test]
fn test_high_throughput() {
    let job_system = JobSystem::new(8);
    let num_jobs = 1000;
    let mut jobs: Vec<Box<dyn FnOnce() + Send>> = Vec::new();

    for _ in 0..num_jobs {
        jobs.push(Box::new(|| {
            // Simulate some work
            let mut _sum = 0;
            for i in 0..100 {
                _sum += i;
            }
        }));
    }

    let counter = job_system.run_multiple(jobs);
    job_system.wait_for_counter(&counter);

    assert!(counter.is_complete());
    job_system.shutdown().expect("Shutdown failed");
}
#[test]
fn test_yield_fairness() {
    let job_system = JobSystem::new(2);
    // Use an atomic log to track order/interleaving
    let log = Arc::new(std::sync::Mutex::new(Vec::new()));

    let log1 = log.clone();
    let job1 = job_system.run(move || {
        for _ in 0..5 {
            log1.lock().unwrap().push(1);
            // This yield should allow job2 to run if on same thread,
            // or just be a no-op if on different threads.
            // But we want to test that it doesn't block forever.
            crate::context::yield_now();
        }
    });

    let log2 = log.clone();
    let job2 = job_system.run(move || {
        for _ in 0..5 {
            log2.lock().unwrap().push(2);
            crate::context::yield_now();
        }
    });

    job_system.wait_for_counter(&job1);
    job_system.wait_for_counter(&job2);

    let final_log = log.lock().unwrap();
    assert_eq!(final_log.len(), 10);
    // We expect some mixing, though exact interleaving depends on thread scheduling.
    // At least ensure it completed.

    job_system.shutdown().expect("Shutdown failed");
}

#[test]
fn test_priority_scheduling() {
    // Single thread to force serialization and simpler priority check
    let job_system = JobSystem::new(1);
    let execution_order = Arc::new(std::sync::Mutex::new(Vec::new()));

    // We want to fill the worker with a long running job, then submit Low, then High.
    // The High should technically run before Low when the long job yields or finishes?
    // Actually, getting deterministic priority order in a concurrent system is hard.
    // But with 1 thread, if we submit Low then High, and the worker picks up High first
    // (if neither has started), that proves priority.

    // However, submit() pushes to queue. Normal injector is FIFO-ish.
    // If we push Low (injector) then High (high_priority_injector).
    // The worker loop checks High before Injector.

    // Pause the worker for a moment to let us submit both
    let barrier = Arc::new(std::sync::Barrier::new(2));
    let barrier_clone = barrier.clone();

    // Occupy the worker
    let c_block = job_system.run(move || {
        barrier_clone.wait(); // Wait for main thread to submit others
        thread::sleep(Duration::from_millis(50)); // Give time for submission
    });

    barrier.wait(); // Worker is now running c_block

    let order1 = execution_order.clone();
    let c_low = job_system.run_priority(crate::job::JobPriority::Low, move || {
        order1.lock().unwrap().push("Low");
    });

    let order2 = execution_order.clone();
    let c_high = job_system.run_priority(crate::job::JobPriority::High, move || {
        order2.lock().unwrap().push("High");
    });

    job_system.wait_for_counter(&c_block);
    job_system.wait_for_counter(&c_low);
    job_system.wait_for_counter(&c_high);

    let final_order = execution_order.lock().unwrap();
    // High should run before Low because Worker checks high_priority_injector first
    assert_eq!(*final_order, vec!["High", "Low"]);

    job_system.shutdown().expect("Shutdown failed");
}
