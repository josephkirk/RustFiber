# fiber_validation Specification

## Purpose
TBD - created by archiving change implement-fiber-yielding-gaps. Update Purpose after archive.
## Requirements
### Requirement: Yield Fairness
The system SHALL demonstrate that yielding fibers allow other ready fibers on the same thread to execute.
#### Scenario: Interleaved Execution
- **Given** two fibers on a single-threaded worker.
- **When** both fibers loop and call `yield_now`.
- **Then** their execution steps are interleaved in the output log.

### Requirement: Stack Persistence
The system SHALL prove that local variables on the stack are preserved across yields.
#### Scenario: Variable Retention
- **Given** a fiber with a local variable `x = 42`.
- **When** it yields and is resumed later.
- **Then** `x` is still `42`.

