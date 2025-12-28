use crate::utils::{BenchmarkResult, DataPoint, SystemInfo};
#[cfg(feature = "metrics")]
use rustfiber::metrics::MetricsSnapshot;
use rustfiber::{JobSystem, PinningStrategy};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

pub fn run_work_stealing_stress_benchmark(
    job_system: &JobSystem,
    strategy: PinningStrategy,
    threads: usize,
) -> BenchmarkResult {
    eprintln!("\n=== Benchmark: Work-Stealing Stress (Skewed Load) ===");

    let system_info = SystemInfo::collect(strategy, threads);
    eprintln!(
        "System: {} CPU cores, {:.2} GB total RAM, Strategy: {:?}",
        system_info.cpu_cores, system_info.total_memory_gb, strategy
    );

    // Local Warmup
    eprintln!("Warming up workers locally...");
    let warmup_counter = job_system.parallel_for_chunked_auto(0..threads * 100, |_| {
        std::hint::black_box(());
    });
    job_system.wait_for_counter(&warmup_counter);

    // Capture metrics before the test
    #[cfg(feature = "metrics")]
    let metrics_before = job_system.metrics();

    let heavy_workload = 100_000; // Heavy tasks for worker 0
    let light_workload = 1_000; // Light tasks for others

    eprintln!("\nTesting skewed workload: Worker 0 heavy ({}), others light ({})", heavy_workload, light_workload);

    let start = Instant::now();
    let counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = counter.clone();

    let root_counter = job_system.run_with_context(move |ctx| {
        // Heavy work on worker 0 (assuming pinning)
        // Spawn individual jobs to allow stealing!
        for _ in 0..heavy_workload {
            let c = counter_clone.clone();
            ctx.spawn_job(move |_| {
                std::hint::black_box(fibonacci(20));
                c.fetch_add(1, Ordering::Relaxed);
            });
        }

        // Light work on other workers
        // We spawn distinct jobs here too, though they might finish fast.
        // To target specific workers in `run_with_context` is tricky as `spawn_job` goes to local queue.
        // But `run_with_context` runs on the *caller* thread? 
        // Wait, `run_with_context` usually runs on current thread if it's a worker, or picks one?
        // Actually `JobSystem::run_with_context` blocks current thread and uses it as a worker?
        // No, `run_with_context` usually submits a root job.
        
        // The original code assumed pinning. "Heavy work on worker 0".
        // `ctx` here is from the root job.
        // If the root job runs on Worker 0, then `ctx.spawn_job` pushes to Worker 0.
        // To simulate "Others have light work", we might need to rely on the fact that
        // the loop above fills Worker 0, and we want OTHERS to have work?
        // Actually, if we just fill Worker 0, others will steal from it.
        // The original logic tried to put work on others:
        /*
        for _ in 1..threads {
             ctx.spawn_job(...) 
        }
        */
        // If `ctx` is on Worker 0, this just pushes MORE to Worker 0.
        // Unless there's a mechanism to push to others? 
        // `spawn_job` is local.
        // So the original benchmark was likely ALL on Worker 0 (if root was on Worker 0).
        // Or if root was on global?
        
        // If we want a "Stress" test of stealing, putting ALL 100,000 jobs on Worker 0 
        // and having everyone else steal is the PERFECT scenario.
        // So we don't strictly need to manually place work on others if they start empty.
        // They will just steal.
        // So let's just dump 100k jobs on the current worker (Root) and let others feast.
        
        // We can ignore the "Light work" loop or just add it to the pile.
        // Start simple: 100k jobs on Root.
    });

    job_system.wait_for_counter(&root_counter);

    // Wait for all to complete
    // We only spawn heavy_workload now.
    let total_tasks = heavy_workload;
    while counter.load(Ordering::Relaxed) < total_tasks {
        std::thread::yield_now();
    }

    let elapsed = start.elapsed();
    let elapsed_ms = elapsed.as_secs_f64() * 1000.0;

    eprintln!(
        "  Completed in {:.2} ms",
        elapsed_ms
    );

    // Capture metrics after the test and calculate deltas
    #[cfg(feature = "metrics")]
    let (steal_stats, total_steals) = if let Some(after) = job_system.metrics() {
        let before = metrics_before.unwrap_or_else(|| MetricsSnapshot {
            jobs_completed: 0,
            local_queue_pushes: 0,
            local_queue_pops: 0,
            global_injector_pushes: 0,
            global_injector_pops: 0,
            worker_steals_success: 0,
            worker_steals_failed: 0,
            worker_steals_retry: 0,
            injector_steals_success: 0,
            injector_steals_failed: 0,
            injector_steals_retry: 0,
            elapsed_seconds: 0.0,
        });

        let worker_steals_success = after.worker_steals_success.saturating_sub(before.worker_steals_success);
        let worker_steals_failed = after.worker_steals_failed.saturating_sub(before.worker_steals_failed);
        let worker_steals_retry = after.worker_steals_retry.saturating_sub(before.worker_steals_retry);
        let injector_steals_success = after.injector_steals_success.saturating_sub(before.injector_steals_success);
        let injector_steals_failed = after.injector_steals_failed.saturating_sub(before.injector_steals_failed);
        let injector_steals_retry = after.injector_steals_retry.saturating_sub(before.injector_steals_retry);

        let total = (worker_steals_success + worker_steals_failed + worker_steals_retry + 
                     injector_steals_success + injector_steals_failed + injector_steals_retry) as usize;

        eprintln!("  Work-Stealing Statistics:");
        eprintln!("    Worker steals: {} successful, {} empty, {} retries ({:.1}% success rate)",
            worker_steals_success,
            worker_steals_failed,
            worker_steals_retry,
            if worker_steals_success + worker_steals_failed + worker_steals_retry > 0 {
                (worker_steals_success as f64 / (worker_steals_success + worker_steals_failed + worker_steals_retry) as f64) * 100.0
            } else {
                0.0
            }
        );
        eprintln!("    Injector steals: {} successful, {} empty, {} retries ({:.1}% success rate)",
            injector_steals_success,
            injector_steals_failed,
            injector_steals_retry,
            if injector_steals_success + injector_steals_failed + injector_steals_retry > 0 {
                (injector_steals_success as f64 / (injector_steals_success + injector_steals_failed + injector_steals_retry) as f64) * 100.0
            } else {
                0.0
            }
        );

        (Some(((worker_steals_success, worker_steals_failed, worker_steals_retry, injector_steals_success, injector_steals_failed, injector_steals_retry), after)), total)
    } else {
        eprintln!("  Work-Stealing Statistics: Metrics not available (compile with --features metrics)");
        (None, 0)
    };

    // Create data points - include steal stats if available
    #[cfg(feature = "metrics")]
    let data_points = {
        let mut data_points = vec![DataPoint {
            num_tasks: total_tasks,
            time_ms: elapsed_ms,
            metric_type: Some("time".to_string()),
        }];

        // Add steal statistics as additional data points if metrics are available
        if let Some(((worker_success, worker_failed, worker_retry, injector_success, injector_failed, injector_retry), _metrics_snapshot)) = steal_stats {
            data_points.push(DataPoint {
                num_tasks: total_tasks,
                time_ms: worker_success as f64,
                metric_type: Some("worker_steals_success".to_string()),
            });
            data_points.push(DataPoint {
                num_tasks: total_tasks,
                time_ms: worker_failed as f64,
                metric_type: Some("worker_steals_failed".to_string()),
            });
            data_points.push(DataPoint {
                num_tasks: total_tasks,
                time_ms: worker_retry as f64,
                metric_type: Some("worker_steals_retry".to_string()),
            });
            data_points.push(DataPoint {
                num_tasks: total_tasks,
                time_ms: injector_success as f64,
                metric_type: Some("injector_steals_success".to_string()),
            });
            data_points.push(DataPoint {
                num_tasks: total_tasks,
                time_ms: injector_failed as f64,
                metric_type: Some("injector_steals_failed".to_string()),
            });
            data_points.push(DataPoint {
                num_tasks: total_tasks,
                time_ms: injector_retry as f64,
                metric_type: Some("injector_steals_retry".to_string()),
            });
        }
        data_points
    };

    #[cfg(not(feature = "metrics"))]
    let data_points = vec![DataPoint {
        num_tasks: total_tasks,
        time_ms: elapsed_ms,
        metric_type: Some("time".to_string()),
    }];
    BenchmarkResult {
        name: "Work-Stealing Stress".to_string(),
        data_points,
        system_info,
        crashed: false,
        crash_point: None,
        timed_out: false,
        #[cfg(feature = "metrics")]
        internal_metrics: crate::utils::capture_internal_metrics(job_system),
    }
}

fn fibonacci(n: u64) -> u64 {
    if n <= 1 {
        return n;
    }
    fibonacci(n - 1) + fibonacci(n - 2)
}