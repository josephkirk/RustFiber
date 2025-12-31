Implementing an **intrusive wait list** is the most advanced architectural requirement for a high-performance fiber system. Unlike a standard `Vec<Fiber>`, which requires heap allocation and locking, an intrusive list embeds the linking pointers directly into the fiber’s own structure, allowing for zero-allocation, lock-free suspension.

### 1. Structural Definition

The "Intrusive" nature comes from the `WaitNode` being a field of the `Fiber` itself. This ensures that as long as the fiber is alive (suspended), the memory for its "node" is guaranteed to be valid.

#### **Fiber Extension**

```rust
pub struct Fiber {
    // ... existing fiber state (stack, registers) ...
    
    /// The intrusive node used when the fiber is waiting.
    /// Wrapped in Pin to ensure it never moves while linked.
    pub wait_node: UnsafeCell<WaitNode>,
}

#[repr(C)]
pub struct WaitNode {
    /// Pointer to the next fiber in the wait list.
    /// Using AtomicPtr allows for lock-free 'Push' operations.
    pub next: AtomicPtr<WaitNode>,
    /// Handle used by the scheduler to resume this specific fiber.
    pub fiber_handle: FiberHandle,
}

```

#### **Wait List (The Head)**

The `AtomicCounter` (or any synchronization primitive) acts as the container.

```rust
pub struct WaitList {
    /// The head of a lock-free stack of waiting fibers.
    head: AtomicPtr<WaitNode>,
}

```

---

### 2. Core Operations & Memory Ordering

To achieve maximum throughput, the wait list must be **lock-free**. Because fibers typically wait one-by-one but are woken in batches, a **Lock-Free Stack (Treiber Stack)** is the optimal implementation.

#### **A. Suspend (The Push)**

When a fiber calls `wait_for_counter`, it performs a lock-free push onto the counter's wait list.

1. **Prepare Node:** The fiber sets its `wait_node.next` to the current `head` of the list.
2. **Atomic Swap:** It uses `compare_exchange` to update the `head` to point to its own `wait_node`.
3. **Ordering:** Must use `Ordering::Release` to ensure all prior memory writes (the job's work) are visible to whoever wakes the fiber.

#### **B. Wake (The PopAll)**

When the counter reaches zero, the "waker" thread drains the entire list in a single atomic operation.

1. **Atomic Exchange:** The waker swaps the `head` with `null`.
2. **Batch Resume:** The waker now owns a private linked list. It iterates through the list and pushes the `FiberHandles` back into the scheduler's local or injector queue.
3. **Ordering:** Must use `Ordering::Acquire` to synchronize with the `Release` from the pushing fibers.

---

### 3. Safety Invariants & Constraints

Implementing this in Rust requires navigating "Unsafe Rust" to satisfy the borrow checker while maintaining manual pointer stability.

* **Memory Stability (`Pin`):** Once a fiber is added to a `WaitList`, it **must not move** in memory. This is guaranteed by the `FiberPool` which pre-allocates stacks at stable addresses and the use of the `Pin` wrapper.
* **Ownership Transfer:** When a fiber yields, ownership effectively passes from the `Worker` thread to the `AtomicCounter`. The counter "owns" the fiber until it transitions to zero.
* **Zero Allocation:** Because the `WaitNode` is pre-allocated as part of the `Fiber` struct, `wait_for_counter` involves zero calls to the global allocator (`malloc`), eliminating a major source of thread contention.

### 4. Specification Summary Table

| Requirement | Implementation Detail | Rationale |
| --- | --- | --- |
| **Storage** | `AtomicPtr<WaitNode>` | Enables lock-free concurrent pushes. |
| **Locality** | `#[repr(align(64))]` | Prevents false sharing on the `head` pointer. |
| **Wait Logic** | LIFO (Stack-based) | Simple lock-free implementation via `compare_exchange`. |
| **Wake Logic** | Batch Drain | Reduces queue contention by moving nodes as a group. |

[Parallelizing the Naughty Dog engine using fibers](https://www.youtube.com/watch?v=mKXo-lo7YLk)

This video is relevant because it explains the original implementation details of this architecture at Naughty Dog, providing the performance context for why intrusive lists are used instead of standard containers.

Implementing the intrusive wait list in Rust requires careful use of `AtomicPtr`, `UnsafeCell`, and `Pin` to ensure that memory remains stable while the fiber is suspended. The following specification outlines the exact implementation details for a lock-free, zero-allocation dependency system.

---

### 1. Data Structures

The core principle is that the `WaitNode` is embedded within the `Fiber` struct itself. This ensures that as long as the fiber exists, the node memory is valid, allowing it to be linked into a list without a new heap allocation.

```rust
use std::sync::atomic::{AtomicPtr, Ordering};
use std::cell::UnsafeCell;

/// The node embedded in every Fiber. 
/// Must be 'repr(C)' for stable pointer offsets.
#[repr(C)]
pub struct WaitNode {
    /// Atomic pointer to the next node in the list.
    pub next: AtomicPtr<WaitNode>,
    /// The ID or handle of the fiber to be resumed.
    pub fiber_id: FiberHandle,
}

pub struct Fiber {
    pub stack: FiberStack,
    /// The intrusive node. Using UnsafeCell because 'next' 
    /// will be modified by different threads while the fiber is suspended.
    pub wait_node: UnsafeCell<WaitNode>,
}

pub struct AtomicCounter {
    pub value: AtomicUsize,
    /// The head of a LIFO intrusive stack.
    pub wait_list: AtomicPtr<WaitNode>,
}

```

---

### 2. The "Push" Logic (Fiber Suspension)

When a fiber calls `wait_for_counter`, it must atomically link itself into the counter's list. This uses a **Treiber Stack** approach to ensure thread safety without mutexes.

```rust
pub fn wait_for_counter(counter: &AtomicCounter, fiber: &Pin<Box<Fiber>>) {
    // 1. Prepare the node inside the fiber
    let node_ptr = fiber.wait_node.get();
    
    unsafe {
        let mut current_head = counter.wait_list.load(Ordering::Relaxed);
        loop {
            // Point our 'next' to the current head
            (*node_ptr).next.store(current_head, Ordering::Relaxed);

            // Attempt to swap the head with our node
            match counter.wait_list.compare_exchange_weak(
                current_head,
                node_ptr,
                Ordering::Release, // Ensure our 'next' write is visible
                Ordering::Relaxed,
            ) {
                Ok(_) => break, // Successfully linked
                Err(new_head) => current_head = new_head, // Retry with updated head
            }
        }
    }
    [cite_start]// 2. Yield control back to the scheduler [cite: 116]
}

```

---

### 3. The "PopAll" Logic (Batch Waking)

When the counter reaches zero, the thread that performed the final decrement "claims" the entire list of fibers to be resumed.

```rust
pub fn signal_counter(counter: &AtomicCounter) {
    let old_val = counter.value.fetch_sub(1, Ordering::Release);
    
    [cite_start]// If we hit zero, we must wake the waiting fibers [cite: 36]
    if old_val == 1 {
        [cite_start]// Use Ordering::Acquire to see all data writes from the waiting fibers [cite: 78]
        let mut head = counter.wait_list.swap(std::ptr::null_mut(), Ordering::Acquire);
        
        // head now points to a private linked list of fibers
        while !head.is_null() {
            unsafe {
                let node = &*head;
                let next_node = node.next.load(Ordering::Relaxed);
                
                [cite_start]// Push the fiber handle back to the ready queue [cite: 124]
                scheduler_push_ready(node.fiber_id);
                
                head = next_node;
            }
        }
    }
}

```

---

### 4. Implementation Invariants

* **Memory Stability:** The `Fiber` must be `Pin<Box<Fiber>>` or similar. If the `Fiber` struct moves in memory while its `wait_node` is in the `AtomicCounter` list, the `next` pointers in the list will become dangling.


* 
**Memory Ordering:** * The **Push** must use `Ordering::Release` to ensure that any work the job did before waiting is visible to the waker.


* The **Swap (PopAll)** must use `Ordering::Acquire` to synchronize with the pushers.




* 
**Small Job Optimization:** By embedding the `WaitNode`, the system avoids any calls to `malloc` during the `wait_for_counter` call, preserving the "Zero OS Interference" pillar.


Integrating the intrusive wait list into a **Fiber Pool** is the final step to ensuring the system is truly zero-allocation and safe for reuse. The goal is to ensure that when a fiber is returned to the pool after completing a job, its intrusive pointers are reset so it doesn't carry "garbage" state into the next task.

### 1. The Fiber Pool Structure

The pool should manage a set of pre-allocated, pinned fibers. Using `Pin<Box<Fiber>>` ensures that the `WaitNode` embedded within each fiber stays at a stable memory address, which is a hard requirement for intrusive pointers.

```rust
pub struct FiberPool {
    /// A lock-free stack of available fibers.
    available_fibers: crossbeam_deque::Injector<Pin<Box<Fiber>>>,
    stack_size: usize,
}

impl FiberPool {
    [cite_start]/// Initializes the pool with pre-allocated stacks[cite: 56, 57].
    pub fn new(count: usize, stack_size: usize) -> Self {
        let pool = Self {
            available_fibers: crossbeam_deque::Injector::new(),
            stack_size,
        };
        for i in 0..count {
            pool.available_fibers.push(Fiber::new(i, stack_size));
        }
        pool
    }
}

```

---

### 2. The Reset and Recycle Mechanism

When a job completes, the worker loop returns the fiber to the pool. At this point, the `WaitNode` must be "cleansed."

```rust
impl Fiber {
    [cite_start]/// Resets the intrusive node state for reuse[cite: 59].
    pub fn reset_wait_node(&self) {
        unsafe {
            let node = &mut *self.wait_node.get();
            [cite_start]// Clear the 'next' pointer so it doesn't point to a stale address[cite: 60].
            node.next.store(std::ptr::null_mut(), Ordering::Relaxed);
        }
    }
}

```

---

### 3. Integration with the Worker Loop

The worker loop acts as the coordinator. It fetches a fiber, runs the job, and then handles the cleanup based on whether the fiber finished normally or yielded.

```rust
fn worker_loop(worker: &mut Worker) {
    loop {
        if let Some(job) = worker.get_next_job() {
            [cite_start]// 1. Acquire a clean fiber from the pool[cite: 82].
            let mut fiber = worker.system.fiber_pool.get();

            [cite_start]// 2. Execute the job inside the fiber[cite: 83].
            let result = fiber.run(job);

            match result {
                FiberState::Complete => {
                    // 3a. [cite_start]Job finished: reset pointers and return to pool[cite: 84, 85].
                    fiber.reset_wait_node();
                    worker.system.fiber_pool.return_fiber(fiber);
                }
                FiberState::Yielded => {
                    // 3b. Job yielded: the fiber is now owned by an AtomicCounter's 
                    // intrusive list. [cite_start]We do NOT return it to the pool yet[cite: 86, 87].
                }
            }
        }
    }
}

```

---

### 4. Summary of Safety Guards

* 
**Stable Addresses:** By using a pool of `Pin<Box<Fiber>>`, we guarantee the `WaitNode` pointers remain valid even if the pool itself is moved.


* 
**Leak Prevention:** If a fiber yields, it is only returned to the pool by the `AtomicCounter` once the counter hits zero and "wakes" the fiber.


* 
**Zero-Downtime Switching:** Resetting only the `next` pointer and the stack pointer (managed by `corosensei`) takes ~10-50 nanoseconds, maintaining the high-throughput goal.


To achieve the high-performance targets of the architectural specification, the `signal` mechanism must handle the transition from the intrusive wait list back to the scheduler without causing a "thundering herd" of individual lock acquisitions.

### **1. AtomicCounter Signal and Batch Resumption**

The following implementation demonstrates how a thread completing a job decrements the counter and, if it reaches zero, "claims" the entire list of waiting fibers. Instead of pushing fibers one by one, we perform a **Batch Wake** to minimize synchronization overhead.

```rust
impl AtomicCounter {
    pub fn signal(&self, system: &Arc<JobSystem>) {
        // 1. Decrement the counter with Release ordering to ensure 
        [cite_start]// prior job work is visible to the resuming fibers[cite: 77].
        let old_val = self.value.fetch_sub(1, Ordering::Release);

        [cite_start]// 2. If the counter transitioned to zero, we are the 'waker'[cite: 36].
        if old_val == 1 {
            [cite_start]// 3. Atomically drain the entire list in one operation[cite: 124].
            [cite_start]// Acquire ensures we see the data writes from the 'pushing' fibers[cite: 78].
            let mut head = self.wait_list.swap(std::ptr::null_mut(), Ordering::Acquire);

            [cite_start]// 4. Temporary collection for batch injection[cite: 124].
            let mut batch = Vec::with_capacity(32); 

            while !head.is_null() {
                unsafe {
                    let node = &*head;
                    let next_node = node.next.load(Ordering::Relaxed);

                    [cite_start]// Convert the node/handle back into a Job or Fiber task[cite: 82].
                    batch.push(node.fiber_id);
                    
                    head = next_node;
                }
            }

            [cite_start]// 5. Use crossbeam-deque's batch injection to amortize overhead[cite: 125].
            system.injector.push_batch(batch);
        }
    }
}

```

---

### **2. FiberPool Integration and Reset**

Once a fiber is eventually resumed and completes its new task, it must be cleansed before returning to the pool to prevent memory corruption in subsequent frames.

* 
**RAII for Automatic Cleanup:** Each fiber should be wrapped in a guard that ensures the `wait_node` is nullified even if the job panics.


* 
**Memory Pinned:** The `FiberPool` uses `Pin<Box<Fiber>>` so that the `wait_node` addresses stored in `AtomicCounter` lists remain stable.


* 
**Linear Allocation Sync:** When the pool recycles fibers at the end of a frame, it aligns with the reset of the **Frame Linear Allocators**, ensuring no dangling pointers exist in the next frame.



### **3. Performance Impact of Intrusive Batching**

| Metric | Standard `Vec<Fiber>` + Mutex | Intrusive Batching |
| --- | --- | --- |
| **Allocation Cost** | <br> heap allocations 

 | <br>**Zero** (Pre-allocated in pool) 

 |
| **Contention** | High (Locked per push/pop) 

 | <br>**Minimal** (Single atomic swap) 

 |
| **Latency** | ~1000ns (Kernel involves) 

 | <br>**~50ns** (User-space swap) 

 |
| **Cache Behavior** | Random (Heap-based) 

 | <br>**Excellent** (Embedded in stack) 

 |
