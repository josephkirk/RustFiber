use rustfiber::JobSystem;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

#[test]
fn test_producer_consumer_pattern() {
    let job_system = JobSystem::new(2);
    let items_produced = Arc::new(AtomicUsize::new(0));
    let items_consumed = Arc::new(AtomicUsize::new(0));

    let produced = items_produced.clone();
    let consumed = items_consumed.clone();

    let counter = job_system.run_with_context(move |ctx| {
        let num_items = 20;
        let mut consumer_counters = vec![];

        for _i in 0..num_items {
            produced.fetch_add(1, Ordering::SeqCst);
            let consumed_clone = consumed.clone();

            let consumer = ctx.spawn_job(move |_ctx| {
                std::thread::sleep(Duration::from_micros(10));
                consumed_clone.fetch_add(1, Ordering::SeqCst);
            });
            consumer_counters.push(consumer);
        }

        for (_i, counter) in consumer_counters.iter().enumerate() {
            ctx.wait_for(counter);
        }
    });

    job_system.wait_for_counter(&counter);
    assert_eq!(items_produced.load(Ordering::SeqCst), 20);
    assert_eq!(items_consumed.load(Ordering::SeqCst), 20);
    job_system.shutdown().expect("Shutdown failed");
}
