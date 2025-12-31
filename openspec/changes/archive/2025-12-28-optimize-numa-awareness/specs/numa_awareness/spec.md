# Spec: NUMA Awareness

## ADDED Requirements

### Requirement: Topology Discovery
The system MUST detect the underlying hardware topology to identify NUMA nodes and core associations.
#### Scenario: Multi-Socket System
- **Given** a system with 2 sockets (NUMA nodes) and 16 cores each (32 total).
- **When** the JobSystem is initialized.
- **Then** it MUST correctly identify 2 NUMA nodes and map cores 0-15 to Node 0 and cores 16-31 to Node 1.

### Requirement: NUMA-Local Memory Allocation
Memory-intensive structures (Stacks, Frame Allocators) MUST be allocated on the same NUMA node as the worker thread that uses them.
#### Scenario: Worker Initialization
- **Given** a Worker pinned to Core 0 (Node 0).
- **When** the Worker initializes its `FiberPool` and `FrameAllocator`.
- **Then** the backing memory pages MUST be physically allocated on Node 0 (using First-Touch policy by writing to them from the pinned thread).

### Requirement: Topology-Aware Work Stealing
Workers MUST prioritize stealing from siblings on the same NUMA node before attempting to steal from remote nodes.
#### Scenario: Local Starvation
- **Given** Worker A on Node 0 has no tasks.
- **And** Worker B on Node 0 has 10 tasks.
- **And** Worker C on Node 1 has 10 tasks.
- **When** Worker A attempts to steal.
- **Then** it MUST try to steal from Worker B first.

#### Scenario: Global Starvation
- **Given** Worker A on Node 0 has no tasks.
- **And** Worker B on Node 0 has no tasks.
- **And** Worker C on Node 1 has 10 tasks.
- **When** Worker A attempts to steal.
- **Then** it MUST fallback to steal from Worker C (remote steal).
