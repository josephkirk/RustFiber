use rustfiber::JobSystem;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

#[test]
fn test_frame_allocation_lifecycle() {
    let system = JobSystem::new(2);
    let counter_val = Arc::new(AtomicUsize::new(0));
    let c = counter_val.clone();

    // Run a root job (heap)
    let jobs_done = system.run_with_context(move |ctx| {
        // Spawn nested job (frame allocated by default in nested context)
        let _ = ctx.spawn_job(move |_| {
            c.fetch_add(1, Ordering::SeqCst);
        });
    });
    
    system.wait_for_counter(&jobs_done);
    assert_eq!(counter_val.load(Ordering::SeqCst), 1);
    
    // Start new frame (reset allocators)
    // NOTE: In a real engine this is called after ensuring all frame jobs are done.
    // We simulated that with wait_for_counter above.
    system.start_new_frame();
    
    // Run again to ensure allocator reset worked and didn't crash
    let c2 = counter_val.clone();
    let jobs_done2 = system.run_with_context(move |ctx| {
        // Spawn multiple nested jobs to fill allocator slightly
        for _ in 0..10 {
            let c3 = c2.clone();
            ctx.spawn_job(move |_| {
                c3.fetch_add(1, Ordering::SeqCst);
            });
        }
    });
    
    system.wait_for_counter(&jobs_done2);
    assert_eq!(counter_val.load(Ordering::SeqCst), 11);
    
    system.shutdown().unwrap();
}
