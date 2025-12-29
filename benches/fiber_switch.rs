//! Fiber switch latency benchmark using criterion.
//!
//! Measures the raw context-switch cost using direct fiber APIs.
//! This bypasses all job system overhead (dispatch, parking, work-stealing).
//!
//! Created by Nguyen Phi Hung.

use criterion::{Criterion, criterion_group, criterion_main};
use rustfiber::JobSystem;

// Import internal fiber types for raw benchmarking
use rustfiber::fiber::{Fiber, FiberInput, FiberState};
use rustfiber::job::Job;

/// A minimal fiber harness for benchmarking raw context switches.
///
/// This bypasses all job system machinery (queues, work stealing, parking)
/// to measure the pure corosensei context-switch cost.
struct BenchFiber {
    fiber: Box<Fiber>,
}

impl BenchFiber {
    /// Creates a new benchmark fiber with the specified stack size.
    fn new(stack_size: usize) -> Self {
        Self {
            fiber: Box::new(Fiber::new(stack_size, false)),
        }
    }

    /// Creates a benchmark fiber with default stack size (128KB).
    fn default_stack() -> Self {
        Self::new(128 * 1024)
    }

    /// Execute a raw fiber switch: Caller → Fiber → Caller.
    #[inline(never)]
    fn execute<F: FnOnce() + Send + 'static>(&mut self, f: F) {
        // Use public Job::new() constructor
        let job = Job::new(f);

        struct NoOpScheduler;
        impl rustfiber::counter::JobScheduler for NoOpScheduler {
            fn schedule(&self, _job: Job) {}
        }
        static NOOP: NoOpScheduler = NoOpScheduler;
        let scheduler_ptr = &NOOP as &dyn rustfiber::counter::JobScheduler as *const _;

        let fiber_ptr = self.fiber.as_mut() as *mut Fiber;
        let input = FiberInput::Start(job, scheduler_ptr, fiber_ptr, None, None);

        let state = self.fiber.resume(input);

        debug_assert!(
            matches!(state, FiberState::Complete),
            "BenchFiber: expected Complete"
        );
    }
}

/// Benchmark raw fiber context switch using BenchFiber.
fn bench_raw_fiber_switch(c: &mut Criterion) {
    let mut fiber = BenchFiber::default_stack();

    // Warmup
    for _ in 0..1000 {
        fiber.execute(|| {});
    }

    c.bench_function("raw_fiber_switch", |b| {
        b.iter(|| {
            fiber.execute(std::hint::black_box(|| {}));
        })
    });
}

/// Benchmark fiber switch with minimal work inside.
fn bench_raw_fiber_with_work(c: &mut Criterion) {
    let mut fiber = BenchFiber::default_stack();

    // Warmup
    for _ in 0..1000 {
        fiber.execute(|| {
            std::hint::black_box(42);
        });
    }

    c.bench_function("raw_fiber_with_work", |b| {
        b.iter(|| {
            fiber.execute(|| {
                std::hint::black_box(42);
            });
        })
    });
}

/// Benchmark JobSystem cold path for comparison.
fn bench_job_system_cold(c: &mut Criterion) {
    let system = JobSystem::new(1);

    // Warmup
    for _ in 0..100 {
        let counter = system.run(|| {});
        system.wait_for_counter(&counter);
    }

    c.bench_function("job_system_cold", |b| {
        b.iter(|| {
            let counter = system.run(std::hint::black_box(|| {}));
            system.wait_for_counter(&counter);
        })
    });
}

criterion_group!(
    benches,
    bench_raw_fiber_switch,
    bench_raw_fiber_with_work,
    bench_job_system_cold
);
criterion_main!(benches);
