# Best Practices for RustFiber Job System

This guide provides recommendations for configuring and using the RustFiber job system effectively across different workload types.

## Choosing the Right Preset

### Gaming Workloads
Use `JobSystem::for_gaming()` for:
- Game engines with high concurrency requirements
- Entity component systems (ECS)
- Physics simulations
- Rendering pipelines

**Why it works:**
- Large fiber pools handle thousands of concurrent tasks
- AvoidSMT pinning prevents cache thrashing from hyper-threading
- Smaller stacks allow more fibers per memory budget

### Data Processing Workloads
Use `JobSystem::for_data_processing()` for:
- Batch analytics and ETL jobs
- Image/video processing
- Scientific computing
- Database operations

**Why it works:**
- Balanced configuration scales well with workload size
- Linear pinning provides predictable performance
- Standard memory settings work for most algorithms

### Low Latency Workloads
Use `JobSystem::for_low_latency()` for:
- Real-time audio processing
- Control systems and robotics
- Financial trading systems
- Network packet processing

**Why it works:**
- Small pools reduce startup time and memory usage
- CCD isolation minimizes cross-core communication latency
- Minimal fiber switching overhead

## Custom Configuration Guidelines

### Thread Count
- **Default:** Use `available_parallelism()` (all CPU cores)
- **Gaming:** All cores for maximum parallelism
- **Data Processing:** All cores for throughput
- **Low Latency:** Fewer cores if jitter is critical

### Fiber Pool Sizing
- **Initial Pool:** Start with 16-64 fibers per worker
- **Target Pool:** Allow growth to 128-512 fibers
- **Rule of thumb:** 10-100x the number of concurrent tasks

### Stack Size
- **Small (128KB):** Low latency, many fibers
- **Medium (256KB):** Gaming, general purpose
- **Large (512KB+):** Complex algorithms, recursion

### Pinning Strategies
- **None:** Default OS scheduling, most portable
- **Linear:** Predictable core assignment
- **AvoidSMT:** Better cache performance on hyper-threaded CPUs
- **CCDIsolation:** Minimal latency on multi-CCX AMD systems
- **TieredSpillover:** Adaptive for mixed workloads

## Performance Tips

### Memory Management
- Use frame allocators for temporary allocations
- Avoid heap allocations in hot paths
- Prefer `spawn_detached` for fire-and-forget tasks

### Synchronization
- Use counters for dependency tracking
- Prefer `wait_for` over polling
- Group related tasks with shared counters

### Load Balancing
- Work-stealing handles uneven workloads automatically
- Prefer small, uniform task sizes
- Use `parallel_for` for data-parallel algorithms

## Common Pitfalls

### Over-allocating Fibers
- Too many fibers increases memory usage
- Context switching overhead can hurt performance
- Start small and measure

### Ignoring NUMA
- Pinning strategies help with multi-socket systems
- Test on target hardware configurations

### Blocking Operations
- Avoid blocking I/O in job functions
- Use async I/O or separate threads for blocking work

## Monitoring and Tuning

### Key Metrics
- Job completion latency
- CPU utilization per core
- Memory usage patterns
- Context switch rates

### Profiling Tools
- Use CPU profilers to identify bottlenecks
- Monitor fiber pool utilization
- Check for work-stealing imbalances

## Migration from Other Systems

### From Thread Pools
- Increase fiber counts (threads are expensive)
- Use counters instead of channels for sync
- Leverage frame allocators for zero-allocation spawning

### From Task Schedulers
- Similar API but with fiber-based execution
- Better suited for fine-grained parallelism
- Automatic load balancing via work-stealing