//! Producer-consumer benchmark using criterion.
//!
//! Tests lock-free queue producer-consumer pattern.
//!
//! Created by Nguyen Phi Hung.

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use crossbeam::queue::SegQueue;
use rustfiber::JobSystem;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

fn bench_producer_consumer(c: &mut Criterion) {
    let num_threads = num_cpus::get();
    let system = JobSystem::new(num_threads);

    let mut group = c.benchmark_group("producer_consumer");
    group.sample_size(10);

    for num_items in [10_000, 100_000, 500_000] {
        group.throughput(Throughput::Elements(num_items as u64));

        group.bench_function(BenchmarkId::new("items", num_items), |b| {
            b.iter(|| {
                let queue: Arc<SegQueue<usize>> = Arc::new(SegQueue::new());
                let produced = Arc::new(AtomicUsize::new(0));
                let consumed = Arc::new(AtomicUsize::new(0));

                let num_producers = (num_threads / 2).max(1);
                let num_consumers = (num_threads / 2).max(1);
                let items_per_producer = num_items / num_producers;

                let mut jobs: Vec<Box<dyn FnOnce() + Send>> = Vec::new();

                // Producer jobs
                for i in 0..num_producers {
                    let q = queue.clone();
                    let p = produced.clone();
                    let my_items = if i == num_producers - 1 {
                        items_per_producer + (num_items % num_producers)
                    } else {
                        items_per_producer
                    };

                    jobs.push(Box::new(move || {
                        for j in 0..my_items {
                            q.push(j);
                        }
                        p.fetch_add(my_items, Ordering::SeqCst);
                    }));
                }

                // Consumer jobs
                for _ in 0..num_consumers {
                    let q = queue.clone();
                    let c = consumed.clone();
                    let target = num_items;

                    jobs.push(Box::new(move || {
                        while c.load(Ordering::SeqCst) < target {
                            if q.pop().is_some() {
                                c.fetch_add(1, Ordering::SeqCst);
                            } else {
                                std::thread::yield_now();
                            }
                        }
                    }));
                }

                let counter = system.run_multiple(jobs);
                system.wait_for_counter(&counter);

                std::hint::black_box(consumed.load(Ordering::SeqCst));
            })
        });
    }

    group.finish();
}

criterion_group!(benches, bench_producer_consumer);
criterion_main!(benches);
