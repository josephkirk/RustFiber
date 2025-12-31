# Design: Optimize Work Stealing

## Architecture
The system uses a `Chase-Lev Deque` per worker.
- **Local Access**: `pop()` from bottom (LIFO). Optimizes for cache locality by processing the most recently spawned task (likely hot in cache).
- **Stealing**: `steal()` from top (FIFO). Thieves take the oldest task, which in recursive algorithms is likely the largest "chunk" of work, amortizing the cost of stealing.

## State Machine
Workers transition through states:
1.  **Active**: Popping local work.
2.  **Searching**: Stealing from others or popping from Injector.
3.  **Brief Idle**: `spin_loop` (short duration) to maintain responsiveness.
4.  **Deep Idle**: `yield_now` or `park` (long duration) to save energy.

## Trade-offs
- **Spin vs Park**: Spinning burns CPU but reacts instantly. Parking saves CPU but has high wake-up latency. We adopt a hybrid approach: spin briefly, then yield/park.
