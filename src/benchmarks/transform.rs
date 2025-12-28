use crate::utils::{BenchmarkResult, DataPoint, SystemInfo};
use rustfiber::{JobSystem, PinningStrategy};
use std::sync::Arc;
use std::time::Instant;

#[repr(C, align(64))]
#[derive(Clone)]
struct Transform {
    position: [f32; 3],
    rotation: [f32; 4], // quaternion
    scale: [f32; 3],
}

pub fn run_transform_hierarchy_benchmark(
    job_system: &JobSystem,
    strategy: PinningStrategy,
    threads: usize,
) -> BenchmarkResult {
    eprintln!("\n=== Benchmark: Transform Hierarchy Update ===");

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

    let hierarchy_depth = 8; // Increased depth for more meaningful scaling
    let nodes_per_level: u32 = 4; // Smaller branching factor to make it deeper
    let total_nodes = (0..hierarchy_depth).map(|level| nodes_per_level.pow(level)).sum::<u32>() as usize;

    eprintln!("\nSimulating transform hierarchy: {} levels, {} nodes per level, total {} nodes", hierarchy_depth, nodes_per_level, total_nodes);

    // Build hierarchy: flat array with parent indices
    // We use a Vec of Transforms, but we should probably use a Mutex or UnsafeCell for concurrent updates if we were doing real work.
    // However, to keep it simple and focus on scheduling:
    let transforms = Arc::new(vec![Transform {
        position: [0.0, 0.0, 0.0],
        rotation: [0.0, 0.0, 0.0, 1.0],
        scale: [1.0, 1.0, 1.0],
    }; total_nodes]);
    
    let parent_indices = Arc::new({
        let mut parent_indices = vec![None; total_nodes];
        let mut level_start = vec![0; hierarchy_depth as usize];
        let mut current = 0;
        for level in 0..hierarchy_depth {
            level_start[level as usize] = current;
            current += nodes_per_level.pow(level) as usize;
        }
        let mut current_index = 1;
        for level in 1..hierarchy_depth {
            let parent_start = level_start[(level - 1) as usize];
            let num_parents = nodes_per_level.pow(level - 1) as usize;
            for parent_idx in 0..num_parents {
                let parent = parent_start + parent_idx;
                for _ in 0..nodes_per_level {
                    if current_index < total_nodes {
                        parent_indices[current_index] = Some(parent);
                        current_index += 1;
                    }
                }
            }
        }
        parent_indices
    });

    let test_sizes = vec![1, 10, 100]; // Number of full hierarchy updates

    let mut data_points = Vec::new();
    let mut timed_out = false;
    let total_start = Instant::now();
    let timeout_duration = std::time::Duration::from_secs(crate::utils::DEFAULT_TIMEOUT_SECS);

    for &num_updates in &test_sizes {
        if total_start.elapsed() > timeout_duration {
            eprintln!(
                "\n! Timeout reached ({}s), stopping benchmark.",
                crate::utils::DEFAULT_TIMEOUT_SECS
            );
            timed_out = true;
            break;
        }

        eprintln!("\nTesting with {} hierarchy updates...", num_updates);

        let start = Instant::now();

        for _ in 0..num_updates {
            let transforms_clone = transforms.clone();
            let parent_indices_clone = parent_indices.clone();
            
            // Outer job to manage the whole hierarchy update
            let update_counter = job_system.run_with_context(move |ctx| {
                let mut level_starts = vec![0; hierarchy_depth as usize];
                let mut current = 0;
                for lvl in 0..hierarchy_depth {
                    level_starts[lvl as usize] = current;
                    current += nodes_per_level.pow(lvl) as usize;
                }

                // Process in topological order (levels): root to leaves
                for level in 0..hierarchy_depth {
                    let level_start = level_starts[level as usize];
                    let level_count = nodes_per_level.pow(level) as usize;
                    let level_end = (level_start + level_count).min(total_nodes);
                    
                    if level_start >= level_end { break; }

                    let p_indices = parent_indices_clone.clone();
                    let t_buffer = transforms_clone.clone();

                    // Process level in parallel
                    // Use parallel_for to avoid overhead of too many jobs
                    let level_counter = ctx.parallel_for(level_start..level_end, 64, move |node_idx| {
                        if let Some(parent) = p_indices[node_idx] {
                            // Update transform based on parent
                            // In real scenario, matrix multiplication.
                            // We do some work to simulate it.
                            let p_pos = t_buffer[parent].position[0];
                            std::hint::black_box(p_pos + 1.0);
                        }
                    });

                    // CRITICAL: Wait for current level to finish before starting next level
                    ctx.wait_for(&level_counter);
                }
            });
            job_system.wait_for_counter(&update_counter);
        }

        let elapsed = start.elapsed();
        let elapsed_ms = elapsed.as_secs_f64() * 1000.0;

        eprintln!(
            "  Completed in {:.2} ms ({:.2} ms per update)",
            elapsed_ms,
            elapsed_ms / num_updates as f64
        );

        data_points.push(DataPoint {
            num_tasks: num_updates,
            time_ms: elapsed_ms,
            metric_type: None,
        });
    }

    BenchmarkResult {
        name: "Transform Hierarchy".to_string(),
        data_points,
        system_info,
        crashed: false,
        crash_point: None,
        timed_out,
        #[cfg(feature = "metrics")]
        internal_metrics: crate::utils::capture_internal_metrics(job_system),
    }
}