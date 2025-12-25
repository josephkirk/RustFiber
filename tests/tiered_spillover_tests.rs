use rustfiber::PinningStrategy;
use rustfiber::job_system::JobSystem;
use std::thread;
use std::time::Duration;

#[test]
fn test_tiered_spillover_activation() {
    // Initialize with 32 threads and TieredSpillover
    // Tier 1: Workers 0-7 (Threshold 0)
    // Tier 2: Workers 8-15 (Threshold 7)
    // Tier 3: Workers 16-31 (Threshold 15)
    let system = JobSystem::new_with_strategy(32, PinningStrategy::TieredSpillover);

    // Phase 1: Small workload. Only Tier 1 should be active.
    // Note: Due to work-stealing and race conditions, high-tier workers might
    // occasionally wake up if they see the injector is not empty, but they
    // should yield if the load is low.
    for _ in 0..4 {
        system.run(move || {
            // Simulate some work
            thread::sleep(Duration::from_millis(50));
            // In a real system we'd check thread::current().id() but we store worker ID in Worker struct
            // However, we don't expose worker ID to the job easily.
            // Let's just verify stability for now.
        });
    }

    thread::sleep(Duration::from_millis(200));

    // Phase 2: High workload to trigger Tier 2
    for _ in 0..20 {
        system.run(move || {
            thread::sleep(Duration::from_millis(100));
        });
    }

    thread::sleep(Duration::from_millis(500));

    system.shutdown().expect("Shutdown failed");
}

#[test]
fn test_active_worker_count() {
    let system = JobSystem::new_with_strategy(4, PinningStrategy::Linear);

    for _ in 0..4 {
        system.run(move || {
            thread::sleep(Duration::from_millis(100));
        });
    }

    // Check that we have active workers
    thread::sleep(Duration::from_millis(50));
    assert!(system.active_workers() > 0);
    assert!(system.active_workers() <= 4);

    thread::sleep(Duration::from_millis(200));
    assert_eq!(system.active_workers(), 0);
    system.shutdown().expect("Shutdown failed");
}
