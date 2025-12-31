# Design: Local Queue Access

## Architectural Change
The `Context` struct is the bridge between the Job System and the user's closure. Currently, it only provides access to global services (via `JobSystem`) and the thread-local allocator.

To support efficient work-stealing, we need to allow the `Context` to submit work to the specific *Worker* executing the current fiber.

### Lifetime and Safety
- The `Context` is transient; it lives only as long as the closure execution.
- The `Worker` (and its `local_queue`) lives for the duration of the thread.
- Therefore, passing a raw pointer to the `local_queue` is safe *within the scope of job execution*.
- We must wrap this pointer in a `SendPtr` equivalent or ensure `Context` remains `!Send` (it effectively references thread-local state). Wait, `Context` is passed to `FnOnce(&Context)`, so it's borrowed.

### Implementation Details
- `local_queue` type: `crossbeam::deque::Worker<Job>`.
- `Context` update:
  ```rust
  pub struct Context<'a> {
      job_system: &'a JobSystem,
      allocator: Option<SendPtr<FrameAllocator>>,
      local_queue: Option<SendPtr<Deque<Job>>>, // New field
  }
  ```
- `spawn_job` logic:
  ```rust
  if let Some(queue) = self.local_queue {
      unsafe { (*queue.0).push(job); }
  } else {
      self.job_system.submit_to_injector(job);
  }
  ```

## Dependencies
- `crossbeam::deque` is already used.
