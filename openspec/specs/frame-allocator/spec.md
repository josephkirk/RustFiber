# frame-allocator Specification

## Purpose
TBD - created by archiving change implement-frame-linear-allocator. Update Purpose after archive.
## Requirements
### Requirement: Bump Allocation Strategy {id: R-FRAME-ALLOC}
The system MUST provide a mechanism to allocate memory linearly (bump pointer) to strictly avoid lock contention during allocation.

#### Scenario: High-Frequency Job Spawning
Given a high-performance game loop spawning 10,000 jobs per frame
When the jobs are created using the Frame Allocator
Then no system-level locks (mutex/futex) are acquired for memory allocation
And the allocation time is O(1) (pointer increment).

### Requirement: Frame Reset Mechanism {id: R-FRAME-RESET}
The allocator MUST support a `reset` operation that invalidates all previous allocations and reclaims the entire memory block instantly.

#### Scenario: End of Frame
Given a frame where all jobs have completed
When `reset()` is called
Then the allocation pointer returns to the base address
And the memory is ready for reuse in the next frame
And no individual `free()` calls are made.

### Requirement: Job System Integration {id: R-JOB-INTEGRATION}
The `Job` structure MUST support storage of work closures executed from the frame allocator.

#### Scenario: Running a Frame Job
Given a job allocated in the frame allocator
When `Job::execute` is called
Then the closure is invoked correctly
And the memory handling (drop/dealloc) respects the frame allocator rules (no-op drop).

