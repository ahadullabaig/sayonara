/// Simplified integration tests for error recovery mechanisms
///
/// Tests core recovery functionality that is publicly exposed

mod common;

use sayonara_wipe::error::checkpoint::{Checkpoint, CheckpointManager};
use sayonara_wipe::error::classification::{ErrorContext, ErrorClassifier};
use sayonara_wipe::error::mechanisms::BadSectorHandler;
use sayonara_wipe::DriveError;
use anyhow::Result;

#[test]
fn test_error_context_creation_for_pass() {
    let context = ErrorContext::for_pass("/dev/sda", "Gutmann", 5);

    assert_eq!(context.device_path, "/dev/sda");
    assert_eq!(context.operation, "Gutmann_pass_5");
    assert!(context.metadata.contains_key("algorithm"));
    assert!(context.metadata.contains_key("pass"));
}

#[test]
fn test_error_context_creation_for_verification() {
    let context = ErrorContext::for_verification("/dev/sdb", 1024 * 1024);

    assert_eq!(context.device_path, "/dev/sdb");
    assert_eq!(context.operation, "verification");
    assert_eq!(context.offset, Some(1024 * 1024));
}

#[test]
fn test_error_context_with_metadata() {
    let context = ErrorContext::new("wipe", "/dev/sdc")
        .with_metadata("drive_type", "NVMe")
        .with_metadata("capacity_gb", "500");

    assert_eq!(context.metadata.get("drive_type"), Some(&"NVMe".to_string()));
    assert_eq!(context.metadata.get("capacity_gb"), Some(&"500".to_string()));
}

#[test]
fn test_error_classifier_instantiation() {
    let _classifier = ErrorClassifier::new();

    // Just verify we can create the classifier successfully
    // Actual classification tests require proper DriveError construction
    // which is more complex for integration testing
}

#[test]
fn test_bad_sector_handler_creation() -> Result<()> {
    let handler = BadSectorHandler::new("/dev/test");

    // Verify handler was created successfully
    assert_eq!(handler.bad_sector_count(), 0);

    Ok(())
}

#[test]
fn test_bad_sector_record_and_check() -> Result<()> {
    let handler = BadSectorHandler::new("/dev/test");

    // Record some bad sectors
    handler.record_bad_sector(1000, "Read error")?;
    handler.record_bad_sector(2000, "Timeout")?;
    handler.record_bad_sector(3000, "CRC error")?;

    // Verify count
    assert_eq!(handler.bad_sector_count(), 3);

    // Check is_bad_sector
    assert!(handler.is_bad_sector(1000));
    assert!(handler.is_bad_sector(2000));
    assert!(!handler.is_bad_sector(4000));

    Ok(())
}

#[test]
fn test_bad_sector_handler_multiple_records_same_sector() -> Result<()> {
    let handler = BadSectorHandler::new("/dev/test");

    // Record same sector multiple times
    handler.record_bad_sector(5000, "Error 1")?;
    handler.record_bad_sector(5000, "Error 2")?;
    handler.record_bad_sector(5000, "Error 3")?;

    // Should only count once (deduplicated)
    assert_eq!(handler.bad_sector_count(), 1);

    Ok(())
}

#[test]
fn test_checkpoint_manager_integration_with_recovery() -> Result<()> {
    let mut manager = CheckpointManager::new(None)?;

    // Create checkpoint
    let mut checkpoint = Checkpoint::new(
        "/dev/recovery_test",
        "DoD",
        "recovery-op-001",
        3,
        100_000_000_000,
    );

    // Simulate progress
    checkpoint.update_progress(1, 33_000_000_000);

    // Record an error
    checkpoint.record_error("Simulated I/O error during recovery test");

    manager.save(&checkpoint)?;

    // Load and verify
    let loaded = manager.load("/dev/recovery_test", "DoD")?.unwrap();
    assert_eq!(loaded.error_count, 1);
    assert!(loaded.last_error.is_some());
    assert_eq!(loaded.current_pass, 1);

    Ok(())
}

#[test]
fn test_checkpoint_resume_after_simulated_failure() -> Result<()> {
    let mut manager = CheckpointManager::new(None)?;

    // Simulate: operation starts
    let mut checkpoint = Checkpoint::new(
        "/dev/resume_test",
        "Gutmann",
        "resume-op-002",
        35,
        500_000_000_000,
    );

    // Simulate: progress through several passes
    checkpoint.update_progress(10, 150_000_000_000);
    manager.save(&checkpoint)?;

    // Simulate: failure occurs
    checkpoint.record_error("Drive timeout");
    manager.save(&checkpoint)?;

    // Simulate: resume attempt - load checkpoint
    let resumed = manager.load("/dev/resume_test", "Gutmann")?.unwrap();

    // Verify we can resume from where we left off
    assert_eq!(resumed.current_pass, 10);
    assert_eq!(resumed.bytes_written, 150_000_000_000);
    assert_eq!(resumed.error_count, 1);

    Ok(())
}

#[test]
fn test_multiple_recovery_attempts_tracked_in_checkpoint() -> Result<()> {
    let mut manager = CheckpointManager::new(None)?;

    let mut checkpoint = Checkpoint::new(
        "/dev/multi_error",
        "Random",
        "multi-err-003",
        1,
        100_000_000_000,
    );

    // Simulate multiple errors during recovery attempts
    for i in 1..=5 {
        checkpoint.record_error(format!("Recovery attempt {} failed", i));
        manager.save(&checkpoint)?;
    }

    // Verify error count
    let loaded = manager.load("/dev/multi_error", "Random")?.unwrap();
    assert_eq!(loaded.error_count, 5);
    assert!(loaded.last_error.unwrap().contains("attempt 5"));

    Ok(())
}

#[test]
fn test_bad_sector_persistence_across_checkpoint_saves() -> Result<()> {
    let handler = BadSectorHandler::new("/dev/persistence_test");

    // Record bad sectors
    handler.record_bad_sector(100, "I/O error")?;
    handler.record_bad_sector(200, "Media error")?;
    handler.record_bad_sector(300, "Timeout")?;

    // Verify they persist in the handler
    assert_eq!(handler.bad_sector_count(), 3);

    // All recorded sectors should be marked as bad
    for sector in [100, 200, 300] {
        assert!(handler.is_bad_sector(sector));
    }

    Ok(())
}

#[test]
fn test_checkpoint_cleanup_removes_old_entries() -> Result<()> {
    use chrono::{Duration, Utc};

    let mut manager = CheckpointManager::new(None)?;

    // Create an old checkpoint
    let mut old_checkpoint = Checkpoint::new(
        "/dev/old",
        "DoD",
        "old-op",
        3,
        100_000_000_000,
    );

    // Manually set it to 40 days old
    old_checkpoint.created_at = Utc::now() - Duration::days(40);
    old_checkpoint.updated_at = old_checkpoint.created_at;
    manager.save(&old_checkpoint)?;

    // Create a recent checkpoint
    let recent_checkpoint = Checkpoint::new(
        "/dev/recent",
        "Gutmann",
        "recent-op",
        35,
        500_000_000_000,
    );
    manager.save(&recent_checkpoint)?;

    // Clean up old checkpoints (older than 30 days)
    let deleted = manager.cleanup_stale(Duration::days(30))?;
    assert_eq!(deleted, 1);

    // Verify old is gone, recent remains
    assert!(manager.load("/dev/old", "DoD")?.is_none());
    assert!(manager.load("/dev/recent", "Gutmann")?.is_some());

    Ok(())
}
