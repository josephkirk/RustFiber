//! Integration tests for Context-based nested parallelism
//!
//! These tests demonstrate the practical use cases enabled by the Context type,
//! including the examples from the problem statement.

use rustfiber::JobSystem;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

#[test]
fn test_particle_system_like_subdivision() {
    // This test simulates the particle system example from the problem statement
    // where work is subdivided into chunks processed in parallel
    let job_system = JobSystem::new(4);
    
    let num_particles = 100;
    let particles_processed = Arc::new(AtomicUsize::new(0));
    let processed_clone = particles_processed.clone();
    
    let counter = job_system.run_with_context(move |ctx| {
        // Simulate subdividing work across multiple jobs
        let chunk_size = num_particles / 4;
        let mut counters = vec![];
        
        for _chunk_id in 0..4 {
            let processed = processed_clone.clone();
            let counter = ctx.spawn_job(move |_ctx| {
                // Simulate processing particles in this chunk
                for _ in 0..chunk_size {
                    processed.fetch_add(1, Ordering::SeqCst);
                    // Simulate particle update work (reduced for faster tests)
                    std::thread::sleep(Duration::from_micros(1));
                }
            });
            counters.push(counter);
        }
        
        // Wait for all chunks to complete
        for counter in counters {
            ctx.wait_for(&counter);
        }
    });
    
    job_system.wait_for_counter(&counter);
    assert_eq!(particles_processed.load(Ordering::SeqCst), num_particles);
    job_system.shutdown().expect("Shutdown failed");
}

#[test]
fn test_recursive_parallel_decomposition() {
    // Test recursive task decomposition with limited depth to avoid thread exhaustion
    let job_system = JobSystem::new(4);
    let result = Arc::new(AtomicUsize::new(0));
    
    let result_clone = result.clone();
    let counter = job_system.run_with_context(move |ctx| {
        // Two levels of decomposition only
        let mut counters = vec![];
        
        // Level 1: Split into 4 chunks
        for i in 0..4 {
            let result1 = result_clone.clone();
            let start = i * 25;
            let end = (i + 1) * 25;
            
            let child = ctx.spawn_job(move |_ctx| {
                // Level 2: Compute directly (no more spawning)
                let sum: usize = (start..end).sum();
                result1.fetch_add(sum, Ordering::SeqCst);
            });
            counters.push(child);
        }
        
        // Wait for all chunks
        for c in counters {
            ctx.wait_for(&c);
        }
    });
    
    job_system.wait_for_counter(&counter);
    
    // Sum of 0..100 is 4950
    let expected: usize = (0..100).sum();
    assert_eq!(result.load(Ordering::SeqCst), expected);
    job_system.shutdown().expect("Shutdown failed");
}

#[test]
fn test_producer_consumer_pattern() {
    // Test producer-consumer pattern where producers spawn work and consumers process it
    let job_system = JobSystem::new(4);
    let items_produced = Arc::new(AtomicUsize::new(0));
    let items_consumed = Arc::new(AtomicUsize::new(0));
    
    let produced = items_produced.clone();
    let consumed = items_consumed.clone();
    
    let counter = job_system.run_with_context(move |ctx| {
        // Producer spawns multiple consumer jobs
        let num_items = 20;
        let mut consumer_counters = vec![];
        
        for _ in 0..num_items {
            produced.fetch_add(1, Ordering::SeqCst);
            let consumed_clone = consumed.clone();
            
            let consumer = ctx.spawn_job(move |_ctx| {
                // Simulate consuming/processing the item (reduced for faster tests)
                std::thread::sleep(Duration::from_micros(10));
                consumed_clone.fetch_add(1, Ordering::SeqCst);
            });
            consumer_counters.push(consumer);
        }
        
        // Wait for all consumers to finish
        for counter in consumer_counters {
            ctx.wait_for(&counter);
        }
    });
    
    job_system.wait_for_counter(&counter);
    assert_eq!(items_produced.load(Ordering::SeqCst), 20);
    assert_eq!(items_consumed.load(Ordering::SeqCst), 20);
    job_system.shutdown().expect("Shutdown failed");
}

#[test]
fn test_cooperative_yielding() {
    // Test that long-running jobs can yield to allow other work to run
    let job_system = JobSystem::new(2);
    let iterations_completed = Arc::new(AtomicUsize::new(0));
    let other_job_ran = Arc::new(AtomicUsize::new(0));
    
    let iterations = iterations_completed.clone();
    let other = other_job_ran.clone();
    
    // Start a long-running job that yields periodically
    let long_counter = job_system.run_with_context(move |ctx| {
        for i in 0..100 {
            // Do some work
            iterations.fetch_add(1, Ordering::SeqCst);
            
            // Yield every 10 iterations to let other work run
            if i % 10 == 0 {
                ctx.yield_now();
            }
        }
    });
    
    // Start another job that should be able to run while the first yields
    let short_counter = job_system.run_with_context(move |_ctx| {
        other.fetch_add(1, Ordering::SeqCst);
    });
    
    job_system.wait_for_counter(&long_counter);
    job_system.wait_for_counter(&short_counter);
    
    assert_eq!(iterations_completed.load(Ordering::SeqCst), 100);
    assert_eq!(other_job_ran.load(Ordering::SeqCst), 1);
    job_system.shutdown().expect("Shutdown failed");
}

#[test]
fn test_hierarchical_job_tree() {
    // Test a multi-level job hierarchy where jobs spawn children that spawn grandchildren
    let job_system = JobSystem::new(4);
    let total_work = Arc::new(AtomicUsize::new(0));
    
    let work = total_work.clone();
    let counter = job_system.run_with_context(move |ctx| {
        // Level 1: Root spawns 3 children
        let mut child_counters = vec![];
        
        for _ in 0..3 {
            let work1 = work.clone();
            let child = ctx.spawn_job(move |ctx| {
                work1.fetch_add(1, Ordering::SeqCst);
                
                // Level 2: Each child spawns 2 grandchildren
                let mut grandchild_counters = vec![];
                for _ in 0..2 {
                    let work2 = work1.clone();
                    let grandchild = ctx.spawn_job(move |_ctx| {
                        work2.fetch_add(1, Ordering::SeqCst);
                    });
                    grandchild_counters.push(grandchild);
                }
                
                // Wait for grandchildren
                for gc in grandchild_counters {
                    ctx.wait_for(&gc);
                }
            });
            child_counters.push(child);
        }
        
        // Wait for all children
        for c in child_counters {
            ctx.wait_for(&c);
        }
    });
    
    job_system.wait_for_counter(&counter);
    
    // 3 children + (3 * 2) grandchildren = 9 total work units
    assert_eq!(total_work.load(Ordering::SeqCst), 9);
    job_system.shutdown().expect("Shutdown failed");
}

#[test]
fn test_backwards_compatibility_simple_jobs() {
    // Verify that simple jobs (without context) still work as before
    let job_system = JobSystem::new(4);
    let value = Arc::new(AtomicUsize::new(0));
    let value_clone = value.clone();
    
    let counter = job_system.run(move || {
        value_clone.store(42, Ordering::SeqCst);
    });
    
    job_system.wait_for_counter(&counter);
    assert_eq!(value.load(Ordering::SeqCst), 42);
    job_system.shutdown().expect("Shutdown failed");
}

#[test]
fn test_backwards_compatibility_multiple_jobs() {
    // Verify that run_multiple still works without context
    let job_system = JobSystem::new(4);
    let sum = Arc::new(AtomicUsize::new(0));
    
    let num_jobs = 50;
    let mut jobs: Vec<Box<dyn FnOnce() + Send>> = Vec::new();
    
    for i in 0..num_jobs {
        let sum_clone = sum.clone();
        jobs.push(Box::new(move || {
            sum_clone.fetch_add(i, Ordering::SeqCst);
        }));
    }
    
    let counter = job_system.run_multiple(jobs);
    job_system.wait_for_counter(&counter);
    
    let expected_sum: usize = (0..num_jobs).sum();
    assert_eq!(sum.load(Ordering::SeqCst), expected_sum);
    job_system.shutdown().expect("Shutdown failed");
}

#[test]
fn test_mixing_context_and_simple_jobs() {
    // Test that context-based and simple jobs can coexist
    let job_system = JobSystem::new(4);
    let context_job_ran = Arc::new(AtomicUsize::new(0));
    let simple_job_ran = Arc::new(AtomicUsize::new(0));
    
    let ctx_clone = context_job_ran.clone();
    let ctx_counter = job_system.run_with_context(move |_ctx| {
        ctx_clone.fetch_add(1, Ordering::SeqCst);
    });
    
    let simple_clone = simple_job_ran.clone();
    let simple_counter = job_system.run(move || {
        simple_clone.fetch_add(1, Ordering::SeqCst);
    });
    
    job_system.wait_for_counter(&ctx_counter);
    job_system.wait_for_counter(&simple_counter);
    
    assert_eq!(context_job_ran.load(Ordering::SeqCst), 1);
    assert_eq!(simple_job_ran.load(Ordering::SeqCst), 1);
    job_system.shutdown().expect("Shutdown failed");
}
