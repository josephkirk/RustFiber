use rustfiber::{JobSystem, PinningStrategy};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

#[test]
fn test_none_strategy() {
    let job_system = JobSystem::new_with_strategy(4, PinningStrategy::None);
    assert_eq!(job_system.num_workers(), 4);
    job_system.shutdown().unwrap();
}

#[test]
fn test_linear_strategy() {
    let job_system = JobSystem::new_with_strategy(2, PinningStrategy::Linear);
    assert_eq!(job_system.num_workers(), 2);
    job_system.shutdown().unwrap();
}

#[test]
fn test_avoid_smt_strategy() {
    let job_system = JobSystem::new_with_strategy(2, PinningStrategy::AvoidSMT);
    assert_eq!(job_system.num_workers(), 2);

    let counter = job_system.run(|| {
        // We can't easily verify the actual affinity from here without platform-specific calls
        // but we verify the code path doesn't crash.
    });
    job_system.wait_for_counter(&counter);
    job_system.shutdown().unwrap();
}

#[test]
fn test_ccd_isolation_strategy() {
    // Attempt to request more than 8 workers to test fallback/wrapping
    let job_system = JobSystem::new_with_strategy(10, PinningStrategy::CCDIsolation);
    assert_eq!(job_system.num_workers(), 10);

    let executed = Arc::new(AtomicUsize::new(0));
    let mut jobs = Vec::new();
    for _ in 0..100 {
        let executed_clone = executed.clone();
        jobs.push(Box::new(move || {
            executed_clone.fetch_add(1, Ordering::SeqCst);
        }) as Box<dyn FnOnce() + Send>);
    }

    let counter = job_system.run_multiple(jobs);
    job_system.wait_for_counter(&counter);
    assert_eq!(executed.load(Ordering::SeqCst), 100);

    job_system.shutdown().unwrap();
}
