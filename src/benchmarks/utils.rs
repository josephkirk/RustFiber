use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct BenchmarkResult {
    pub name: String,
    pub data_points: Vec<DataPoint>,
    pub system_info: SystemInfo,
    pub crashed: bool,
    pub crash_point: Option<usize>,
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

    #[cfg(target_os = "linux")]
    fn get_total_memory_gb() -> f64 {
        use std::fs;

        // Try to read from /proc/meminfo
        if let Ok(contents) = fs::read_to_string("/proc/meminfo") {
            for line in contents.lines() {
                if line.starts_with("MemTotal:")
                    && let Some(kb_str) = line.split_whitespace().nth(1)
                    && let Ok(kb) = kb_str.parse::<f64>()
                {
                    return kb / (1024.0 * 1024.0); // Convert KB to GB
                }
            }
        }
        8.0 // Default fallback
    }

    #[cfg(not(target_os = "linux"))]
    fn get_total_memory_gb() -> f64 {
        8.0 // Default fallback for non-Linux systems
    }
}

pub fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
}
