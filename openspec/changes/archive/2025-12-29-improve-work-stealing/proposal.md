# Improve Work Stealing Efficiency

## Summary
Investigate and resolve the near-0% success rate for work steals observed in benchmarks, which indicates a "thundering herd" problem where idle threads saturate the memory bus with failed attempts. Introduce a randomized back-off mechanism to mitigate contention and improve the benchmark visualization to better represent these metrics.

## Problem Statement
Current stress benchmarks show a massive spike in failed steal attempts at higher thread counts (16T+), with success rates approaching 0.0%. This suggests that idle workers are aggressively polling empty queues without sufficient back-off, leading to cache thrashing and bus saturation. Additionally, the current benchmark visualization creates overlapping labels and makes it difficult to distinguish between 1T and 32T attempt counts due to the linear scale.

## Proposed Solution
1.  **Randomized Back-off**: Implement a randomized wait or yield strategy in the worker loop when stealing fails, preventing synchronized polling loops.
2.  **Visualization Improvements**: Update `run_benchmarks.py` to use a logarithmic scale for steal attempts and fix overlapping labels.

## Risks
- **Latency**: Excessive back-off could increase latency for waking up workers when new work arrives. The back-off strategy must be carefully tuned.
- **Fairness**: Randomization might lead to some workers staying idle longer than others, though this is generally acceptable for work-stealing.
