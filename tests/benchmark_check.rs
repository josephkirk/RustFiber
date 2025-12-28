use rustfiber::JobSystem;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

#[test]
#[ignore] // Run with --ignored explicitly to see output
fn benchmark_heap_vs_frame() {
    let system = JobSystem::new(4);
    let count = 50_000;

    // 1. Heap Allocation (via global run)
    let start_heap = Instant::now();
    let mut counters = Vec::with_capacity(count);
    for _ in 0..count {
        counters.push(system.run(|| {}));
    }
    // Wait for all
    for c in counters {
        system.wait_for_counter(&c);
    }
    let duration_heap = start_heap.elapsed();
    println!(
        "Heap Alloc + Global Submit ({} jobs): {:?}",
        count, duration_heap
    );

    // 2. Frame Allocation (nested)
    // Let's use atomic counter.
    let done_count = Arc::new(AtomicUsize::new(0));
    let c = done_count.clone();

    let start_frame_run = Instant::now();
    let root = system.run_with_context(move |ctx| {
        for _ in 0..count {
            let c_inner = c.clone();
            let _ = ctx.spawn_job(move |_| {
                c_inner.fetch_add(1, Ordering::Relaxed);
            });
        }
    });
    system.wait_for_counter(&root);

    // Wait for atomic
    while done_count.load(Ordering::Relaxed) < count {
        std::thread::yield_now();
    }
    let duration_frame = start_frame_run.elapsed();

    println!(
        "Frame Alloc + Global Submit ({} jobs): {:?}",
        count, duration_frame
    );

    println!(
        "Speedup: {:.2}x",
        duration_heap.as_secs_f64() / duration_frame.as_secs_f64()
    );
}
