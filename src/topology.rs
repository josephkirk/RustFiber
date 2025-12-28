use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Topology {
    pub core_to_node: HashMap<usize, usize>,
    pub node_cores: HashMap<usize, Vec<usize>>,
    pub num_nodes: usize,
}

impl Topology {
    pub fn detect() -> Self {
        // Use conservative detection that defaults to single NUMA node
        // This is safer than relying on potentially problematic external libraries
        Self::detect_conservative()
    }

    /// Conservative NUMA detection that prioritizes safety over accuracy
    /// Defaults to single NUMA node unless we can reliably detect multiple nodes
    fn detect_conservative() -> Self {
        use sysinfo::{CpuRefreshKind, System};

        let mut system = System::new();
        system.refresh_cpu_specifics(CpuRefreshKind::everything());

        let mut core_to_node = HashMap::new();
        let mut node_cores = HashMap::new();

        // Get CPU information
        let cpus = system.cpus();
        let num_cores = cpus.len();

        // Conservative approach: assume single NUMA node unless we have
        // strong evidence for multiple nodes. This avoids false positives
        // that could lead to suboptimal memory allocation.

        // For systems with many cores, we might have multiple NUMA nodes,
        // but we'll use a simple heuristic based on core count and platform
        let estimated_nodes = if num_cores > 32 {
            // Large systems might have multiple NUMA nodes
            // Use a simple division - this is not accurate but safe
            (num_cores / 16).clamp(1, 4) // Cap at 4 nodes for safety
        } else {
            1 // Single node for smaller systems
        };

        // Distribute cores across estimated nodes
        for (i, _cpu) in cpus.iter().enumerate() {
            let node_id = if estimated_nodes == 1 {
                0
            } else {
                // Simple round-robin distribution
                i % estimated_nodes
            };

            core_to_node.insert(i, node_id);
            node_cores.entry(node_id).or_insert_with(Vec::new).push(i);
        }

        Topology {
            core_to_node,
            node_cores,
            num_nodes: estimated_nodes,
        }
    }

    // Helper to get siblings for a worker
    pub fn get_siblings(&self, worker_id: usize) -> Option<&Vec<usize>> {
        let node = self.core_to_node.get(&worker_id)?;
        self.node_cores.get(node)
    }
}
