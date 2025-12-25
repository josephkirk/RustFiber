# Fiber Yielding Implementation Plan

## Overview

This document outlines the implementation plan for adding true fiber-level cooperative yielding to RustFiber. Currently, `Context::yield_now()` falls back to `std::thread::yield_now()`, which yields the entire OS thread. True fiber yielding would allow individual jobs to suspend and resume execution without blocking worker threads, enabling better CPU utilization and more responsive scheduling.

## Current State

### What We Have
- ✅ Context type for safe job system access
- ✅ Nested parallelism support via `spawn_job()` and `spawn_jobs()`
- ✅ Counter-based synchronization
- ✅ Work-stealing scheduler
- ⚠️ Thread-level yielding via `Context::yield_now()`

### Current Limitations
- `yield_now()` yields the entire worker thread, not just the fiber
- No ability to suspend a fiber mid-execution and resume it later
- Long-running jobs can starve other work even with yielding
- No priority-based scheduling

## Goals

1. **True Fiber Suspension**: Allow jobs to suspend execution and be resumed later
2. **Non-Blocking Yielding**: Yielding should allow the worker thread to execute other jobs
3. **Transparent Resumption**: Fibers should resume exactly where they yielded
4. **Minimal Overhead**: Yielding should be fast and not impact non-yielding jobs
5. **Safe API**: Maintain Rust's safety guarantees (no data races, memory safety)

## Implementation Approaches

### Approach 1: Stackful Fibers with Context Switching (Recommended)

Use a dedicated crate like `boost-context` or `context-rs` for low-level context switching.

**Pros:**
- True stack preservation - fibers can yield from anywhere
- Natural call stack support
- Best performance for complex yielding scenarios
- Industry-proven approach (used in game engines)

**Cons:**
- Platform-specific assembly code required
- More complex implementation
- Stack size management needed
- Unsafe code required at the core

**Architecture:**
```
┌─────────────────────────────────────────────┐
│           Worker Thread                      │
│                                              │
│  ┌────────────┐      ┌────────────┐        │
│  │  Fiber A   │      │  Fiber B   │        │
│  │  (8KB)     │      │  (8KB)     │        │
│  │            │      │            │        │
│  │  yield()   │◄────►│  running   │        │
│  │  suspended │      │            │        │
│  └────────────┘      └────────────┘        │
│                                              │
│         Fiber Scheduler                      │
│  ┌──────────────────────────────┐          │
│  │  Ready Queue                 │          │
│  │  [Fiber B, Fiber C, ...]     │          │
│  └──────────────────────────────┘          │
└─────────────────────────────────────────────┘
```

### Approach 2: Async/Await Integration

Leverage Rust's async ecosystem and integrate with executor like Tokio or async-std.

**Pros:**
- Leverages existing Rust async infrastructure
- No unsafe context switching needed
- Composable with existing async code
- Compiler-assisted state machines

**Cons:**
- API changes required (jobs would need to be async)
- Overhead of async state machines
- Less control over scheduling
- Not a true "fiber" system anymore

### Approach 3: Generator-Based Approach

Use Rust's generator feature (nightly-only) to create resumable functions.

**Pros:**
- Native Rust feature (when stabilized)
- Compiler-generated state machines
- Memory safe by design

**Cons:**
- Currently unstable/nightly-only
- Limited control over scheduling
- May not meet all use cases

## Recommended Implementation: Stackful Fibers

We recommend Approach 1 (Stackful Fibers) as it best fits the game engine use case and provides maximum flexibility.

## Detailed Implementation Plan

### Phase 1: Core Fiber Infrastructure (2-3 weeks)

#### 1.1 Add Dependencies
```toml
[dependencies]
# Choose one:
context-rs = "0.2"  # Pure Rust with platform-specific assembly
# OR
boost-context = "0.1"  # Bindings to Boost.Context
```

#### 1.2 Create Fiber Context Structure

**File:** `src/fiber_context.rs`

```rust
use std::ptr::NonNull;

/// Represents a fiber's execution context (registers, stack pointer, etc.)
pub struct FiberContext {
    /// Platform-specific context data
    context: context_rs::Context,
    /// Stack memory for this fiber
    stack: Stack,
    /// Current state of the fiber
    state: FiberState,
}

enum FiberState {
    Ready,      // Ready to run
    Running,    // Currently executing
    Suspended,  // Yielded, waiting to resume
    Completed,  // Finished execution
}

struct Stack {
    memory: NonNull<u8>,
    size: usize,
}

impl FiberContext {
    pub fn new(stack_size: usize) -> Self {
        // Allocate stack memory
        // Initialize context
    }
    
    pub fn resume(&mut self) {
        // Switch to this fiber's context
    }
    
    pub fn yield_to(&mut self, other: &mut FiberContext) {
        // Save current context, switch to other
    }
}
```

#### 1.3 Update Fiber Structure

**File:** `src/fiber.rs`

```rust
pub struct Fiber {
    /// The job to execute (or None if already started)
    job: Option<Job>,
    /// Execution context (stack, registers)
    context: Option<FiberContext>,
    /// Unique fiber ID
    id: FiberId,
}

impl Fiber {
    pub fn new_with_context(job: Job, stack_size: usize) -> Self {
        Fiber {
            job: Some(job),
            context: Some(FiberContext::new(stack_size)),
            id: FiberId::new(),
        }
    }
    
    pub fn can_yield(&self) -> bool {
        self.context.is_some()
    }
    
    pub fn resume(&mut self) -> FiberResult {
        // Resume fiber execution
    }
}

pub enum FiberResult {
    Completed,
    Yielded,
    Error(String),
}
```

### Phase 2: Scheduler Integration (1-2 weeks)

#### 2.1 Update Worker Run Loop

**File:** `src/worker.rs`

```rust
impl Worker {
    fn run_loop(params: WorkerParams) {
        let mut current_fiber: Option<Fiber> = None;
        
        loop {
            if shutdown.load(Ordering::Relaxed) {
                break;
            }
            
            // Check if we have a fiber to resume
            if let Some(mut fiber) = current_fiber.take() {
                match fiber.resume() {
                    FiberResult::Completed => {
                        // Fiber done, get next job
                    }
                    FiberResult::Yielded => {
                        // Put back in ready queue for later
                        yield_queue.push(fiber);
                    }
                    FiberResult::Error(e) => {
                        eprintln!("Fiber error: {}", e);
                    }
                }
            }
            
            // Try to get work (check yielded fibers first)
            let fiber = yield_queue.pop()
                .or_else(|| get_job_from_queues().map(Fiber::new));
                
            if let Some(f) = fiber {
                current_fiber = Some(f);
            } else {
                std::thread::yield_now();
            }
        }
    }
}
```

#### 2.2 Add Yield Queue

Each worker needs a queue for yielded fibers:

```rust
pub(crate) struct WorkerParams {
    // ... existing fields ...
    pub(crate) yield_queue: Arc<crossbeam::queue::SegQueue<Fiber>>,
}
```

### Phase 3: Context API Enhancement (1 week)

#### 3.1 Implement True Yielding

**File:** `src/context.rs`

```rust
impl<'a> Context<'a> {
    /// Cooperatively yield execution to allow other work to run.
    ///
    /// The fiber will be suspended and placed back in the ready queue.
    /// The worker thread will execute other jobs until this fiber
    /// is resumed.
    ///
    /// # Example
    ///
    /// ```
    /// job_system.run_with_context(|ctx| {
    ///     for i in 0..1000000 {
    ///         // Do some work
    ///         if i % 1000 == 0 {
    ///             ctx.yield_now();  // Let other work run
    ///         }
    ///     }
    /// });
    /// ```
    pub fn yield_now(&self) {
        // Check if we're running in a fiber with context
        if let Some(fiber_ctx) = get_current_fiber_context() {
            // Save current state and yield to scheduler
            fiber_ctx.yield_to_scheduler();
        } else {
            // Fallback for non-fiber jobs
            std::thread::yield_now();
        }
    }
}

// Thread-local storage for current fiber context
thread_local! {
    static CURRENT_FIBER: RefCell<Option<*mut FiberContext>> = RefCell::new(None);
}

fn get_current_fiber_context() -> Option<&'static mut FiberContext> {
    CURRENT_FIBER.with(|f| {
        f.borrow().map(|ptr| unsafe { &mut *ptr })
    })
}
```

### Phase 4: Advanced Features (2-3 weeks)

#### 4.1 Wait with Yielding

Allow fibers to yield while waiting for counters:

```rust
impl<'a> Context<'a> {
    /// Wait for a counter with yielding.
    ///
    /// Instead of busy-waiting, this suspends the fiber and only
    /// resumes when the counter is ready.
    pub fn wait_for_yielding(&self, counter: &Counter) {
        while !counter.is_complete() {
            self.yield_now();
        }
    }
}
```

#### 4.2 Priority Scheduling

Add priority levels for jobs:

```rust
pub enum JobPriority {
    High,
    Normal,
    Low,
}

impl JobSystem {
    pub fn run_with_priority<F>(&self, priority: JobPriority, work: F) -> Counter
    where
        F: FnOnce(&Context) + Send + 'static,
    {
        // Implementation
    }
}
```

#### 4.3 Fiber Pool

Reuse fiber contexts to avoid allocation overhead:

```rust
pub struct FiberPool {
    available: Arc<SegQueue<FiberContext>>,
    stack_size: usize,
}

impl FiberPool {
    pub fn get_or_create(&self) -> FiberContext {
        self.available.pop()
            .unwrap_or_else(|| FiberContext::new(self.stack_size))
    }
    
    pub fn return_fiber(&self, fiber: FiberContext) {
        self.available.push(fiber);
    }
}
```

### Phase 5: Testing & Optimization (1-2 weeks)

#### 5.1 Unit Tests

```rust
#[test]
fn test_fiber_yielding() {
    let job_system = JobSystem::new(2);
    let iterations = Arc::new(AtomicUsize::new(0));
    
    let iter = iterations.clone();
    let counter = job_system.run_with_context(|ctx| {
        for _ in 0..1000 {
            iter.fetch_add(1, Ordering::SeqCst);
            ctx.yield_now();
        }
    });
    
    job_system.wait_for_counter(&counter);
    assert_eq!(iterations.load(Ordering::SeqCst), 1000);
}

#[test]
fn test_yield_fairness() {
    // Test that yielding allows other work to run
    let job_system = JobSystem::new(1);
    let order = Arc::new(Mutex::new(Vec::new()));
    
    let ord1 = order.clone();
    let job1 = job_system.run_with_context(|ctx| {
        for i in 0..5 {
            ord1.lock().unwrap().push(format!("Job1-{}", i));
            ctx.yield_now();
        }
    });
    
    let ord2 = order.clone();
    let job2 = job_system.run_with_context(|ctx| {
        for i in 0..5 {
            ord2.lock().unwrap().push(format!("Job2-{}", i));
            ctx.yield_now();
        }
    });
    
    job_system.wait_for_counter(&job1);
    job_system.wait_for_counter(&job2);
    
    // Verify jobs interleaved
    let sequence = order.lock().unwrap();
    assert!(sequence.contains(&"Job1-0".to_string()));
    assert!(sequence.contains(&"Job2-0".to_string()));
}
```

#### 5.2 Benchmarks

Create benchmarks to measure:
- Yield overhead (context switch time)
- Throughput with varying yield frequencies
- Comparison with thread yielding
- Memory overhead of fiber stacks

#### 5.3 Performance Optimization

- **Stack Size Tuning**: Find optimal default stack size
- **Context Switch Optimization**: Minimize register save/restore overhead
- **Cache Locality**: Keep hot fibers on the same worker thread
- **Fiber Pool**: Reduce allocation overhead by reusing contexts

## Safety Considerations

### Memory Safety
- ✅ Stack memory must be properly aligned and sized
- ✅ No stack overflow detection (add guard pages)
- ✅ Fiber contexts must not outlive their stacks
- ✅ Thread-local storage for current fiber context

### Concurrency Safety
- ✅ Fibers must not be resumed on multiple threads simultaneously
- ✅ Yielded fibers must be properly synchronized
- ✅ Counter updates must remain atomic
- ✅ No data races in fiber state transitions

### Unsafe Code Review
All unsafe code must be:
1. Documented with safety invariants
2. Reviewed by multiple developers
3. Tested extensively
4. Fuzzed for edge cases

## Configuration

Add configuration options:

```rust
pub struct FiberConfig {
    /// Stack size for each fiber (default: 64KB)
    pub stack_size: usize,
    /// Enable fiber yielding (default: true)
    pub enable_yielding: bool,
    /// Maximum fiber pool size per worker (default: 32)
    pub max_pooled_fibers: usize,
}

impl JobSystem {
    pub fn new_with_fiber_config(num_threads: usize, config: FiberConfig) -> Self {
        // Implementation
    }
}
```

## Migration Path

### Backwards Compatibility

All existing code continues to work:
- Jobs without context still work (no fiber overhead)
- `run()` and `run_multiple()` unchanged
- Only `run_with_context()` jobs get fiber support

### Opt-In Feature Flag

```toml
[dependencies]
rustfiber = { version = "0.2", features = ["fiber-yielding"] }
```

### Documentation Updates

- Update README with yielding examples
- Add tutorial on when to use yielding
- Document performance characteristics
- Add migration guide

## Timeline

| Phase | Duration | Dependencies |
|-------|----------|--------------|
| Phase 1: Core Fiber Infrastructure | 2-3 weeks | None |
| Phase 2: Scheduler Integration | 1-2 weeks | Phase 1 |
| Phase 3: Context API Enhancement | 1 week | Phase 2 |
| Phase 4: Advanced Features | 2-3 weeks | Phase 3 |
| Phase 5: Testing & Optimization | 1-2 weeks | Phase 4 |
| **Total** | **7-11 weeks** | |

## Success Metrics

1. **Performance**: Yield overhead < 100ns per context switch
2. **Throughput**: No regression on non-yielding workloads
3. **Fairness**: Yielding jobs don't starve other work
4. **Safety**: Zero memory safety issues in fiber code
5. **Usability**: Clear API, good documentation, working examples

## Open Questions

1. **Stack Size**: What's the optimal default? Should it be configurable per-job?
2. **Platform Support**: Which platforms to support initially? (x86_64, ARM, etc.)
3. **Debugging**: How to debug fibers? (stack traces, etc.)
4. **Fiber Limits**: Should there be a maximum number of concurrent fibers?
5. **Integration**: How to integrate with async/await if needed?

## References

- [Boost.Context Documentation](https://www.boost.org/doc/libs/1_85_0/libs/context/doc/html/index.html)
- [Naughty Dog Fiber Architecture](https://www.gdcvault.com/play/1022186/Parallelizing-the-Naughty-Dog-Engine)
- [Fibers in Game Engines (blog post)](https://blog.molecular-matters.com/2015/08/24/job-system-2-0-lock-free-work-stealing-part-1-basics/)
- [context-rs crate](https://docs.rs/context-rs/)
- [Rust Generators RFC](https://github.com/rust-lang/rfcs/blob/master/text/2033-experimental-coroutines.md)

## Next Steps

1. ✅ Complete Context type implementation (DONE)
2. 📋 Get feedback on this plan from stakeholders
3. 🔍 Evaluate context switching libraries (context-rs vs boost-context)
4. 📝 Create detailed API design document
5. 🚀 Begin Phase 1 implementation

---

*Document Version: 1.0*  
*Last Updated: 2025-12-25*  
*Author: RustFiber Team*
