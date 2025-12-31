# Tasks: Optimize Job Submission

- [x] Add `local_queue` field to `Context` struct (Option<Pointer>) <!-- id: 0 -->
- [x] Update `Context::new` to accept optional local queue pointer <!-- id: 1 -->
- [x] Update `Worker::run_loop` to pass local queue when creating `Context` <!-- id: 2 -->
- [x] Update `Context::spawn_job` to push to local queue if available <!-- id: 3 -->
- [x] Run benchmarks to verify speedup <!-- id: 4 -->
