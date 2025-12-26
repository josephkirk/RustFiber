use rustfiber::JobSystem;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

#[test]
fn test_mixing_context_and_simple_jobs() {
    let job_system = JobSystem::new(2);
    let context_job_ran = Arc::new(AtomicUsize::new(0));
    let simple_job_ran = Arc::new(AtomicUsize::new(0));

    let ctx_clone = context_job_ran.clone();
    let ctx_counter = job_system.run_with_context(move |_ctx| {
        ctx_clone.fetch_add(1, Ordering::SeqCst);
    });

    let simple_clone = simple_job_ran.clone();
    let simple_counter = job_system.run(move || {
        simple_clone.fetch_add(1, Ordering::SeqCst);
    });

    job_system.wait_for_counter(&ctx_counter);
    job_system.wait_for_counter(&simple_counter);

    assert_eq!(context_job_ran.load(Ordering::SeqCst), 1);
    assert_eq!(simple_job_ran.load(Ordering::SeqCst), 1);
    job_system.shutdown().expect("Shutdown failed");
}
