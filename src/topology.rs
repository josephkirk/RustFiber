use sysinfo::{System, CpuRefreshKind};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Topology {
    pub core_to_node: HashMap<usize, usize>,
    pub node_cores: HashMap<usize, Vec<usize>>,
    pub num_nodes: usize,
}

impl Topology {
    pub fn detect() -> Self {
        let mut system = System::new();
        system.refresh_cpu_specifics(CpuRefreshKind::everything());
        
        let mut core_to_node = HashMap::new();
        let mut node_cores = HashMap::new();
        
        // Use sysinfo to detect CPUs.
        // Since reliable NUMA ID detection is tricky without specialized crates/FFI on some platforms,
        // we use a best-effort approach.
        // For widely used x86 platforms, we can sometimes infer or just default to Node 0 if not exposed.
        // Note: Real production code would use `hwloc`.
        
        let cpus = system.cpus();
        
        // Simple heuristic: If we can't find explicit node info, check if `vendor_id` or other props differ?
        // Unlikely. We'll iterate and map 1-to-1 if possible or default to 0.
        // For this implementation, we map all to 0 to be safe (Uniform), 
        // as incorrect NUMA splitting is worse than Uniform.
        
        for (i, _cpu) in cpus.iter().enumerate() {
            let node_id = 0; // Placeholder until hwloc binding
            
            core_to_node.insert(i, node_id);
            node_cores.entry(node_id).or_insert_with(Vec::new).push(i);
        }

        Topology {
            core_to_node,
            node_cores,
            num_nodes: 1, // Default to 1 until better detection
        }
    }
    
    // Helper to get siblings for a worker
    pub fn get_siblings(&self, worker_id: usize) -> Option<&Vec<usize>> {
        let node = self.core_to_node.get(&worker_id)?;
        self.node_cores.get(node)
    }
}
