use criterion::{Criterion, criterion_group, criterion_main};
use rand::prelude::*;
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

    fn clear(&self, job_system: &JobSystem) {
        // Parallel clear using the job system
        self.cells.par_iter(job_system).for_each(|cell| {
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

fn run_simulation_step(world: &mut World, job_system: &JobSystem, rng: &mut impl Rng) {
    if world.active_count < world.capacity {
        world.spawn(2000, rng); // Faster spawn to fill up quickly
    }

    // Phase 1: Clear the dynamic part of the grid (keep walls)
    // For benchmark simplicity, we clear everything and re-add walls, or selective clear.
    // Let's do a full clear + rebuild walls for consistent load.
    world.grid.clear(job_system);

    // Re-add static walls (fast enough to do serial or parallel_for)
    // Actually, let's just not clear the walls.
    // Optimization: Only clear used cells? No, we need O(Grid) or O(N) clear.
    // Atomic clear of 1M bools is fast.

    // Phase 2: Populate Grid with PREVIOUS positions (Collision Map)
    // This allows particles to "see" where others are currently.
    let active_slice = &mut world.particles[0..world.active_count];

    active_slice.par_iter_mut(job_system).for_each(|p| {
        if !p.active {
            return;
        }
        // Simple bounding to keep in grid
        if let Some(idx) = world.grid.get_index(p.x, p.y) {
            world.grid.cells[idx].store(true, Ordering::Relaxed);
        }
    });

    // Re-enforce floor (in case a particle overwrote it? No, store(true) matches).
    // But we cleared it. So we need to ensure floor is 'true'.
    // Better: Populate walls *after* clearing.
    // world.grid.create_walls(); // (Serial for now, effectively negligible compared to 100k particles)
    // Let's do it inside the "Update" phase via boundary checks instead of wall cells.
    // Or just treat y=0 as floor.

    let dt = world.dt;
    let gravity = world.gravity;
    let cell_size = world.grid.cell_size;

    // Phase 3: Update Particles
    active_slice.par_iter_mut(job_system).for_each(|p| {
        if !p.active {
            return;
        }

        p.vy += gravity * dt;

        // Proposed new position
        let next_x = p.x + p.vx * dt;
        let next_y = p.y + p.vy * dt;

        // Check collision at next_x, next_y
        // We check grid cell.
        let curr_idx = world.grid.get_index(p.x, p.y);
        let next_idx = world.grid.get_index(next_x, next_y);
        let _idx_below = world.grid.get_index(p.x, p.y - cell_size); // Strictly below current

        // Ground / Wall check via coordinates
        if next_y <= 0.0 {
            // Hit floor
            p.y = 0.0;
            p.vy = 0.0;
            p.vx *= 0.5; // Friction
        } else if let Some(n_idx) = next_idx {
            // Check if target cell is occupied
            // Note: We populated grid with CURRENT positions.
            // So if we move into a cell that currently holds a particle, we collide.
            // This prevents moving *through* people.

            // Self-check: if next_idx == curr_idx, we are just staying in same cell (or moving sub-pixel). Safe.
            let self_check = if let Some(c_idx) = curr_idx {
                c_idx == n_idx
            } else {
                false
            };

            if !self_check && world.grid.cells[n_idx].load(Ordering::Relaxed) {
                // Collision!
                // Try sliding (Sand behavior)
                // Check Down-Left and Down-Right relative to *Current* position (stacking logic)

                let try_left = world.grid.get_index(p.x - cell_size, p.y - cell_size);
                let try_right = world.grid.get_index(p.x + cell_size, p.y - cell_size);

                let free_left = try_left.is_some()
                    && !world.grid.cells[try_left.unwrap()].load(Ordering::Relaxed);
                let free_right = try_right.is_some()
                    && !world.grid.cells[try_right.unwrap()].load(Ordering::Relaxed);

                if free_left && (!free_right || rand::random::<bool>()) {
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
                    // Stuck / Stacked
                    p.vx = 0.0;
                    p.vy = 0.0;
                    // Keep old position (don't apply next_x/y)
                }
            } else {
                // Free to move
                p.x = next_x;
                p.y = next_y;
            }
        } else {
            // Out of bounds?
            p.x = next_x.clamp(-400.0, 400.0);
            p.y = next_y.max(0.0);
        }
    });
}

fn bench_sand_simulation(c: &mut Criterion) {
    let job_system = JobSystem::default(); // Uses num_cpus
    let mut rng = rand::rng();

    let mut group = c.benchmark_group("simulation");
    group.sample_size(10); // Simulation is heavy, take fewer samples

    group.bench_function("sand_100k_progressive", |b| {
        b.iter_with_setup(
            || World::new(100_000),
            |mut world| {
                // Simulate 120 frames.
                // 0 -> 100k particles takes 100 frames (1000 per frame).
                // We run 120 frames to include some full-load frames.
                for _ in 0..120 {
                    run_simulation_step(&mut world, &job_system, &mut rng);
                }
            },
        );
    });

    group.finish();
}

criterion_group!(benches, bench_sand_simulation);
criterion_main!(benches);
