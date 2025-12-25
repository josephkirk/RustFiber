# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **Tiered Spillover Pinning**: Dynamic core activation strategy that prioritizes high-performance physical CCD0 cores and spills over to CCD1 and SMT threads only when utilization exceeds 80%.
- **Advanced Thread Pinning**: Introduced `PinningStrategy` enum supporting `Linear`, `AvoidSMT`, and `CCDIsolation` to optimize for various x86 and Ryzen topologies.
- **Multi-Core Benchmark Suite**: Extended the Python runner to automatically iterate and test across 1, 4, 16, 32, 64, and 96 core counts.
- **Comparison Graphing**: Implemented multi-series plotting in the benchmark suite for real-time performance comparison between core counts and strategies.
- **Benchmark Control**: Added 1-minute timeouts to Rust benchmark functions and incremental JSON streaming to prevent hangs and provide immediate feedback.
- **Hardware Detection**: Integrated `sysinfo` for accurate system RAM and core count reporting.
- **Active Worker Tracking**: Added real-time load monitoring to the `WorkerPool`.

### Changed
- **Work-Stealing Scheduler**: Migrated from simple MPSC channels to a robust work-stealing implementation using `crossbeam-deque`.
- **Documentation Overhaul**: Completely rewritten `README.md`, `BENCHMARKS.md`, and `ARCHITECTURE.md` to reflect the new engine capabilities.
- **Worker Refactoring**: Simplified `Worker` initialization by grouping parameters into a configuration struct, satisfying Clippy's argument limit.

### Fixed
- **Clippy & Formatting**: Resolved all static analysis warnings (Too many arguments, private interfaces) and applied global `cargo fmt`.
- **Producer-Consumer Deadlock**: Fixed distribution logic in the producer-consumer benchmark to handle uneven thread/job counts without hanging.

---

## [8cabfca] - 2025-12-25
### Changed
- Update README with benchmark images and correct numbering.

---

## [4f493b8] - 2025-12-25
### Changed
- Optimize job scheduler: work-stealing queues, batch submission, CPU affinity (#3).

---

## [82bbc80] - 2025-12-25
### Added
- Add benchmark scripts for performance testing with Python visualization (#2).

---

## [ad27c06] - 2025-12-25
### Added
- Build high-performance fiber-based job system in Rust (#1).

---

## [3be1aa4] - 2025-12-25
### Added
- Initial commit.
