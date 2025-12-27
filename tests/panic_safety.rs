use rustfiber::counter::Counter;
use rustfiber::job::Job;
use rustfiber::worker::WorkerPool;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

#[test]
fn test_panic_safety_counter_decrement() {
    let pool = WorkerPool::new(1);
    let counter = Counter::new(1);
    
    // Create a job that panics
    let job = Job::with_counter(
        || {
            panic!("Intentional panic for testing");
        },
        counter.clone(),
    );

    pool.submit(job);

    // Wait for a bit to let the job run and panic
    std::thread::sleep(Duration::from_millis(100));

    // Counter should be decremented despite panic
    assert!(counter.is_complete(), "Counter should be zero even after panic");
}

#[test]
fn test_worker_recovery_after_panic() {
    let pool = WorkerPool::new(1);
    let counter = Counter::new(1);
    
    // 1. Submit panicking job
    pool.submit(Job::with_counter(|| panic!("Boom"), counter.clone()));
    
    std::thread::sleep(Duration::from_millis(50));
    assert!(counter.is_complete());

    // 2. Submit normal job to verify worker is still alive
    let success = Arc::new(AtomicBool::new(false));
    let success_clone = success.clone();
    let counter2 = Counter::new(1);
    
    pool.submit(Job::with_counter(move || {
        success_clone.store(true, Ordering::SeqCst);
    }, counter2.clone()));

    // Wait for second job
    let start = std::time::Instant::now();
    while !counter2.is_complete() {
        if start.elapsed() > Duration::from_secs(1) {
            panic!("Worker did not process subsequent job!");
        }
        std::thread::sleep(Duration::from_millis(10));
    }

    assert!(success.load(Ordering::SeqCst), "Subsequent job failed to run");
}
