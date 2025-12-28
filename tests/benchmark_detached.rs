use rustfiber::{JobSystem, counter::Counter};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

#[test]
#[ignore]
fn benchmark_spawn_variations() {
    let system = JobSystem::new(4);
    let count = 50_000;

    // 1. spawn_job (Baseline: Frame Alloc + Heap Counter)
    let done_count = Arc::new(AtomicUsize::new(0));
    let c = done_count.clone();
    let start_base = Instant::now();
    let root = system.run_with_context(move |ctx| {
        for _ in 0..count {
            let c_inner = c.clone();
            let _ = ctx.spawn_job(move |_| {
                c_inner.fetch_add(1, Ordering::Relaxed);
            });
        }
    });
    system.wait_for_counter(&root);
    while done_count.load(Ordering::Relaxed) < count {
        std::thread::yield_now();
    }
    let duration_base = start_base.elapsed();
    println!("spawn_job (New Counter per Job): {:?}", duration_base);

    // 2. spawn_detached (Frame Alloc + No Counter)
    let done_count = Arc::new(AtomicUsize::new(0));
    let c = done_count.clone();
    let start_detached = Instant::now();
    let root = system.run_with_context(move |ctx| {
        for _ in 0..count {
            let c_inner = c.clone();
            ctx.spawn_detached(move |_| {
                c_inner.fetch_add(1, Ordering::Relaxed);
            });
        }
    });
    system.wait_for_counter(&root); // wait for spawning to finish
    while done_count.load(Ordering::Relaxed) < count {
        std::thread::yield_now(); // wait for jobs to finish
    }
    let duration_detached = start_detached.elapsed();
    println!("spawn_detached (No Counter): {:?}", duration_detached);

    // 3. spawn_with_counter (Frame Alloc + Shared Counter)
    let group = Counter::new(count as usize);
    let group_for_spawn = group.clone();

    let start_group = Instant::now();
    let root = system.run_with_context(move |ctx| {
        for _ in 0..count {
            ctx.spawn_with_counter(move |_| {}, group_for_spawn.clone());
        }
    });
    system.wait_for_counter(&root); // spawning done
    system.wait_for_counter(&group); // execution done
    let duration_group = start_group.elapsed();
    println!("spawn_with_counter (Shared Counter): {:?}", duration_group);

    println!(
        "Speedup Detached vs Base: {:.2}x",
        duration_base.as_secs_f64() / duration_detached.as_secs_f64()
    );
    println!(
        "Speedup Group vs Base: {:.2}x",
        duration_base.as_secs_f64() / duration_group.as_secs_f64()
    );
}
