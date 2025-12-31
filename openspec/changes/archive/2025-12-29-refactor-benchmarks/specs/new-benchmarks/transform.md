# Transform Hierarchy Benchmark Spec

## Overview
Simulates real-world game loop transform hierarchy updates.

## Test Cases
- Build hierarchy with depth 5, 10 children per node
- Update in topological order (leaves to root)
- Vary number of update cycles

## Metrics
- Time per update (ms)
- Total time for hierarchy

## Implementation
- Flat array of transforms with parent indices
- Process levels in reverse order using job system
- Each job updates based on parent position