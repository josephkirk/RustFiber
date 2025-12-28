use rustfiber::job_system::{FiberConfig, JobSystem};
use std::time::Duration;

#[test]
fn test_fiber_config_defaults() {
    let config = FiberConfig::default();
    assert_eq!(config.stack_size, 512 * 1024); // 512KB
    assert_eq!(config.initial_pool_size, 16);
    assert_eq!(config.target_pool_size, 128);
    assert_eq!(config.frame_stack_size, 1024 * 1024); // 1MB
    assert_eq!(config.prefetch_pages, false);
}

#[test]
fn test_fiber_config_custom() {
    let mut config = FiberConfig::default();
    config.stack_size = 256 * 1024;
    config.initial_pool_size = 8;
    config.target_pool_size = 64;
    config.frame_stack_size = 512 * 1024;
    config.prefetch_pages = true;

    assert_eq!(config.stack_size, 256 * 1024);
    assert_eq!(config.initial_pool_size, 8);
    assert_eq!(config.target_pool_size, 64);
    assert_eq!(config.frame_stack_size, 512 * 1024);
    assert_eq!(config.prefetch_pages, true);
}

#[test]
fn test_job_system_with_custom_config() {
    let mut config = FiberConfig::default();
    config.initial_pool_size = 4;
    config.target_pool_size = 16;

    let job_system = JobSystem::new_with_config(2, config);

    // Run some work to ensure the system functions
    let counter = job_system.run(|| {
        std::thread::sleep(Duration::from_millis(1));
    });
    job_system.wait_for_counter(&counter);

    // Clean shutdown
    job_system.shutdown().unwrap();
}

#[test]
fn test_incremental_pool_growth_config() {
    // Test that target_pool_size > initial_pool_size enables incremental growth
    let mut config = FiberConfig::default();
    config.initial_pool_size = 4;
    config.target_pool_size = 16;

    assert!(
        config.target_pool_size > config.initial_pool_size,
        "Target pool size should be larger than initial to enable incremental growth"
    );

    let job_system = JobSystem::new_with_config(2, config);

    // Run some work to trigger potential fiber usage
    let counters: Vec<_> = (0..20)
        .map(|_| {
            job_system.run(|| {
                std::thread::sleep(Duration::from_micros(100));
            })
        })
        .collect();

    for counter in counters {
        job_system.wait_for_counter(&counter);
    }

    // Give some time for incremental growth to occur
    std::thread::sleep(Duration::from_millis(50));

    // Clean shutdown
    job_system.shutdown().unwrap();
}

#[test]
fn test_prefetch_pages_config() {
    // Test that prefetch_pages config is properly stored and accessible
    let mut config = FiberConfig::default();
    config.prefetch_pages = true;

    assert_eq!(config.prefetch_pages, true);

    // Test default is false
    let default_config = FiberConfig::default();
    assert_eq!(default_config.prefetch_pages, false);
}
