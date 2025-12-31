# work_stealing Spec Delta

## ADDED Requirements

### Requirement: Randomized Back-off
Workers SHALL implement a randomized back-off or wait duration when a steal attempt fails, to prevent synchronized polling loops (thundering herd).

#### Scenario: Contention Mitigation
- **Given** multiple idle workers attempting to steal from the same victim (or any victim).
- **When** a steal attempt fails (returns Empty or Retry).
- **Then** the worker MUST wait for a randomized duration (or perform a randomized number of spin loops) before the next attempt.
- **And** this duration SHOULD increase exponentially with repeated failures (up to a limit) or be sufficiently random to desynchronize threads.

## MODIFIED Requirements

### Requirement: Deep Idle
(Refined to explicitly mention randomized entry or backoff)
Workers SHALL yield the CPU or park when no work is found after a defined search duration, incorporating randomized back-off before entering deep sleep.
