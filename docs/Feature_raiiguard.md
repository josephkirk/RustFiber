To achieve the robustness required for an industrial-grade job system, the architecture must account for **stack unwinding** (panics). If a job panics and the system does not catch it, the worker thread may crash, or more commonly, the dependency counters will never reach zero, causing a permanent system deadlock.

---

### 1. The RAII Counter Guard

The most critical safety mechanism is an **RAII (Resource Acquisition Is Initialization) Guard** for the `AtomicCounter`. This ensures that the counter is decremented regardless of whether the job completes normally or terminates via a panic.

#### **Implementation Specification**

```rust
pub struct CounterGuard<'a> {
    counter: &'a AtomicCounter,
    system: &'a Arc<JobSystem>,
}

impl<'a> Drop for CounterGuard<'a> {
    fn drop(&mut self) {
        // The Drop implementation ensures the dependency graph resolves
        // even if the job scope exits prematurely due to a panic.
        [cite_start]self.counter.signal(self.system); [cite: 108, 109]
    }
}

```

When a job is submitted with a dependency, the user should be provided with this guard. By moving the guard into the job's closure, the Rust compiler guarantees that `drop()` will be called when the job's stack frame is destroyed.

---

### 2. The `catch_unwind` Boundary

In Rust, crossing an FFI boundary or a manual context switch during unwinding is **Undefined Behavior (UB)**. Because fibers swap stacks manually, a panic must be intercepted before it reaches the scheduler loop.

#### **Execution Wrapper**

The scheduler must wrap the job execution in `std::panic::catch_unwind`:

```rust
let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
    (job_closure)(); // Execute the actual work
}));

if let Err(payload) = result {
    // 1. Log the panic for debugging.
    // 2. The CounterGuard (captured in the closure) will automatically 
    [cite_start]//    drop here, signaling the counter and preventing deadlock. [cite: 108]
    // 3. Mark the fiber for recycling.
}

```

---

### 3. Safety Invariants for Panic Handling

* **AssertUnwindSafe:** Jobs must be `UnwindSafe`. Since jobs are `Send`, they typically satisfy this requirement unless they capture shared mutable state like `&mut T` or `RefCell`.


* 
**Zero-Downtime Recovery:** By catching the panic, the worker thread remains alive and can immediately pick up the next job from its local queue.


* **Fiber Integrity:** After a panic, the fiber's stack might be in an inconsistent state. The `FiberPool` must ensure that a panicked fiber has its stack pointer and registers fully reset before it is leased out again.



---

### 4. Comparison: Standard vs. Panic-Safe Implementation

| Feature | Standard Implementation | Panic-Safe Architecture |
| --- | --- | --- |
| **Counter Decrement** | Manual `counter.signal()` at end. | Automatic via `CounterGuard::drop`. 

 |
| **Logic Error Result** | Permanent Deadlock. 

 | System continues; only one job fails. 

 |
| **Thread Stability** | Worker thread may crash. 

 | Worker catches panic and recovers. 

 |
| **Performance Overhead** | Zero. | Negligible (setup of a landing pad). |

