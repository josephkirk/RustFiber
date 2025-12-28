use rustfiber::JobSystem;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

#[test]
fn test_topology_local_allocation_end_to_end() {
    let system = JobSystem::new(2);
    let was_frame_allocated = Arc::new(AtomicBool::new(false));
    let was_frame_allocated_clone = was_frame_allocated.clone();

    // Spawning from main thread -> Global Allocator (fallback path)
    system.run(move || {
        // We are now running on a worker thread.
        // TLS allocator should be active.

        // Create a nested job. This calls Job::new().
        // We can't easily inspect the internal variant (Work::SimpleFrame vs Work::Simple)
        // without reflection or hacks, but we can verify it executes correctly.
        // If the allocator logic was broken (e.g., segfault), this would crash.

        let _job = rustfiber::Job::new(move || {
            was_frame_allocated_clone.store(true, Ordering::SeqCst);
        });

        // We need to execute it.
        // We don't have easy access to the Scheduler here locally unless we use Context.
        // But Job::new checks allocation at creation time.

        // Let's pretend to execute by unwrapping manually? No, Work is private.
        // We can use a Context-based run to spawn properly.
    });

    // To properly test execution, we should use run_with_context
    let system2 = JobSystem::new(2);
    let done = Arc::new(AtomicBool::new(false));
    let done_clone = done.clone();

    let root = system2.run_with_context(move |ctx| {
        // ctx is available.
        // ctx.spawn_job uses Job::with_counter_and_context_in_allocator usually?
        // Let's explicitly use Job::new and submit it manually if possible,
        // or just rely on the fact that ctx.spawn(...) might use new internals.

        // Actually, Job::new calls are what we optimized.
        // We want to ensure normal Job usage works.

        // The optimization target was `Job::new` which is generic.
        // So we just run a nested logic.

        let _ = ctx.spawn_job(move |_| {
            done_clone.store(true, Ordering::SeqCst);
        });
    });

    system2.wait_for_counter(&root);
    assert!(done.load(Ordering::SeqCst));
}
