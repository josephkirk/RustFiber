# Proposal: Tiered Spillover Pinning Strategies

## Goal
Implement a dynamic thread pinning and utilization strategy that prioritizes high-performance physical cores and only "spills over" to secondary cores or SMT threads when the system is heavily loaded.

## Problem
Modern multi-CCD processors (like AMD Ryzen) have non-uniform memory and cache access:
1. **L3 Locality**: Cores on the same CCD share a fast L3 cache. Crossing to another CCD introduces latency.
2. **SMT Fighting**: SMT threads (siblings) share execution units. Heavy compute on both siblings often results in lower total throughput than one physical core.
3. **Power/Thermal**: Running all cores at low utilization is less efficient than running a few cores at high frequency.

Current pinning strategies are static (e.g., `Linear`, `AvoidSMT`, `CCDIsolation`). There is no mechanism to dynamically scale the "active" worker set based on demand.

## Proposed Solution
Introduce `PinningStrategy::TieredSpillover` which organizes workers into three priority tiers:

- **Tier 1 (Core)**: 8 physical cores on CCD0 (Logical 0, 2, ..., 14). Always active.
- **Tier 2 (Expansion)**: 8 physical cores on CCD1 (Logical 16, 18, ..., 30). Wakes up when Tier 1 saturation > 80%.
- **Tier 3 (SMT)**: Logical SMT threads (Logical 1, 3, ..., 31). Wakes up when Tier 2 is also saturated.

A shared `AtomicUsize` in the `WorkerPool` will track the count of currently executing jobs. Tiers 2 and 3 will use this load metric to decide whether to participate in work-stealing or remain in a low-power "parked" state.

## Benefits
- Maximizes L3 cache hits for small/medium workloads.
- Avoids SMT performance degradation for compute-heavy tasks unless absolutely necessary.
- Improves efficiency on high-core-count systems.
