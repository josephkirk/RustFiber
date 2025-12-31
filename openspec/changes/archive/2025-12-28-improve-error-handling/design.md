# Design for Improved Error Handling

## Error Type Design

### JobSystemError
```rust
#[derive(Debug)]
pub enum JobSystemError {
    WorkerPanic { count: usize },
    ShutdownTimeout,
    // Future: other system-level errors
}
```

### WorkerPoolError
```rust
#[derive(Debug)]
pub enum WorkerPoolError {
    WorkerPanic { failed_count: usize },
    // Future: injector errors, etc.
}
```

## Implementation Strategy
- Use `thiserror` crate for automatic `Error` and `Display` implementations
- Keep error types in the same modules as the functions that return them
- Provide `From` implementations for easy conversion from internal errors

## Backward Compatibility
Since this is a breaking change, we may need to provide wrapper functions or deprecation warnings in a future version. For now, update directly as the API is not yet stable.

## Testing
- Test panic scenarios to ensure errors are properly propagated
- Test successful shutdowns return Ok
- Verify error messages are informative