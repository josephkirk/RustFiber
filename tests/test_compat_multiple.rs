use rustfiber::JobSystem;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

#[test]
fn test_backwards_compatibility_multiple_jobs() {
    let job_system = JobSystem::new(2);
    let sum = Arc::new(AtomicUsize::new(0));

    let num_jobs = 50;
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
