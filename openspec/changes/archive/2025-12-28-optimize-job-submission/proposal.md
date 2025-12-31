# Optimize Job Submission Overhead

## Problem
Nested job submission currently incurs significant overhead due to contention on the **Global Injector**. Even when using the lock-free **Frame Allocator**, the performance benefits are masked by the synchronization cost of `self.job_system.submit_to_injector(job)`.

## Solution
Enable `Context::spawn_job` to push directly to the executing Worker's **Local Queue** (Deque), bypassing the global lock.

## Key Changes
1.  **Expose Local Queue**: Pass a reference (pointer) to the worker's `Injector` (or `Worker<Job>`) into the `Context`.
2.  **Update Context**: Add `local_queue: Option<*const worker::Deque<Job>>` field to `Context`.
3.  **Optimize spawn_job**: Check for `local_queue`. If present, push there. Else, fall back to global injector.

## Benefits
- Drastic reduction in contention for nested parallelism.
- Unlocks the full speed potential of Frame Allocation.
