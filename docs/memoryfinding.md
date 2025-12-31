* **Memory Management (Bump Allocation):**
* 
**The Research Requirement:** The architecture demands **Frame Linear Allocators** to avoid lock contention in the global allocator.


* 
**The Progress:** `RustFiber` generally relies on standard heap allocations (`Box`, `Vec`), which can lead to significant performance degradation during high-frequency job creation (tens of thousands per frame).

Memory Management Evaluation
This document evaluates the RustFiber source code against the requirement for Frame Linear Allocators (Bump Allocation).

Requirement vs. Reality
Requirement	Current State	Status
Frame Linear Allocators	Standard Heap Allocation	🔴 Critical Gap
Avoid global lock contention	Box::new used for every Job	⚠️ Performance Risk
High-frequency job creation	Arc::new for Counters	⚠️ Overhead
Findings
1. Ubiquitous Heap Allocation via Box<dyn FnOnce>
Every job submitted to the system is boxed. The 
Job
 struct definition enforces this:

// src/job.rs
pub enum Work {
    Simple(Box<dyn FnOnce() + Send + 'static>),
    WithContext {
        work: Box<dyn FnOnce(&Context) + Send + 'static>,
        // ...
    },
    // ...
}
This means:

Per-Job Allocation: A call to malloc (or helper) is made for every single 
spawn_job
 call.
Cache Locality: Job closures are scattered across the heap, potentially hurting cache performance compared to a contiguous linear buffer.
Deallocation: Each job completion triggers a free.
2. Batch Operations still Allocate
Even 
run_multiple
 collects jobs into a Vec first, then converts them to 
Job
 objects (which box the closures).

// src/job_system.rs
pub fn run_multiple<I>(&self, jobs: I) -> Counter {
    let jobs_vec: Vec<_> = jobs.into_iter().collect(); // Allocation 1 (Vec backing)
    // ...
    let job_objs: Vec<_> = jobs_vec.into_iter().map(|work| {
         // ...
         Job::with_counter(work, counter_clone) // Allocation 2+ (Box per job)
    }).collect();
    // ...
}
3. Synchronization Primitives
Counters: Counter uses Arc internally (Arc<Inner>). This is another allocation per counter (often per batch of jobs).
Queues: The crossbeam-deque structures allocate internal nodes, though this is amortized.
4. Missing Infrastructure
There is zero evidence of bump allocation infrastructure in src/.

No Bump or Arena types found.
No 
reset()
 mechanism for frame boundaries.
No custom allocator API passed to jobs.
Recommendations
To meet the requirement, the system needs a fundamental architectural shift:

Introduce Frame Allocators: Implement or integrate a bump allocator (e.g., bumpalo or a custom LinearAllocator).
Change Job Storage:
Instead of Box<dyn FnOnce>, 
Job
 needs to hold a pointer/reference to a closure allocated in the frame arena.
Since standard Box governs lifecycle (drop), this is non-trivial. We might need &'a dyn FnOnce where 'a is the frame lifetime.
Per-Thread Allocators: To avoid locks, each 
Worker
 should likely have its own bump allocator for the jobs it spawns or processes.
Frame Reset: A mechanism to reset these allocators at the end of a "frame" (synchronization point) is required.
Immediate Action Item: Design a FrameAllocator integration plan to replace Box<dyn FnOnce> with arena-allocated closures.