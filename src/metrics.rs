#[cfg(feature = "metrics")]
use std::sync::atomic::{AtomicU64, Ordering};
#[cfg(feature = "metrics")]
use std::time::Instant;

/// Optional performance metrics for the job system.
#[cfg(feature = "metrics")]
#[derive(Debug)]
pub struct Metrics {
    /// Total number of jobs completed.
    pub jobs_completed: AtomicU64,
    /// Total pushes to local worker queues.
    pub local_queue_pushes: AtomicU64,
    /// Total pops from local worker queues.
    pub local_queue_pops: AtomicU64,
    /// Total pushes to global injector.
    pub global_injector_pushes: AtomicU64,
    /// Total pops from global injector.
    pub global_injector_pops: AtomicU64,
    /// Successful steals from other workers.
    pub worker_steals_success: AtomicU64,
    /// Failed steal attempts from other workers.
    pub worker_steals_failed: AtomicU64,
    /// Steal attempts from other workers that need retry (contention).
    pub worker_steals_retry: AtomicU64,
    /// Successful steals from global injector.
    pub injector_steals_success: AtomicU64,
    /// Failed steal attempts from global injector.
    pub injector_steals_failed: AtomicU64,
    /// Steal attempts from global injector that need retry (contention).
    pub injector_steals_retry: AtomicU64,
    /// Time when metrics collection started.
    pub start_time: Instant,
}

#[cfg(feature = "metrics")]
impl Metrics {
    /// Creates a new metrics instance.
    pub fn new() -> Self {
        Self {
            jobs_completed: AtomicU64::new(0),
            local_queue_pushes: AtomicU64::new(0),
            local_queue_pops: AtomicU64::new(0),
            global_injector_pushes: AtomicU64::new(0),
            global_injector_pops: AtomicU64::new(0),
            worker_steals_success: AtomicU64::new(0),
            worker_steals_failed: AtomicU64::new(0),
            worker_steals_retry: AtomicU64::new(0),
            injector_steals_success: AtomicU64::new(0),
            injector_steals_failed: AtomicU64::new(0),
            injector_steals_retry: AtomicU64::new(0),
            start_time: Instant::now(),
        }
    }

    /// Returns a snapshot of current metrics values.
    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            jobs_completed: self.jobs_completed.load(Ordering::Relaxed),
            local_queue_pushes: self.local_queue_pushes.load(Ordering::Relaxed),
            local_queue_pops: self.local_queue_pops.load(Ordering::Relaxed),
            global_injector_pushes: self.global_injector_pushes.load(Ordering::Relaxed),
            global_injector_pops: self.global_injector_pops.load(Ordering::Relaxed),
            worker_steals_success: self.worker_steals_success.load(Ordering::Relaxed),
            worker_steals_failed: self.worker_steals_failed.load(Ordering::Relaxed),
            worker_steals_retry: self.worker_steals_retry.load(Ordering::Relaxed),
            injector_steals_success: self.injector_steals_success.load(Ordering::Relaxed),
            injector_steals_failed: self.injector_steals_failed.load(Ordering::Relaxed),
            injector_steals_retry: self.injector_steals_retry.load(Ordering::Relaxed),
            elapsed_seconds: self.start_time.elapsed().as_secs_f64(),
        }
    }
}

/// Snapshot of metrics at a point in time.
#[cfg(feature = "metrics")]
#[derive(Debug, Clone)]
pub struct MetricsSnapshot {
    pub jobs_completed: u64,
    pub local_queue_pushes: u64,
    pub local_queue_pops: u64,
    pub global_injector_pushes: u64,
    pub global_injector_pops: u64,
    pub worker_steals_success: u64,
    pub worker_steals_failed: u64,
    pub worker_steals_retry: u64,
    pub injector_steals_success: u64,
    pub injector_steals_failed: u64,
    pub injector_steals_retry: u64,
    pub elapsed_seconds: f64,
}

#[cfg(feature = "metrics")]
impl MetricsSnapshot {
    /// Calculates jobs per second throughput.
    pub fn jobs_per_second(&self) -> f64 {
        if self.elapsed_seconds > 0.0 {
            self.jobs_completed as f64 / self.elapsed_seconds
        } else {
            0.0
        }
    }

    /// Approximates local queue depth (pushes - pops).
    pub fn local_queue_depth(&self) -> i64 {
        self.local_queue_pushes as i64 - self.local_queue_pops as i64
    }

    /// Approximates global injector depth (pushes - pops).
    pub fn global_injector_depth(&self) -> i64 {
        self.global_injector_pushes as i64 - self.global_injector_pops as i64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_metrics_creation() {
        let metrics = Metrics::new();
        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.jobs_completed, 0);
        assert_eq!(snapshot.local_queue_pushes, 0);
        assert_eq!(snapshot.worker_steals_success, 0);
        assert_eq!(snapshot.worker_steals_failed, 0);
        assert_eq!(snapshot.worker_steals_retry, 0);
        assert_eq!(snapshot.injector_steals_success, 0);
        assert_eq!(snapshot.injector_steals_failed, 0);
        assert_eq!(snapshot.injector_steals_retry, 0);
        assert!(snapshot.elapsed_seconds >= 0.0);
    }

    #[test]
    fn test_metrics_updates() {
        let metrics = Metrics::new();

        metrics.jobs_completed.fetch_add(5, Ordering::Relaxed);
        metrics.local_queue_pushes.fetch_add(10, Ordering::Relaxed);
        metrics.local_queue_pops.fetch_add(8, Ordering::Relaxed);

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.jobs_completed, 5);
        assert_eq!(snapshot.local_queue_pushes, 10);
        assert_eq!(snapshot.local_queue_pops, 8);
        assert_eq!(snapshot.local_queue_depth(), 2);
    }

    #[test]
    fn test_throughput_calculation() {
        let metrics = Metrics::new();
        metrics.jobs_completed.fetch_add(100, Ordering::Relaxed);

        // Simulate 1 second elapsed
        thread::sleep(Duration::from_millis(10)); // Small sleep for test
        let snapshot = metrics.snapshot();

        assert!(snapshot.jobs_per_second() > 0.0);
    }
}
