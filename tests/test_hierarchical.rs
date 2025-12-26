use rustfiber::JobSystem;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

#[test]
fn test_hierarchical_job_tree() {
    let job_system = JobSystem::new(2);
    let total_work = Arc::new(AtomicUsize::new(0));

    let work = total_work.clone();
    let counter = job_system.run_with_context(move |ctx| {
        let mut child_counters = vec![];

        for _ in 0..3 {
            let work1 = work.clone();
            let child = ctx.spawn_job(move |ctx| {
                work1.fetch_add(1, Ordering::SeqCst);

                let mut grandchild_counters = vec![];
                for _ in 0..2 {
                    let work2 = work1.clone();
                    let grandchild = ctx.spawn_job(move |_ctx| {
                        work2.fetch_add(1, Ordering::SeqCst);
                    });
                    grandchild_counters.push(grandchild);
                }

                for gc in grandchild_counters {
                    ctx.wait_for(&gc);
                }
            });
            child_counters.push(child);
        }

        for c in child_counters {
            ctx.wait_for(&c);
        }
    });

    job_system.wait_for_counter(&counter);

    assert_eq!(total_work.load(Ordering::SeqCst), 9);
    job_system.shutdown().expect("Shutdown failed");
}
