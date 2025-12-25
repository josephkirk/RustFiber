//! Integration tests for the fiber-based job system.

use crate::{Counter, JobSystem};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use std::thread;

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
    job_system.shutdown();
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
    job_system.shutdown();
}

#[test]
fn test_counter_synchronization() {
    let job_system = JobSystem::new(4);
    let counter = Counter::new(10);
    
    for _ in 0..10 {
        let counter_clone = counter.clone();
        job_system.run(move || {
            thread::sleep(Duration::from_millis(10));
            counter_clone.decrement();
        });
    }

    // Wait for all jobs
    while !counter.is_complete() {
        thread::sleep(Duration::from_millis(5));
    }

    assert!(counter.is_complete());
    job_system.shutdown();
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
    job_system.shutdown();
}
