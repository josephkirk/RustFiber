use rustfiber::JobSystem;

use std::time::Instant;

#[test]
fn test_swapping_latency() {
    println!("Starting latency test");
    // Force 1 worker to ensure swtiching happens on the same thread context
    let job_system = JobSystem::new(1);

    let switches = 10_000;

    let start = Instant::now();

    // We run a job that yields 'switches' times
    let c = job_system.run_with_context(move |ctx| {
        for _ in 0..switches {
            ctx.yield_now();
        }
    });

    job_system.wait_for_counter(&c);
    let duration = start.elapsed();

    println!("Performed {} switches in {:?}", switches, duration);

    // On Windows, thread::yield_now() or sleep() is ~15ms.
    // 10,000 * 15ms = 150 seconds.
    // Even if 1ms, it's 10 seconds.
    // Fiber switch should be < 1us. 10,000 * 1us = 10ms.
    // Debug build allowance: let's say 2s max (very generous, to allow for CI noise).
    assert!(
        duration.as_millis() < 200,
        "Fiber switching too slow! Likely OS yielding. Duration: {:?}",
        duration
    );

    job_system.shutdown().unwrap();
}
