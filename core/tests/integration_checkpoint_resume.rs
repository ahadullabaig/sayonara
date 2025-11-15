/// Integration tests for checkpoint/resume functionality
///
/// Tests the complete checkpoint lifecycle including:
/// - Creation and saving
/// - Loading and resuming
/// - Error handling and recovery
/// - Stale checkpoint cleanup
mod common;

use anyhow::Result;
use chrono::{Duration, Utc};
use sayonara_wipe::error::checkpoint::{Checkpoint, CheckpointManager};

#[test]
fn test_checkpoint_creation_and_save() -> Result<()> {
    let mut manager = CheckpointManager::new(None)?;

    let checkpoint = Checkpoint::new("/dev/sda", "Gutmann", "test-op-001", 35, 500_000_000_000);

    assert_eq!(checkpoint.device_path, "/dev/sda");
    assert_eq!(checkpoint.algorithm, "Gutmann");
    assert_eq!(checkpoint.total_passes, 35);
    assert_eq!(checkpoint.current_pass, 0);
    assert_eq!(checkpoint.bytes_written, 0);

    manager.save(&checkpoint)?;

    Ok(())
}

#[test]
fn test_checkpoint_load_and_resume() -> Result<()> {
    let mut manager = CheckpointManager::new(None)?;

    // Create and save initial checkpoint
    let mut checkpoint = Checkpoint::new("/dev/sda", "Gutmann", "test-op-002", 35, 500_000_000_000);

    checkpoint.update_progress(10, 150_000_000_000);
    manager.save(&checkpoint)?;

    // Load checkpoint and verify
    let loaded = manager.load("/dev/sda", "Gutmann")?;
    assert!(loaded.is_some());

    let loaded = loaded.unwrap();
    assert_eq!(loaded.device_path, "/dev/sda");
    assert_eq!(loaded.algorithm, "Gutmann");
    assert_eq!(loaded.current_pass, 10);
    assert_eq!(loaded.bytes_written, 150_000_000_000);

    Ok(())
}

#[test]
fn test_checkpoint_update_progress() -> Result<()> {
    let mut manager = CheckpointManager::new(None)?;

    let mut checkpoint = Checkpoint::new("/dev/sdb", "DoD", "test-op-003", 3, 250_000_000_000);

    // Initial state
    assert_eq!(checkpoint.current_pass, 0);
    assert_eq!(checkpoint.bytes_written, 0);

    // Update progress
    checkpoint.update_progress(1, 100_000_000_000);
    assert_eq!(checkpoint.current_pass, 1);
    assert_eq!(checkpoint.bytes_written, 100_000_000_000);

    manager.save(&checkpoint)?;

    // Load and verify update
    let loaded = manager.load("/dev/sdb", "DoD")?.unwrap();
    assert_eq!(loaded.current_pass, 1);
    assert_eq!(loaded.bytes_written, 100_000_000_000);

    Ok(())
}

#[test]
fn test_checkpoint_error_recording() -> Result<()> {
    let mut manager = CheckpointManager::new(None)?;

    let mut checkpoint = Checkpoint::new("/dev/sdc", "Random", "test-op-004", 1, 100_000_000_000);

    // Record first error
    checkpoint.record_error("I/O error at sector 12345");
    assert_eq!(checkpoint.error_count, 1);
    assert_eq!(
        checkpoint.last_error,
        Some("I/O error at sector 12345".to_string())
    );

    manager.save(&checkpoint)?;

    // Record second error
    checkpoint.record_error("Timeout waiting for device");
    assert_eq!(checkpoint.error_count, 2);
    assert_eq!(
        checkpoint.last_error,
        Some("Timeout waiting for device".to_string())
    );

    manager.save(&checkpoint)?;

    // Verify persistence
    let loaded = manager.load("/dev/sdc", "Random")?.unwrap();
    assert_eq!(loaded.error_count, 2);
    assert_eq!(
        loaded.last_error,
        Some("Timeout waiting for device".to_string())
    );

    Ok(())
}

#[test]
fn test_checkpoint_completion_percentage() -> Result<()> {
    let mut checkpoint = Checkpoint::new("/dev/sdd", "Zero", "test-op-005", 1, 1_000_000_000);

    // 0% complete
    assert_eq!(checkpoint.completion_percentage(), 0.0);

    // 25% complete
    checkpoint.update_progress(0, 250_000_000);
    assert!((checkpoint.completion_percentage() - 25.0).abs() < 0.01);

    // 50% complete
    checkpoint.update_progress(0, 500_000_000);
    assert!((checkpoint.completion_percentage() - 50.0).abs() < 0.01);

    // 100% complete
    checkpoint.update_progress(0, 1_000_000_000);
    assert!((checkpoint.completion_percentage() - 100.0).abs() < 0.01);

    Ok(())
}

#[test]
fn test_checkpoint_delete() -> Result<()> {
    let mut manager = CheckpointManager::new(None)?;

    let checkpoint = Checkpoint::new("/dev/sde", "Gutmann", "test-op-006", 35, 500_000_000_000);

    manager.save(&checkpoint)?;

    // Verify it exists
    let loaded = manager.load("/dev/sde", "Gutmann")?;
    assert!(loaded.is_some());

    // Delete by ID
    manager.delete(&checkpoint.id)?;

    // Verify deletion
    let loaded = manager.load("/dev/sde", "Gutmann")?;
    assert!(loaded.is_none());

    Ok(())
}

#[test]
fn test_checkpoint_delete_by_device() -> Result<()> {
    let mut manager = CheckpointManager::new(None)?;

    let checkpoint = Checkpoint::new("/dev/sdf", "DoD", "test-op-007", 3, 250_000_000_000);

    manager.save(&checkpoint)?;

    // Delete by device and algorithm
    let count = manager.delete_by_device("/dev/sdf", "DoD")?;
    assert_eq!(count, 1);

    // Verify deletion
    let loaded = manager.load("/dev/sdf", "DoD")?;
    assert!(loaded.is_none());

    Ok(())
}

#[test]
fn test_checkpoint_list_all() -> Result<()> {
    let mut manager = CheckpointManager::new(None)?;

    // Create multiple checkpoints
    let devices = vec![
        ("/dev/sdg", "Gutmann", "op-008"),
        ("/dev/sdh", "DoD", "op-009"),
        ("/dev/sdi", "Random", "op-010"),
    ];

    for (device, algo, op_id) in &devices {
        let checkpoint = Checkpoint::new(*device, *algo, *op_id, 10, 100_000_000_000);
        manager.save(&checkpoint)?;
    }

    // List all checkpoints
    let checkpoints = manager.list_all()?;
    assert_eq!(checkpoints.len(), 3);

    // Verify all devices are present
    let device_paths: Vec<String> = checkpoints.iter().map(|c| c.device_path.clone()).collect();
    assert!(device_paths.contains(&"/dev/sdg".to_string()));
    assert!(device_paths.contains(&"/dev/sdh".to_string()));
    assert!(device_paths.contains(&"/dev/sdi".to_string()));

    Ok(())
}

#[test]
fn test_checkpoint_stale_cleanup() -> Result<()> {
    let mut manager = CheckpointManager::new(None)?;

    // Create an old checkpoint (simulate by setting old timestamp)
    let mut old_checkpoint =
        Checkpoint::new("/dev/sdj", "Gutmann", "old-op-011", 35, 500_000_000_000);

    // Set created_at to 40 days ago
    old_checkpoint.created_at = Utc::now() - Duration::days(40);
    old_checkpoint.updated_at = old_checkpoint.created_at;
    manager.save(&old_checkpoint)?;

    // Create a recent checkpoint
    let recent_checkpoint = Checkpoint::new("/dev/sdk", "DoD", "recent-op-012", 3, 250_000_000_000);
    manager.save(&recent_checkpoint)?;

    // Clean up checkpoints older than 30 days
    let deleted_count = manager.cleanup_stale(Duration::days(30))?;
    assert_eq!(deleted_count, 1);

    // Verify old checkpoint is gone
    let loaded_old = manager.load("/dev/sdj", "Gutmann")?;
    assert!(loaded_old.is_none());

    // Verify recent checkpoint remains
    let loaded_recent = manager.load("/dev/sdk", "DoD")?;
    assert!(loaded_recent.is_some());

    Ok(())
}

#[test]
fn test_checkpoint_load_nonexistent() -> Result<()> {
    let manager = CheckpointManager::new(None)?;

    // Try to load checkpoint that doesn't exist
    let loaded = manager.load("/dev/nonexistent", "Gutmann")?;
    assert!(loaded.is_none());

    Ok(())
}

#[test]
fn test_checkpoint_multiple_algorithms_same_device() -> Result<()> {
    let mut manager = CheckpointManager::new(None)?;

    let device = "/dev/sdl";

    // Create checkpoints for different algorithms on same device
    let gutmann_cp = Checkpoint::new(device, "Gutmann", "op-013", 35, 500_000_000_000);
    let dod_cp = Checkpoint::new(device, "DoD", "op-014", 3, 500_000_000_000);

    manager.save(&gutmann_cp)?;
    manager.save(&dod_cp)?;

    // Load each one independently
    let loaded_gutmann = manager.load(device, "Gutmann")?.unwrap();
    let loaded_dod = manager.load(device, "DoD")?.unwrap();

    assert_eq!(loaded_gutmann.algorithm, "Gutmann");
    assert_eq!(loaded_gutmann.total_passes, 35);

    assert_eq!(loaded_dod.algorithm, "DoD");
    assert_eq!(loaded_dod.total_passes, 3);

    Ok(())
}

#[test]
fn test_checkpoint_should_save_logic() -> Result<()> {
    let mut manager = CheckpointManager::new(None)?;

    // Set intervals: 100 milliseconds time interval, 1MB bytes interval
    manager.set_intervals(std::time::Duration::from_millis(100), 1024 * 1024);

    // Initially should NOT save (no time has passed, no bytes written)
    assert!(!manager.should_save(0));

    // After short time and few bytes, should not save
    std::thread::sleep(std::time::Duration::from_millis(10));
    assert!(!manager.should_save(100_000)); // 100KB

    // After enough time, should save
    std::thread::sleep(std::time::Duration::from_millis(100));
    assert!(manager.should_save(100_000)); // Time threshold met

    // After enough bytes, should save
    assert!(manager.should_save(2 * 1024 * 1024)); // 2MB - bytes threshold met

    Ok(())
}

#[test]
fn test_checkpoint_progress_description() -> Result<()> {
    let mut checkpoint = Checkpoint::new("/dev/sdm", "Gutmann", "op-015", 35, 1_000_000_000);

    checkpoint.update_progress(10, 300_000_000);

    let description = checkpoint.progress_description();

    // Should contain pass info
    assert!(description.contains("Pass 11/35"));

    // Should contain percentage
    assert!(description.contains("30.00%"));

    // Should contain byte counts
    assert!(description.contains("300000000"));
    assert!(description.contains("1000000000"));

    Ok(())
}

#[test]
fn test_checkpoint_concurrent_access() -> Result<()> {
    // Create a manager with file-based database for concurrent access test
    let temp_dir = tempfile::tempdir()?;
    let db_path = temp_dir.path().join("concurrent_test.db");

    let mut manager1 = CheckpointManager::new(Some(db_path.to_str().unwrap()))?;

    // Save checkpoint from first manager
    let checkpoint = Checkpoint::new("/dev/sdn", "DoD", "concurrent-op-016", 3, 100_000_000_000);
    manager1.save(&checkpoint)?;

    // Load from second manager (different connection)
    let manager2 = CheckpointManager::new(Some(db_path.to_str().unwrap()))?;
    let loaded = manager2.load("/dev/sdn", "DoD")?;

    assert!(loaded.is_some());
    assert_eq!(loaded.unwrap().operation_id, "concurrent-op-016");

    Ok(())
}

#[test]
fn test_checkpoint_stats() -> Result<()> {
    let mut manager = CheckpointManager::new(None)?;

    // Create some checkpoints
    for i in 0..5 {
        let device = format!("/dev/sd{}", (b'o' + i) as char);
        let checkpoint = Checkpoint::new(&device, "Gutmann", "stats-test", 35, 500_000_000_000);
        manager.save(&checkpoint)?;
    }

    // Get stats
    let stats = manager.stats()?;
    assert_eq!(stats.total_checkpoints, 5);
    // Database size should be non-zero since we have checkpoints
    assert!(stats.database_size_bytes > 0);

    Ok(())
}
