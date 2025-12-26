use rustfiber::JobSystem;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

#[test]
fn test_particle_system_like_subdivision() {
    let job_system = JobSystem::new(2);

    let num_particles = 100;
    let particles_processed = Arc::new(AtomicUsize::new(0));
    let processed_clone = particles_processed.clone();

    let counter = job_system.run_with_context(move |ctx| {
        let chunk_size = num_particles / 4;
        let mut counters = vec![];

        for _chunk_id in 0..4 {
            let processed = processed_clone.clone();
            let counter = ctx.spawn_job(move |_ctx| {
                for _ in 0..chunk_size {
                    processed.fetch_add(1, Ordering::SeqCst);
                    std::thread::sleep(Duration::from_micros(1));
                }
            });
            counters.push(counter);
        }

        for counter in counters {
            ctx.wait_for(&counter);
        }
    });

    job_system.wait_for_counter(&counter);
    assert_eq!(particles_processed.load(Ordering::SeqCst), num_particles);
    job_system.shutdown().expect("Shutdown failed");
}
