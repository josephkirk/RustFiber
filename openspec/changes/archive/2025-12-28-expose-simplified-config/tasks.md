# Tasks for Expose Simplified Config and API

- [x] **Add Builder Pattern**: Implement `JobSystemBuilder` with fluent API for configuring fiber settings, pinning strategy, and thread count.

- [x] **Add Preset Constructors**: Create `JobSystem::for_gaming()`, `JobSystem::for_data_processing()`, `JobSystem::for_low_latency()` with optimized defaults.

- [x] **Update Documentation**: Enhance lib.rs examples with best use case scenarios and configuration guidance.

- [x] **Add Best Practices Guide**: Create a new doc file `docs/best_practices.md` documenting optimal configurations for different workloads.

- [x] **Validate Performance**: Run benchmarks to ensure new API has zero overhead compared to direct construction.

- [x] **Update Tests**: Add tests for new constructors and builder pattern.

- [x] **Update README**: Add usage examples showcasing simplified API.