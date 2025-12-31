# Design: RAII Guard and Panic Safety

## CounterGuard
The `CounterGuard` takes ownership of the reference to `AtomicCounter`.
On `Drop`, it signals the counter.
This ensures that even during stack unwinding (panic), the signal is sent.

## Panic Handling
We use `std::panic::catch_unwind` around the job closure execution.
Since fibers are stackful, a panic inside the fiber unwinds the fiber stack.
We must catch this panic *inside* the fiber to:
1.  Allow `CounterGuard` to drop (it drops during unwind).
2.  Prevent `resume` from propagating the panic (though `resume` catches it too).
3.  Mark fiber as "poisoned" or reset it properly.

## Integration
`Job` struct currently holds the closure.
We will modify the execution path to:
```rust
// Concept
{
    let _guard = CounterGuard::new(counter, system);
    let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
        (job.func)();
    }));
}
// _guard drops here
```
