//! Transform hierarchy benchmark using criterion.
//!
//! Simulates game engine transform hierarchy updates.
//!
//! Created by Nguyen Phi Hung.

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use rustfiber::JobSystem;
use std::sync::Arc;

#[repr(C, align(64))]
#[derive(Clone)]
struct Transform {
    position: [f32; 3],
    rotation: [f32; 4],
    scale: [f32; 3],
}

fn bench_transform_hierarchy(c: &mut Criterion) {
    let num_threads = num_cpus::get();
    let system = JobSystem::for_gaming();

    // Warmup
    let warmup = system.parallel_for_chunked_auto(0..num_threads * 100, |_| {
        std::hint::black_box(());
    });
    system.wait_for_counter(&warmup);

    let mut group = c.benchmark_group("transform_hierarchy");
    group.sample_size(10);

    // Different hierarchy configurations
    for (depth, branching) in [(4, 4), (6, 3), (8, 2)] {
        let total_nodes: usize = (0..depth).map(|l| (branching as usize).pow(l)).sum();

        let transforms = Arc::new(vec![
            Transform {
                position: [0.0, 0.0, 0.0],
                rotation: [0.0, 0.0, 0.0, 1.0],
                scale: [1.0, 1.0, 1.0],
            };
            total_nodes
        ]);

        // Build parent indices
        let parent_indices = Arc::new({
            let mut parents = vec![None; total_nodes];
            let mut level_starts = vec![0usize; depth as usize];
            let mut current = 0;
            for lvl in 0..depth {
                level_starts[lvl as usize] = current;
                current += (branching as usize).pow(lvl);
            }

            let mut idx = 1;
            for lvl in 1..depth {
                let parent_start = level_starts[(lvl - 1) as usize];
                let num_parents = (branching as usize).pow(lvl - 1);
                for p in 0..num_parents {
                    for _ in 0..branching {
                        if idx < total_nodes {
                            parents[idx] = Some(parent_start + p);
                            idx += 1;
                        }
                    }
                }
            }
            parents
        });

        group.bench_function(
            BenchmarkId::new("hierarchy", format!("d{}_b{}", depth, branching)),
            |b| {
                b.iter(|| {
                    let t = transforms.clone();
                    let p = parent_indices.clone();

                    let counter = system.run_with_context(move |ctx| {
                        // Process levels in order (topological)
                        let mut level_starts = vec![0usize; depth as usize];
                        let mut current = 0;
                        for lvl in 0..depth {
                            level_starts[lvl as usize] = current;
                            current += (branching as usize).pow(lvl);
                        }

                        for lvl in 0..depth {
                            let start = level_starts[lvl as usize];
                            let count = (branching as usize).pow(lvl);
                            let end = (start + count).min(total_nodes);

                            let t_clone = t.clone();
                            let p_clone = p.clone();

                            let level_counter = ctx.parallel_for(start..end, 64, move |node_idx| {
                                if let Some(parent) = p_clone[node_idx] {
                                    let p_pos = t_clone[parent].position[0];
                                    std::hint::black_box(p_pos + 1.0);
                                }
                            });

                            ctx.wait_for(&level_counter);
                        }
                    });

                    system.wait_for_counter(&counter);
                })
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_transform_hierarchy);
criterion_main!(benches);
