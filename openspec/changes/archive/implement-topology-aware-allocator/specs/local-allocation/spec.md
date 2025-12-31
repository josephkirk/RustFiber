# Local Allocation Requirement

## ADDED Requirements

### Capability: Thread-Local Frame Allocation

#### Requirement: Workers Expose Allocator via TLS
Workers MUST expose their local `FrameAllocator` (memory arena) via Thread Local Storage (TLS) during their execution loop.

#### Scenario: Validation via Job
A validation job checks the TLS pointer and confirms it points to the active worker's stack-allocated arena.

#### Scenario: TLS Invalidation
When a worker finishes or yields significantly, the TLS pointer IS invalidated or managed strictly to avoid dangling references (though frame reset handles validity scope).

#### Requirement: Transparent Job Spawning
Use `Job::spawn` (or modified `Job::new`) to create jobs. This method MUST:
1. Check if the TLS allocator is available.
2. If available, allocate the closure data into the FrameAllocator and return a `Job` with `Work::SimpleFrame` (or context variant).
3. If unavailable (e.g., external thread), fall back to `Box::new` (Global Allocator) and return `Job` with `Work::Simple`.

#### Scenario: High-frequency Benchmark
High-frequency job creation benchmark runs 2x faster (allocation overhead removed) when running on worker threads vs global heap.

#### Scenario: Main Thread Fallback
A job created on the main thread (no worker TLS) functions correctly using the heap fallback.

#### Requirement: Safety and Lifetimes
Allocations in the FrameAllocator are valid ONLY for the duration of the current "frame" (batch of work).

#### Scenario: Frame Reset Safety
The system guarantees that all jobs allocated in Frame N are executed or dropped before Frame N+1 begins (resetting the allocator).

#### Scenario: UB Prevention
Attempting to use a frame-allocated closure after the frame resets is Undefined Behavior (UB), but the logic (Allocator Reset) MUST ensure consumers are drained before reset.


