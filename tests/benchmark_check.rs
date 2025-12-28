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
    // We spawn ONE root job, which spawns `count` children using ctx.spawn_job (Frame Alloc)
    let start_frame = Instant::now();
    let root = system.run_with_context(move |ctx| {
        for _ in 0..count {
            // ctx.spawn_job uses FrameAllocator because root job runs on worker
            let _ = ctx.spawn_job(|_| {});
        }
    });
    system.wait_for_counter(&root); // This waits for children? 
    // Wait, ctx.spawn_job returns a counter. We didn't wait for them!
    // But `root` counter only tracks the root job itself.
    // The root job finishes after spawning.
    // The children might still be running.
    // To measure throughput, we need to wait for children.
    // We can't wait for 50k counters easily in the loop (it would serialize them).
    // We can use a shared atomic to track completions?

    // Better: use `spawn_jobs` batch? spawning individually is what we want to test (alloc overhead).

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
