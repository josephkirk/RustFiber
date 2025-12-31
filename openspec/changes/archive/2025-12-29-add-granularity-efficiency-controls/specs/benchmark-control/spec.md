## ADDED Requirements

### Requirement: Efficiency Visualization
The benchmark visualization system MUST plot Parallel Efficiency ($\frac{Speedup}{Threads}$) on a secondary Y-axis for scaling benchmarks.

#### Scenario: ParallelFor Scaling with Efficiency Axis
- **WHEN** the ParallelFor Scaling benchmark is visualized
- **THEN** the graph MUST show:
  - Primary Y-axis: Speedup (x factor)
  - Secondary Y-axis: Efficiency (0.0 to 1.0)
  - Efficiency plotted as a distinct line style (e.g., dashed)

#### Scenario: Efficiency Calculation
- **GIVEN** normalized speedup values at various thread counts
- **WHEN** efficiency is computed
- **THEN** Efficiency = Speedup / Threads for each data point

#### Scenario: Visual Distinction
- **WHEN** both Speedup and Efficiency are plotted
- **THEN** they MUST use distinct colors and line styles to avoid confusion
- **AND** a legend clearly labeling both metrics MUST be present
