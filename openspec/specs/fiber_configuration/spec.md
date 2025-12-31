# fiber_configuration Specification

## Purpose
TBD - created by archiving change implement-fiber-yielding-gaps. Update Purpose after archive.
## Requirements
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

### Requirement: Builder Pattern API
The system SHALL provide a `JobSystemBuilder` for gradual configuration of job system parameters.
#### Scenario: Gradual Configuration
- **Given** a user wants to customize only thread count and pinning
- **When** using `JobSystem::builder().thread_count(8).pinning_strategy(AvoidSMT).build()`
- **Then** creates job system with 8 threads, AvoidSMT pinning, and default fiber config

### Requirement: Preset Configurations
The system SHALL provide preset constructors for common use cases.
#### Scenario: Gaming Preset
- **Given** a game developer needs high concurrency
- **When** calling `JobSystem::for_gaming()`
- **Then** creates system optimized for gaming (large pools, avoid SMT, etc.)

#### Scenario: Data Processing Preset
- **Given** a data processing application
- **When** calling `JobSystem::for_data_processing()`
- **Then** creates balanced configuration for batch processing

### Requirement: Best Use Case Documentation
The library SHALL document optimal configurations for different workload types.
#### Scenario: Documentation Access
- **Given** a user reads the library docs
- **When** looking at examples and best practices
- **Then** finds clear guidance on when to use each preset and configuration option

