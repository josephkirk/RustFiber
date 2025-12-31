# Design: Parallel For Batching

## Problem
Currently, to run minimal tasks (e.g., incrementing an array index), the user must spawn a job for each index.
```rust
for i in 0..1_000_000 {
    system.run(move || process(i));
}
```
This creates 1,000,000 `Job` structs, 1,000,000 `FnOnce` allocations (if boxed/captured), and 1,000,000 queue pushes.

## Solution: Chunking
We implement a `parallel_for` pattern that inverts the loop control.
```rust
system.parallel_for(0..1_000_000, 1000, |i| process(i));
```
Internally, this creates `1_000_000 / 1000 = 1000` jobs.
Each job looks like:
```rust
let start = chunk.start;
let end = chunk.end;
// A copy of the user's closure
let body = body.clone(); 
move || {
    for i in start..end {
        body(i);
    }
}
```

## Closure Requirements
The user's closure `F` must be:
- `Fn(usize)` or `FnMut(usize)`? `Fn` is safer assuming parallel execution implies shared state or independent mutation via interior mutability. However, since the closure is cloned into each job, and jobs run in parallel, if it was `FnMut`, it would need to be `Clone` and the mutation would be local to the chunk (copy). This might be unexpected.
- Standard restriction: `F: Fn(usize) + Sync + Send + Clone + 'static`.
- Using `Clone` allows us to distribute the logic to multiple jobs without wrapping it in an `Arc` (avoiding indirection if the closure is small).

## API Design

### JobSystem
```rust
pub fn parallel_for<F>(&self, range: Range<usize>, batch_size: usize, body: F) -> Counter
where F: Fn(usize) + Send + Sync + Clone + 'static
```

### Context
```rust
pub fn parallel_for<F>(&self, range: Range<usize>, batch_size: usize, body: F) -> Counter
where F: Fn(usize) + Send + Sync + Clone + 'static
```

## Batch Size Calculation
We can also offer an auto-batching variant:
```rust
pub fn parallel_for_auto<F>(&self, range: Range<usize>, body: F) -> Counter
```
This would calculate `batch_size = len / (num_workers * N)` where N is a factor (e.g., 4) to ensure some load balancing (granularity).

## Implementation Details
1. Calculate number of chunks.
2. Generate iterator of jobs.
3. Use `run_multiple` (or internal equivalent) to submit them efficiently.
