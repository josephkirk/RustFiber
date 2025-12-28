use rustfiber::PinningStrategy;
use serde::{Deserialize, Serialize};

pub const DEFAULT_TIMEOUT_SECS: u64 = 60;

#[derive(Debug, Serialize, Deserialize)]
pub struct InternalMetrics {
    pub worker_steals_success: u64,
    pub worker_steals_failed: u64,
    pub worker_steals_retry: u64,
    pub injector_steals_success: u64,
    pub injector_steals_failed: u64,
    pub injector_steals_retry: u64,
    pub parks: u64,
    pub wakeups: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BenchmarkResult {
    pub name: String,
    pub data_points: Vec<DataPoint>,
    pub system_info: SystemInfo,
    pub crashed: bool,
    pub crash_point: Option<usize>,
    pub timed_out: bool,
    #[cfg(feature = "metrics")]
    pub internal_metrics: Option<InternalMetrics>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DataPoint {
    pub num_tasks: usize,
    pub time_ms: f64,
    #[serde(default)]
    pub metric_type: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SystemInfo {
    pub cpu_cores: usize,
    pub total_memory_gb: f64,
    pub pinning_strategy: PinningStrategy,
}

impl SystemInfo {
    pub fn collect(strategy: PinningStrategy, cpu_cores: usize) -> Self {
        // Get total memory - use sysinfo if available, otherwise estimate
        let total_memory_gb = Self::get_total_memory_gb();

        SystemInfo {
            cpu_cores,
            total_memory_gb,
            pinning_strategy: strategy,
        }
    }

    fn get_total_memory_gb() -> f64 {
        use sysinfo::System;
        let mut sys = System::new_all();
        sys.refresh_memory();
        sys.total_memory() as f64 / (1024.0 * 1024.0 * 1024.0)
    }
}

pub fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
}

#[cfg(feature = "metrics")]
pub fn capture_internal_metrics(job_system: &rustfiber::JobSystem) -> Option<InternalMetrics> {
    job_system.metrics().map(|m| InternalMetrics {
        worker_steals_success: m.worker_steals_success,
        worker_steals_failed: m.worker_steals_failed,
        worker_steals_retry: m.worker_steals_retry,
        injector_steals_success: m.injector_steals_success,
        injector_steals_failed: m.injector_steals_failed,
        injector_steals_retry: m.injector_steals_retry,
        parks: 0,
        wakeups: 0,
    })
}
