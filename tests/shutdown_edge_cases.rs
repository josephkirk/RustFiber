use rustfiber::JobSystem;
use std::time::Duration;

#[test]
fn test_shutdown_during_job_execution() {
    let job_system = JobSystem::new(2);

    // Submit multiple jobs that take some time
    for _ in 0..10 {
        job_system.run(|| {
            std::thread::sleep(Duration::from_millis(10));
        });
    }

    // Call shutdown immediately without waiting for jobs
    // Shutdown should wait for all jobs to complete
    let result = job_system.shutdown();
    assert!(
        result.is_ok(),
        "Shutdown should succeed after jobs complete"
    );
}
