# Tasks: Add Performance Monitoring

- [x] Add `metrics` feature flag to `Cargo.toml` for optional compilation
- [x] Define `Metrics` struct in new `metrics.rs` module with atomic counters for:
  - Total jobs completed
  - Local queue pushes/pops per worker
  - Global injector pushes/pops
- [x] Integrate metrics collection into `Worker`:
  - Increment job counter on job completion
  - Track local queue operations (push/pop)
- [x] Integrate metrics collection into `JobSystem`:
  - Track global injector operations
  - Add `enable_metrics` flag to `JobSystemConfig`
- [x] Implement metrics snapshot method to query current values
- [x] Add throughput calculation (jobs/sec) based on time windows
- [x] Write unit tests for metrics collection accuracy
- [x] Write integration tests to verify metrics work in multi-threaded scenarios
- [x] Update benchmark scripts to optionally enable and report metrics
- [x] Add documentation for metrics usage in README or docs
- [x] Validate proposal with `openspec validate add-performance-monitoring --strict`