# Improve Error Handling

## Summary
Replace generic error types (String, usize) with granular, structured error types to improve debugging and error handling throughout the RustFiber job system.

## Motivation
Currently, functions like `JobSystem::shutdown()` and `WorkerPool::shutdown()` return `Result<(), String>` or `Result<(), usize>`, which provide minimal information for debugging failures. By introducing specific error enums, we can:

- Provide more detailed error information
- Enable better error handling in user code
- Improve maintainability and debugging experience
- Follow Rust best practices for error handling

## Scope
- Define new error types for job system operations
- Update return types in `job_system.rs` and `worker.rs`
- Ensure backward compatibility or provide migration path
- Update any dependent code

## Impact
- Breaking change for shutdown methods
- Improved error messages and debugging
- No performance impact
