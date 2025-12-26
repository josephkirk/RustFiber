use rustfiber::JobSystem;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

#[test]
fn test_recursive_parallel_decomposition() {
    let job_system = JobSystem::new(2);
    let result = Arc::new(AtomicUsize::new(0));

    let result_clone = result.clone();
    let counter = job_system.run_with_context(move |ctx| {
        let mut counters = vec![];

        for i in 0..4 {
            let result1 = result_clone.clone();
            let start = i * 25;
            let end = (i + 1) * 25;

            let child = ctx.spawn_job(move |_ctx| {
                let sum: usize = (start..end).sum();
                result1.fetch_add(sum, Ordering::SeqCst);
            });
            counters.push(child);
        }

        for c in counters {
            ctx.wait_for(&c);
        }
    });

    job_system.wait_for_counter(&counter);

    let expected: usize = (0..100).sum();
    assert_eq!(result.load(Ordering::SeqCst), expected);
    job_system.shutdown().expect("Shutdown failed");
}
