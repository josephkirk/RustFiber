# Work Stealing Design Updates

## Randomized Back-off

### Context
Workers currently use `crossbeam::utils::Backoff` which provides spin and yield hints but is deterministic. When many workers are idle, they effectively synchronize their polling loops ("thundering herd"), causing excessive contention on the victim queues.

### Design
- **Exponential Back-off with Jitter**: When a steal attempt fails, the worker should wait for a randomized duration before retrying.
- **Implementation**:
    - Modify `src/worker.rs`.
    - In the `None => { ... }` branch of the work loop (when no work is found):
        - If `backoff.is_completed()`:
            - Instead of just `thread::yield_now()`, introduce a randomized sleep or yield loop.
            - `thread::sleep(Duration::from_micros(ramdom_range(1, 10)))` might be too heavy (min 1ms on Windows usually).
            - A lighter approach: randomized spin loop count before yielding.
    - **Preferred Approach**: Use `thread::yield_now()` combined with a randomized condition to skip steal attempts in the next iteration, effectively reducing the frequency of polling.

## Visualization

### Context
The current categorical bar chart for steal attempts uses a linear scale, making it hard to see comparison between low and high core counts when the "Empty Attempts" are massive. Labels also overlap.

### Design
- **Logarithmic Scale**: Set Y-axis to log scale for "Steal Attempts".
- **Label Placement**:
    - Adjust label positioning or rotation.
    - Only show labels if there is enough vertical space, or use a separate table/legend.
- **Implementation**: Modify `run_benchmarks.py`.
