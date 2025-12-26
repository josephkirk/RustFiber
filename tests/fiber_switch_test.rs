use rustfiber::JobSystem;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

#[test]
fn test_fiber_yielding() {
    let job_system = JobSystem::new(4);

    let flag1 = Arc::new(AtomicBool::new(false));
    let flag2 = Arc::new(AtomicBool::new(false));

    let f1 = flag1.clone();
    let f2 = flag2.clone();

    // Job 1: Sets F1, Yields, Checks F2
    let counter1 = job_system.run_with_context(move |ctx| {
        println!("Job 1 started on thread {:?}", thread::current().id());
        f1.store(true, Ordering::SeqCst);

        println!("Job 1 yielding");
        ctx.yield_now();
        println!("Job 1 resumed on thread {:?}", thread::current().id());

        // Assert nothing, but logging proves it resumed.
    });

    // Job 2: Waits for F1, Sets F2
    let f1_clone = flag1.clone();
    let f2_clone = flag2;
    let counter2 = job_system.run_with_context(move |_ctx| {
        println!("Job 2 started on thread {:?}", thread::current().id());
        // Busy wait for F1 to ensure Job 1 ran at least once
        while !f1_clone.load(Ordering::SeqCst) {
            std::thread::yield_now(); // Thread yield to not hog CPU if on same thread
        }
        println!("Job 2 saw Job 1");
        f2_clone.store(true, Ordering::SeqCst);
    });

    job_system.wait_for_counter(&counter1);
    job_system.wait_for_counter(&counter2);

    // Ensure both ran
    assert!(flag1.load(Ordering::SeqCst));
    assert!(f2.load(Ordering::SeqCst)); // Wait, f2 was moved. f2 (original) is flagged flag2.
    // flag2 is in scope? yes.

    job_system.shutdown().unwrap();
}

#[test]
fn test_intrusive_wait() {
    println!("Starting intrusive wait test");
    let job_system = JobSystem::new(4);

    // Create a long running job
    let counter = job_system.run(|| {
        println!("Long job calling sleep");
        thread::sleep(Duration::from_millis(50));
        println!("Long job finished sleep");
    });

    let done = Arc::new(AtomicBool::new(false));
    let done_clone = done.clone();

    // Job that waits on the first job using fiber wait
    let wait_job_counter = job_system.run_with_context(move |ctx| {
        println!("Waiter job started");
        // This should suspend the fiber, not block the thread
        ctx.wait_for(&counter);
        println!("Waiter job finished (resumed)");
        done_clone.store(true, Ordering::SeqCst);
    });

    // Main thread waits for waiter
    job_system.wait_for_counter(&wait_job_counter);
    assert!(done.load(Ordering::SeqCst));

    job_system.shutdown().unwrap();
    println!("Using new fiber switch logic was successful");
}
