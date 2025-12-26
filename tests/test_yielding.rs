use rustfiber::JobSystem;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

#[test]
fn test_cooperative_yielding() {
    let job_system = JobSystem::new(2);
    let iterations_completed = Arc::new(AtomicUsize::new(0));
    let other_job_ran = Arc::new(AtomicUsize::new(0));

    let iterations = iterations_completed.clone();
    let other = other_job_ran.clone();

    let long_counter = job_system.run_with_context(move |ctx| {
        for i in 0..100 {
            iterations.fetch_add(1, Ordering::SeqCst);

            if i % 10 == 0 {
                ctx.yield_now();
            }
        }
    });

    let short_counter = job_system.run_with_context(move |_ctx| {
        other.fetch_add(1, Ordering::SeqCst);
    });

    job_system.wait_for_counter(&long_counter);
    job_system.wait_for_counter(&short_counter);

    assert_eq!(iterations_completed.load(Ordering::SeqCst), 100);
    assert_eq!(other_job_ran.load(Ordering::SeqCst), 1);
    job_system.shutdown().expect("Shutdown failed");
}
