use criterion::{Criterion, criterion_group, criterion_main};
use rand::prelude::*;
use rayon::prelude::*;
use rustfiber::{JobSystem, ParallelSlice, ParallelSliceMut};
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Clone, Copy, Debug)]
struct Particle {
    x: f32, // World coordinates
    y: f32,
    vx: f32,
    vy: f32,
    active: bool,
}

// Simple Atomic Grid for "Bruteforce" collision checks
struct CollisionGrid {
    width: i32,
    height: i32,
    cells: Vec<AtomicBool>,
    offset_x: f32,
    offset_y: f32,
    cell_size: f32,
}

impl CollisionGrid {
    fn new(width: i32, height: i32, cell_size: f32) -> Self {
        // Center the grid around (0,0) somewhat
        let offset_x = -(width as f32 * cell_size) / 2.0;
        let offset_y = 0.0;

        // Initialize cells with false (empty)
        let mut cells = Vec::with_capacity((width * height) as usize);
        for _ in 0..(width * height) {
            cells.push(AtomicBool::new(false));
        }

        Self {
            width,
            height,
            cells,
            offset_x,
            offset_y,
            cell_size,
        }
    }

    fn clear_job_system(&self, job_system: &JobSystem) {
        self.cells.fiber_iter(job_system).for_each(|cell| {
            cell.store(false, Ordering::Relaxed);
        });
    }

    fn clear_rayon(&self) {
        self.cells.par_iter().for_each(|cell| {
            cell.store(false, Ordering::Relaxed);
        });
    }

    fn clear_serial(&self) {
        self.cells.iter().for_each(|cell| {
            cell.store(false, Ordering::Relaxed);
        });
    }

    fn get_index(&self, x: f32, y: f32) -> Option<usize> {
        let gx = ((x - self.offset_x) / self.cell_size) as i32;
        let gy = ((y - self.offset_y) / self.cell_size) as i32;

        if gx >= 0 && gx < self.width && gy >= 0 && gy < self.height {
            Some((gy * self.width + gx) as usize)
        } else {
            None
        }
    }

    fn create_walls(&self) {
        // Floor
        for x in 0..self.width {
            let idx = (0 * self.width + x) as usize;
            self.cells[idx].store(true, Ordering::Relaxed);
        }
        // Walls
        for y in 0..self.height {
            let left = (y * self.width + 0) as usize;
            let right = (y * self.width + (self.width - 1)) as usize;
            self.cells[left].store(true, Ordering::Relaxed);
            self.cells[right].store(true, Ordering::Relaxed);
        }
    }
}

struct World {
    particles: Vec<Particle>,
    active_count: usize,
    capacity: usize,
    grid: CollisionGrid,
    gravity: f32,
    dt: f32,
}

impl World {
    fn new(capacity: usize) -> Self {
        let grid_width = 1000; // 1000 cells wide
        let grid_height = 1000; // 1000 cells high
        let cell_size = 1.0; // 1 unit per cell

        let grid = CollisionGrid::new(grid_width, grid_height, cell_size);
        grid.create_walls();

        Self {
            particles: vec![
                Particle {
                    x: 0.0,
                    y: 0.0,
                    vx: 0.0,
                    vy: 0.0,
                    active: false
                };
                capacity
            ],
            active_count: 0,
            capacity,
            grid,
            gravity: -20.0, // Stronger gravity for snappy sand
            dt: 0.016,
        }
    }

    fn spawn(&mut self, count: usize, rng: &mut impl Rng) {
        let start = self.active_count;
        let end = (start + count).min(self.capacity);

        for i in start..end {
            self.particles[i] = Particle {
                x: rng.random_range(-100.0..100.0), // Spawn in center
                y: rng.random_range(800.0..950.0),  // Spawn high up
                vx: rng.random_range(-10.0..10.0),
                vy: rng.random_range(-10.0..0.0),
                active: true,
            };
        }
        self.active_count = end;
    }
}

// Logic for updating a single particle, extracted for reuse across backends
#[inline(always)]
fn update_particle(p: &mut Particle, grid: &CollisionGrid, dt: f32, gravity: f32, cell_size: f32) {
    if !p.active {
        return;
    }

    p.vy += gravity * dt;

    // Proposed new position
    let next_x = p.x + p.vx * dt;
    let next_y = p.y + p.vy * dt;

    // Check collision
    let curr_idx = grid.get_index(p.x, p.y);
    let next_idx = grid.get_index(next_x, next_y);
    let _idx_below = grid.get_index(p.x, p.y - cell_size);

    // Ground / Wall check via coordinates
    if next_y <= 0.0 {
        // Hit floor
        p.y = 0.0;
        p.vy = 0.0;
        p.vx *= 0.5; // Friction
    } else if let Some(n_idx) = next_idx {
        // Self-check
        let self_check = if let Some(c_idx) = curr_idx {
            c_idx == n_idx
        } else {
            false
        };

        if !self_check && grid.cells[n_idx].load(Ordering::Relaxed) {
            // Collision! Try sliding
            let try_left = grid.get_index(p.x - cell_size, p.y - cell_size);
            let try_right = grid.get_index(p.x + cell_size, p.y - cell_size);

            let free_left =
                try_left.is_some() && !grid.cells[try_left.unwrap()].load(Ordering::Relaxed);
            let free_right =
                try_right.is_some() && !grid.cells[try_right.unwrap()].load(Ordering::Relaxed);

            // Deterministic randomish choice for sliding to avoid bias, but keep it simple
            // We use a simple hash of position for determinism if we wanted, but here rand::random is hard
            // inside parallel loop without passing rng. Let's just prefer left then right.
            // Or use particle address/index? We don't have index here.
            // Let's just alternate based on y coordinate parity to avoid strict bias 'falling left'
            let prefer_left = (p.y as i32) % 2 == 0;

            if free_left && (prefer_left || !free_right) {
                // Slide left
                p.x -= cell_size;
                p.y -= cell_size;
                p.vx *= 0.8;
            } else if free_right {
                // Slide right
                p.x += cell_size;
                p.y -= cell_size;
                p.vx *= 0.8;
            } else {
                // Stuck
                p.vx = 0.0;
                p.vy = 0.0;
            }
        } else {
            // Free to move
            p.x = next_x;
            p.y = next_y;
        }
    } else {
        // Out of bounds cleanup
        p.x = next_x.clamp(-400.0, 400.0);
        p.y = next_y.max(0.0);
    }
}

enum Backend<'a> {
    JobSystem(&'a JobSystem),
    Rayon,
    Serial,
}

fn run_simulation_step(world: &mut World, backend: Backend, rng: &mut impl Rng) {
    if world.active_count < world.capacity {
        world.spawn(2000, rng);
    }

    // Phase 1: Clear Grid
    match backend {
        Backend::JobSystem(js) => world.grid.clear_job_system(js),
        Backend::Rayon => world.grid.clear_rayon(),
        Backend::Serial => world.grid.clear_serial(),
    }

    // Phase 2: Populate Collision Map
    let active_slice = &mut world.particles[0..world.active_count];
    let grid_ref = &world.grid;

    // Helper closure for population
    let populate_op = |p: &mut Particle| {
        if p.active {
            if let Some(idx) = grid_ref.get_index(p.x, p.y) {
                grid_ref.cells[idx].store(true, Ordering::Relaxed);
            }
        }
    };

    match backend {
        Backend::JobSystem(js) => {
            active_slice.fiber_iter_mut(js).for_each(populate_op);
        }
        Backend::Rayon => {
            active_slice.par_iter_mut().for_each(populate_op);
        }
        Backend::Serial => {
            active_slice.iter_mut().for_each(populate_op);
        }
    }

    // Phase 3: Update Particles
    let dt = world.dt;
    let gravity = world.gravity;
    let cell_size = world.grid.cell_size;

    let update_op = |p: &mut Particle| {
        update_particle(p, grid_ref, dt, gravity, cell_size);
    };

    match backend {
        Backend::JobSystem(js) => {
            active_slice.fiber_iter_mut(js).for_each(update_op);
        }
        Backend::Rayon => {
            active_slice.par_iter_mut().for_each(update_op);
        }
        Backend::Serial => {
            active_slice.iter_mut().for_each(update_op);
        }
    }
}

fn bench_sand_simulation(c: &mut Criterion) {
    let job_system = JobSystem::default();
    let mut rng = rand::rng();

    let mut group = c.benchmark_group("simulation_backend_compare");
    group.sample_size(10);

    // 1. RustFiber JobSystem
    group.bench_function("sand_100k_job_system", |b| {
        b.iter_with_setup(
            || World::new(100_000),
            |mut world| {
                for _ in 0..120 {
                    run_simulation_step(&mut world, Backend::JobSystem(&job_system), &mut rng);
                }
            },
        );
    });

    // 2. Rayon
    group.bench_function("sand_100k_rayon", |b| {
        b.iter_with_setup(
            || World::new(100_000),
            |mut world| {
                for _ in 0..120 {
                    run_simulation_step(&mut world, Backend::Rayon, &mut rng);
                }
            },
        );
    });

    // 3. Serial (Standard Iterator)
    group.bench_function("sand_100k_serial", |b| {
        b.iter_with_setup(
            || World::new(100_000),
            |mut world| {
                for _ in 0..120 {
                    run_simulation_step(&mut world, Backend::Serial, &mut rng);
                }
            },
        );
    });

    group.finish();
}

// Heavy compute workload (simulating complex aerodynamics/thermodynamics)
// "Best Case" for Rayon: Pure independent compute, minimal memory contention.
#[inline(never)]
fn heavy_physics_update(p: &mut Particle) {
    if !p.active {
        return;
    }

    // Perform some arbitrary heavy math
    let mut v = p.vx + p.vy;
    for _ in 0..200 {
        v = v.sin() + v.cos();
    }
    // Write back to prevent optimization
    p.vx += v * 0.000001;
}

fn run_heavy_physics_step(world: &mut World, backend: Backend, rng: &mut impl Rng) {
    if world.active_count < world.capacity {
        world.spawn(2000, rng);
    }

    let active_slice = &mut world.particles[0..world.active_count];

    // Single Phase: Pure Parallel Update
    match backend {
        Backend::JobSystem(js) => {
            // Use chunking for fair comparison with Rayon on heavy workloads?
            // fiber_iter_mut uses chunking by default (ParallelSliceMut).
            active_slice
                .fiber_iter_mut(js)
                .for_each(heavy_physics_update);
        }
        Backend::Rayon => {
            active_slice.par_iter_mut().for_each(heavy_physics_update);
        }
        Backend::Serial => {
            active_slice.iter_mut().for_each(heavy_physics_update);
        }
    }
}

fn bench_heavy_physics(c: &mut Criterion) {
    let job_system = JobSystem::default();
    let mut rng = rand::rng();

    let mut group = c.benchmark_group("physics_compute_compare");
    group.sample_size(10);

    // We use fewer frames/particles because it's heavy
    let items = 100_000;

    group.bench_function("physics_100k_job_system", |b| {
        b.iter_with_setup(
            || World::new(items),
            |mut world| {
                for _ in 0..10 {
                    // 10 frames of heavy compute
                    run_heavy_physics_step(&mut world, Backend::JobSystem(&job_system), &mut rng);
                }
            },
        );
    });

    group.bench_function("physics_100k_rayon", |b| {
        b.iter_with_setup(
            || World::new(items),
            |mut world| {
                for _ in 0..10 {
                    run_heavy_physics_step(&mut world, Backend::Rayon, &mut rng);
                }
            },
        );
    });

    group.bench_function("physics_100k_serial", |b| {
        b.iter_with_setup(
            || World::new(items),
            |mut world| {
                for _ in 0..10 {
                    run_heavy_physics_step(&mut world, Backend::Serial, &mut rng);
                }
            },
        );
    });

    group.finish();
}

fn bench_sand_1m(c: &mut Criterion) {
    let job_system = JobSystem::default();
    let mut rng = rand::rng();

    let mut group = c.benchmark_group("sand_1m_compare");
    group.sample_size(10);

    let items = 1_000_000;

    group.bench_function("sand_1m_job_system", |b| {
        b.iter_with_setup(
            || {
                let mut w = World::new(items);
                w.spawn(items, &mut rand::rng());
                w
            },
            |mut world| {
                for _ in 0..10 {
                    run_simulation_step(&mut world, Backend::JobSystem(&job_system), &mut rng);
                }
            },
        );
    });

    group.bench_function("sand_1m_rayon", |b| {
        b.iter_with_setup(
            || {
                let mut w = World::new(items);
                w.spawn(items, &mut rand::rng());
                w
            },
            |mut world| {
                for _ in 0..10 {
                    run_simulation_step(&mut world, Backend::Rayon, &mut rng);
                }
            },
        );
    });

    group.finish();
}

fn bench_physics_1m(c: &mut Criterion) {
    let job_system = JobSystem::default();
    let mut rng = rand::rng();

    let mut group = c.benchmark_group("physics_1m_compare");
    group.sample_size(10);

    let items = 1_000_000;

    group.bench_function("physics_1m_job_system", |b| {
        b.iter_with_setup(
            || {
                let mut w = World::new(items);
                w.spawn(items, &mut rand::rng());
                w
            },
            |mut world| {
                for _ in 0..10 {
                    run_heavy_physics_step(&mut world, Backend::JobSystem(&job_system), &mut rng);
                }
            },
        );
    });

    group.bench_function("physics_1m_rayon", |b| {
        b.iter_with_setup(
            || {
                let mut w = World::new(items);
                w.spawn(items, &mut rand::rng());
                w
            },
            |mut world| {
                for _ in 0..10 {
                    run_heavy_physics_step(&mut world, Backend::Rayon, &mut rng);
                }
            },
        );
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_sand_simulation,
    bench_heavy_physics,
    bench_sand_1m,
    bench_physics_1m
);
criterion_main!(benches);
