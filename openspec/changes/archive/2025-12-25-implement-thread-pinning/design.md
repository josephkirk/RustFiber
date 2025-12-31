# Design: Thread Pinning Strategies

## Overview
The goal is to provide finer control over worker thread placement. Currently, pinning is a simple 1:1 mapping of worker ID to `CoreId` index. We will replace this with a strategy-based approach.

## Components

### 1. `PinningStrategy` Enum
```rust
pub enum PinningStrategy {
    /// No pinning (standard OS scheduling)
    None,
    /// Linear pinning (worker i -> core i)
    Linear,
    /// Pin to physical cores only (even-numbered logical processors)
    AvoidSMT,
    /// Pin to physical cores on the first CCD only (Logical 0, 2, ..., 14)
    CCDIsolation,
}
```

### 2. Core Mapper
A helper function will resolve the `PinningStrategy` into a list of available `CoreId`s.

- `Linear`: `core_ids[0..num_threads]`
- `AvoidSMT`: `core_ids.iter().step_by(2).take(num_threads)`
- `CCDIsolation`: `core_ids.iter().step_by(2).take(8).take(num_threads)`

### 3. Worker Initialization
`WorkerPool` will compute the core mapping during construction and pass the specific `CoreId` to each `Worker`.

```rust
// In WorkerPool::new_with_strategy(num_threads, strategy)
let core_ids = core_affinity::get_core_ids().unwrap_or_default();
let mapped_cores = match strategy {
    PinningStrategy::Linear => core_ids.into_iter().collect(),
    PinningStrategy::AvoidSMT => core_ids.into_iter().step_by(2).collect(),
    PinningStrategy::CCDIsolation => core_ids.into_iter().step_by(2).take(8).collect(),
    _ => vec![],
};

// Spawn workers
for id in 0..num_threads {
    let core_id = mapped_cores.get(id).copied();
    Worker::new(id, ..., core_id);
}
```

## Architectural Decisons
### Strategy Precedence
`CCDIsolation` is the most restrictive and essentially a subset of `AvoidSMT` restricted to the first CCD die. 

### Fallback Behavior
If the system has fewer cores than the requested strategy requires (e.g., requesting 16 workers on `CCDIsolation` which only provides 8 slots), the system will:
1.  Fill all slots provided by the strategy.
2.  Warn (or handle gracefully) if worker count > available slots for that strategy. 
    - Decision: We will wrap around or disable pinning for overflowing workers to ensure the system still runs.

### Thread Safety
Core pinning is performed within the worker's thread closure using `core_affinity::set_for_current`.
