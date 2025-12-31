# Design for Simplified Config and API

## Builder Pattern
Introduce `JobSystemBuilder` struct with methods like:
- `with_thread_count(n)` 
- `with_fiber_config(config)`
- `with_pinning_strategy(strategy)`
- `build() -> Result<JobSystem>`

This allows gradual configuration without requiring all parameters upfront.

## Presets
Define presets based on common use cases:
- **Gaming**: High concurrency, avoid SMT, larger pools
- **Data Processing**: Balanced, linear pinning, standard pools  
- **Low Latency**: Smaller stacks, minimal pools, CCD isolation

Each preset tunes FiberConfig and PinningStrategy appropriately.

## Zero-Cost Abstractions
- Builder methods are `#[inline]` and compile to same code as direct construction
- Presets are just const functions returning pre-configured structs
- No runtime overhead for configuration

## Error Handling
Builder validates configuration at build time (e.g., thread count > 0, stack size reasonable).

## Backwards Compatibility
All existing constructors remain unchanged. New API is additive.