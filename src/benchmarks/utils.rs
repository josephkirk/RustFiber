use serde::{Deserialize, Serialize};

pub const DEFAULT_TIMEOUT_SECS: u64 = 60;

#[derive(Debug, Serialize, Deserialize)]
pub struct BenchmarkResult {
    pub name: String,
    pub data_points: Vec<DataPoint>,
    pub system_info: SystemInfo,
    pub crashed: bool,
    pub crash_point: Option<usize>,
    pub timed_out: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DataPoint {
    pub num_tasks: usize,
    pub time_ms: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SystemInfo {
    pub cpu_cores: usize,
    pub total_memory_gb: f64,
}

impl SystemInfo {
    pub fn collect() -> Self {
        // Get CPU count
        let cpu_cores = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);

        // Get total memory - use sysinfo if available, otherwise estimate
        let total_memory_gb = Self::get_total_memory_gb();

        SystemInfo {
            cpu_cores,
            total_memory_gb,
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
