# Proposal: Implement Fiber Yielding Gaps
`change-id: implement-fiber-yielding-gaps`

## Summary
Complete the partial implementation of the fiber yielding system by adding priority scheduling, runtime configuration, and comprehensive validation tests.

## Problem Statement
The current fiber yielding implementation has several gaps identified in `docs/yiedingfinding.md`:
1.  **Missing Prioritization**: All jobs are treated equally, which is inefficient for game engines where some tasks (physics, input variables) are more critical than others (AI, background loading).
2.  **Hardcoded Configuration**: Stack sizes and pool limits are hardcoded, preventing tuning for different hardware or game types.
3.  **Insufficient Validation**: Lack of specific tests for fiber fairness and interleaving guarantees correctness only by coincidence.

## Proposed Solution
1.  **Add `JobPriority` Enum**: Introduce `High`, `Normal`, `Low` priorities.
2.  **Update Scheduler**: Modify `Worker` to handle multiple queues or a priority-aware backing store.
3.  **Configurable Stack Sizes**: Pass `FiberConfig` to `JobSystem::new`.
4.  **Verification Suite**: Add `test_fiber_fairness` and `test_stack_persistence`.

## Impact
- **Performance**: Better CPU utilization by processing critical tasks first.
- **Flexibility**: Usable on constrained platforms (smaller stacks) or heavy workloads (larger stacks).
- **Reliability**: Proven correctness of the cooperative scheduling implementation.
