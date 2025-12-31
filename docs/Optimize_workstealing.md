To achieve the throughput-centric parallelism required by the specification, the work-stealing subsystem must prioritize **data locality** and minimize **contention**. The system utilizes a specialized  threading model where each worker thread manages its own workload while dynamically balancing the global load.

---

### 1. Dual-Ended Queue (Chase-Lev Deque)

The core of the work-stealing optimization is the use of the **Chase-Lev Deque**, typically implemented via the `crossbeam-deque` crate. This structure allows for different access patterns depending on whether the thread is the "owner" or a "thief".

#### **Local Access (LIFO)**

* 
**The Owner Thread:** Pushes and pops jobs from the **bottom** of the deque.


* 
**Rationale:** This establishes a **Last-In, First-Out (LIFO)** behavior.


* 
**Optimization:** In game engines, a newly created job often operates on data that is still "hot" in the CPU's L1/L2 cache (e.g., a physics sub-task spawned by a main physics update). Processing this child job immediately maximizes cache coherency.



#### **Stealing Access (FIFO)**

* 
**The Thief Thread:** When a worker's local queue is empty, it targets another worker's deque and steals from the **top**.


* 
**Rationale:** This establishes a **First-In, First-Out (FIFO)** behavior for stealing.


* 
**Optimization:** In recursive task decomposition, the oldest jobs (at the top) are typically the "root" tasks. Stealing a large root task provides the thief with a significant block of work, reducing the frequency of future steal attempts and amortizing synchronization costs.



---

### 2. The Injector Queue

To handle jobs originating from outside the job system (e.g., OS callbacks, networking, or the main thread), a **Global Injector Queue** is maintained.

* 
**Frequency:** Workers do not check the injector on every iteration to avoid global lock contention.


* 
**Heuristic:** Workers typically check the injector periodically (e.g., every 64 or 100 iterations) or only when their local and steal attempts have failed.



---

### 3. Stealing Strategy and Backoff

To prevent worker threads from consuming excessive power or generating heat while idling, a tiered backoff strategy is required.

| State | Action | Rationale |
| --- | --- | --- |
| **Active** | Pop from local queue (LIFO).

 | Highest performance, best cache locality.

 |
| **Searching** | Steal from neighbors or Injector.

 | Balances load across the CPU cores.

 |
| **Brief Idle** | <br>`std::hint::spin_loop()`.

 | Low-latency wait for incoming work.

 |
| **Deep Idle** | <br>`std::thread::yield_now()` or `park()`.

 | Reduces CPU usage during significant stalls.

 |

---

### 4. Implementation Specification (Rust)

The following structures define the relationship between the global system and the local workers:

```rust
pub struct JobSystem {
    /// The global entry point for jobs from external threads
    pub injector: crossbeam_deque::Injector<Job>,
    /// Handles to steal from every other worker thread
    pub stealers: Vec<crossbeam_deque::Stealer<Job>>,
}

pub struct Worker {
    /// The local queue: only this thread pushes/pops here
    pub local_queue: crossbeam_deque::Worker<Job>,
    /// Access to the global system for stealing
    pub system: Arc<JobSystem>,
}

```



### 5. Summary of Optimization Benefits

* 
**Lock-Free Hot Path:** Local operations (Push/Pop) are performed with minimal atomic overhead, avoiding heavy mutex contention.


* 
**Amortized Contention:** By stealing the oldest (largest) tasks, workers interact with other queues less frequently.


* 
**Scalability:** As core counts rise, the distributed nature of local deques prevents the "bottleneck" effect seen in single-queue architectures.

