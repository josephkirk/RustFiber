use rustfiber::topology::Topology;

fn main() {
    let topo = Topology::detect();
    println!("Detected {} NUMA nodes", topo.num_nodes);
    println!("Core -> Node map: {:?}", topo.core_to_node);
    println!("Node -> Cores map: {:?}", topo.node_cores);
}
