use rustfiber::JobSystem;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

#[test]
fn test_backwards_compatibility_simple_jobs() {
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
