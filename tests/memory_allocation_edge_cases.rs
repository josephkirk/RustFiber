use rustfiber::job_system::FiberConfig;
use rustfiber::{Context, JobSystem};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

#[test]
fn test_memory_allocation_failure_fallback() {
    // Create job system with small frame allocator to force fallback
    let config = FiberConfig {
        frame_stack_size: 1024, // Very small, 1KB
        ..Default::default()
    };

    let job_system = JobSystem::new_with_config(2, config);
    let executed = Arc::new(AtomicUsize::new(0));
    let executed_clone = executed.clone();

    let counter = job_system.run_with_context(move |ctx: &Context| {
        // Spawn many jobs that allocate memory
        for i in 0..100 {
            let exec = executed_clone.clone();
            ctx.spawn_job(move |_| {
                // Allocate some data that might exceed frame size
                let data: Vec<u8> = vec![0; 100]; // 100 bytes each
                exec.fetch_add(data.len(), Ordering::SeqCst);
            });
        }
    });

    job_system.wait_for_counter(&counter);

    // Should have allocated 100 * 100 = 10,000 bytes
    assert_eq!(executed.load(Ordering::SeqCst), 10000);

    job_system.shutdown().unwrap();
}

#[test]
fn test_frame_allocator_exhaustion() {
    // Test that frame allocator handles exhaustion gracefully
    let config = FiberConfig {
        frame_stack_size: 512, // Very small
        ..Default::default()
    };

    let job_system = JobSystem::new_with_config(1, config);

    let counter = job_system.run_with_context(|ctx: &Context| {
        // Try to allocate more than frame size in nested jobs
        for _ in 0..10 {
            ctx.spawn_job(|_| {
                // This should fall back to heap if frame is full
                let _big_data: Vec<u8> = vec![0; 1024]; // 1KB, larger than frame
            });
        }
    });

    job_system.wait_for_counter(&counter);
    job_system.shutdown().unwrap();
}
