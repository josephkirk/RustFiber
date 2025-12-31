# Fiber Configuration Specification

## Purpose
Allow developers to tune the fiber system (stack size, pool size) without modifying engine source code.

## ADDED Requirements
### Requirement: Configurable Stack
The system SHALL allow the default stack size to be specified at `JobSystem` initialization.
#### Scenario: Small Stack Optimization
- **Given** a system with limited memory (e.g., console).
- **When** `JobSystem` is initialized with a 32KB stack size config.
- **Then** all created fibers use 32KB stacks instead of the hardcoded default.

### Requirement: Pool Tuning
The system SHALL allow configuring the initial and maximum size of the fiber pool.
#### Scenario: High Concurrency
- **Given** a game that spawns 20,000 fibers per frame.
- **When** `JobSystem` is initialized with a large initial pool size.
- **Then** no new allocations occur during the first frame of execution.
