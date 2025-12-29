use rustfiber::JobSystem;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

#[test]
fn test_topology_detection() {
    let topology = rustfiber::topology::Topology::detect();

    // Basic validation
    assert!(
        topology.num_nodes > 0,
        "Should detect at least one NUMA node"
    );
    assert!(
        !topology.core_to_node.is_empty(),
        "Should map cores to nodes"
    );
    assert!(
        !topology.node_cores.is_empty(),
        "Should have node-to-cores mapping"
    );

    // Validate mappings are consistent
    for (core_id, &node_id) in &topology.core_to_node {
        assert!(
            topology.node_cores.contains_key(&node_id),
            "Node {} should exist in node_cores",
            node_id
        );
        assert!(
            topology.node_cores[&node_id].contains(core_id),
            "Core {} should be in node {} cores list",
            core_id,
            node_id
        );
    }

    // Validate all cores in node_cores are mapped back to cores
    for (node_id, cores) in &topology.node_cores {
        for &core_id in cores {
            assert_eq!(
                topology.core_to_node.get(&core_id),
                Some(node_id),
                "Core {} should map to node {}",
                core_id,
                node_id
            );
        }
    }

    println!(
        "Detected topology: {} NUMA nodes, {} cores total",
        topology.num_nodes,
        topology.core_to_node.len()
    );
    for (node_id, cores) in &topology.node_cores {
        println!("  Node {}: {} cores {:?}", node_id, cores.len(), cores);
    }
}

#[test]
fn test_topology_siblings() {
    let topology = rustfiber::topology::Topology::detect();

    // Test sibling detection for each core
    for core_id in 0..topology.core_to_node.len() {
        let siblings = topology.get_siblings(core_id);
        assert!(siblings.is_some(), "Core {} should have siblings", core_id);

        let siblings = siblings.unwrap();
        assert!(
            !siblings.is_empty(),
            "Core {} should have at least itself as sibling",
            core_id
        );

        // All siblings should be in the same NUMA node
        let expected_node = topology.core_to_node[&core_id];
        for &sibling_id in siblings {
            assert_eq!(
                topology.core_to_node[&sibling_id], expected_node,
                "Sibling {} should be in same node {} as core {}",
                sibling_id, expected_node, core_id
            );
        }
    }
}

#[test]
fn test_topology_local_allocation_end_to_end() {
    let system = JobSystem::new(2);
    let was_frame_allocated = Arc::new(AtomicBool::new(false));
    let was_frame_allocated_clone = was_frame_allocated.clone();

    // Spawning from main thread -> Global Allocator (fallback path)
    system.run(move || {
        // We are now running on a worker thread.
        // TLS allocator should be active.

        // Create a nested job. This calls Job::new().
        // We can't easily inspect the internal variant (Work::SimpleFrame vs Work::Simple)
        // without reflection or hacks, but we can verify it executes correctly.
        // If the allocator logic was broken (e.g., segfault), this would crash.

        let _job = rustfiber::Job::new(move || {
            was_frame_allocated_clone.store(true, Ordering::SeqCst);
        });

        // We need to execute it.
        // We don't have easy access to the Scheduler here locally unless we use Context.
        // But Job::new checks allocation at creation time.

        // Let's pretend to execute by unwrapping manually? No, Work is private.
        // We can use a Context-based run to spawn properly.
    });

    // To properly test execution, we should use run_with_context
    let system2 = JobSystem::new(2);
    let done = Arc::new(AtomicBool::new(false));
    let done_clone = done.clone();

    let root = system2.run_with_context(move |ctx| {
        // ctx is available.
        // ctx.spawn_job uses Job::with_counter_and_context_in_allocator usually?
        // Let's explicitly use Job::new and submit it manually if possible,
        // or just rely on the fact that ctx.spawn(...) might use new internals.

        // Actually, Job::new calls are what we optimized.
        // We want to ensure normal Job usage works.

        // The optimization target was `Job::new` which is generic.
        // So we just run a nested logic.

        let cnt = ctx.spawn_job(move |_| {
            done_clone.store(true, Ordering::SeqCst);
        });
        ctx.wait_for(&cnt);
    });

    system2.wait_for_counter(&root);
    assert!(done.load(Ordering::SeqCst));
}
