# Design: Low-Overhead Spawning

## API Design

### `spawn_detached`
```rust
pub fn spawn_detached<F>(&self, work: F)
where
    F: FnOnce(&Context) + Send + 'static
{
    // ... logic similar to spawn_job but passing None for counter ...
}
```
This is straightforward. The `Job` struct already stores `Option<Counter>`.

### `spawn_group` vs `spawn_with_counter`
The strategy suggested `spawn_group`. We can implement this by allowing the user to pass an existing `Counter` reference.
However, `Job` owns the `Counter`.
If we pass `&Counter`, we must clone it (increment Arc).
Arc increment is atomic (cheap-ish) but not zero.
It is still much cheaper than `Arc::new` (malloc).

```rust
pub fn spawn_with_counter<F>(&self, work: F, counter: Counter)
where ...
{
    // ... use provided counter ...
}
```
Usage:
```rust
let group = Counter::new(50_000);
for _ in 0..50_000 {
    ctx.spawn_with_counter(work, group.clone());
}
ctx.wait_for(&group);
```
Wait, `Counter::new(k)` means it waits for `k` decrements.
If we spawn 50k jobs, we need `Counter::new(50_000)`.
Currently `Counter::new` takes `initial_value`.

If `Context::spawn_job` creates `Counter::new(1)`, it returns it.
The user waits on that specific job.

For groups, the user creates the counter *once*.

## Allocator Interaction
Both new methods must utilize `spawn_job`'s logic for:
1.  Checking `self.allocator` (Frame Alloc).
2.  Checking `self.local_queue` (Local Push).

We should refactor the core logic of `spawn_job` into a private helper `spawn_internal(work, counter: Option<Counter>)`.
