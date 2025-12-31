# Cache Alignment

## ADDED Requirements

### Requirement: Critical synchronization structures MUST be cache-aligned
Structures involved in high-frequency atomic operations or fine-grained synchronization MUST be aligned to cache line boundaries to prevent false sharing.

#### Scenario: InnerCounter Alignment
Given the `InnerCounter` struct used in `Counter`
When `std::mem::align_of::<InnerCounter>()` is checked
Then it should be at least 128 bytes to ensure isolation in `Arc` allocations.

#### Scenario: WaitNode Alignment
Given the `WaitNode` struct used in fibers and waiting
When `std::mem::align_of::<WaitNode>()` is checked
Then it should be at least 64 bytes to minimize cache line sharing between adjacent nodes.
