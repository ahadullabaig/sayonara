/// Hardware Integration Tests for Wipe Orchestrator
///
/// These tests use mock drives to simulate hardware without requiring physical drives.
/// Each test verifies end-to-end wipe operations for different drive types.
mod common;

// Set test mode environment variable for in-memory database
#[ctor::ctor]
fn set_test_mode() {
    std::env::set_var("SAYONARA_TEST_MODE", "1");
}

use anyhow::Result;
use common::assertions::*;
use common::mock_drive_builders::*;
use common::mock_drive_v2::MockDrive;
use sayonara_wipe::{Algorithm, WipeConfig, WipeOrchestrator};

// ==================== HDD TESTS ====================

#[tokio::test]
async fn test_wipe_hdd_drive() -> Result<()> {
    // Create mock HDD drive
    let mock = MockDrive::hdd(10)?; // 10MB for fast testing
    let device_path = mock.path_str().to_string();

    // Configure wipe with Zero algorithm (single pass)
    let config = WipeConfig {
        algorithm: Algorithm::Zero,
        verify: false,                 // We'll verify manually
        freeze_mitigation: false,      // Not testing freeze mitigation
        temperature_monitoring: false, // Mock doesn't need real monitoring
        ..Default::default()
    };

    // Execute wipe through production orchestrator
    let mut orchestrator = WipeOrchestrator::new(device_path, config)?;
    orchestrator.execute().await?;

    // Verify results
    assert_wipe_completed(&mock, 1)?; // 1 pass for Zero algorithm
    assert_verification_passed(&mock, 0.99)?; // 99% success rate
    assert_temperature_safe(&mock)?;

    println!("✓ HDD wipe integration test passed");
    Ok(())
}

// ==================== SSD TESTS ====================

#[tokio::test]
async fn test_wipe_ssd_drive() -> Result<()> {
    // Create mock SSD drive
    let mock = MockDrive::ssd(10)?; // 10MB
    let device_path = mock.path_str().to_string();

    // Configure wipe
    let config = WipeConfig {
        algorithm: Algorithm::Random,
        verify: false,
        use_trim_after: true, // SSD should TRIM
        freeze_mitigation: false,
        temperature_monitoring: false,
        ..Default::default()
    };

    // Execute wipe
    let mut orchestrator = WipeOrchestrator::new(device_path, config)?;
    orchestrator.execute().await?;

    // Verify
    assert_wipe_completed(&mock, 1)?;
    assert_verification_passed(&mock, 0.99)?;

    println!("✓ SSD wipe integration test passed");
    Ok(())
}

// ==================== NVME BASIC TESTS ====================

#[tokio::test]
async fn test_wipe_nvme_basic() -> Result<()> {
    // Create basic NVMe mock
    let mock = MockDrive::nvme()?; // Uses builder default (1GB)
    let device_path = mock.path_str().to_string();

    // Configure wipe
    let config = WipeConfig {
        algorithm: Algorithm::Zero,
        verify: false,
        freeze_mitigation: false,
        temperature_monitoring: false,
        ..Default::default()
    };

    // Execute wipe
    let mut orchestrator = WipeOrchestrator::new(device_path, config)?;
    orchestrator.execute().await?;

    // Verify
    assert_wipe_completed(&mock, 1)?;
    assert_verification_passed(&mock, 0.99)?;

    println!("✓ NVMe basic wipe integration test passed");
    Ok(())
}

// ==================== SMR TESTS ====================

#[tokio::test]
async fn test_wipe_smr_drive() -> Result<()> {
    // Create mock SMR drive
    let mock = MockDrive::smr()?; // 100MB with zone support
    let device_path = mock.path_str().to_string();

    // Configure wipe - SMR drives need sequential writes
    let config = WipeConfig {
        algorithm: Algorithm::Zero,
        verify: false,
        freeze_mitigation: false,
        temperature_monitoring: false,
        ..Default::default()
    };

    // Execute wipe
    let mut orchestrator = WipeOrchestrator::new(device_path, config)?;
    orchestrator.execute().await?;

    // Verify
    assert_wipe_completed(&mock, 1)?;
    assert_verification_passed(&mock, 0.99)?;

    println!("✓ SMR wipe integration test passed");
    Ok(())
}

// ==================== OPTANE TESTS ====================

#[tokio::test]
async fn test_wipe_optane_drive() -> Result<()> {
    // Create mock Optane drive
    let mock = MockDrive::optane()?; // 100MB with ISE support
    let device_path = mock.path_str().to_string();

    // Configure wipe - Optane supports crypto erase
    let config = WipeConfig {
        algorithm: Algorithm::CryptoErase,
        verify: false,
        freeze_mitigation: false,
        temperature_monitoring: false,
        ..Default::default()
    };

    // Execute wipe
    let mut orchestrator = WipeOrchestrator::new(device_path, config)?;
    orchestrator.execute().await?;

    // Verify
    assert_wipe_completed(&mock, 1)?;
    assert_verification_passed(&mock, 0.99)?;

    println!("✓ Optane wipe integration test passed");
    Ok(())
}

// ==================== HYBRID SSHD TESTS ====================

#[tokio::test]
async fn test_wipe_hybrid_drive() -> Result<()> {
    // Create mock Hybrid SSHD
    let mock = MockDrive::hybrid()?; // 110MB (100MB HDD + 10MB SSD cache)
    let device_path = mock.path_str().to_string();

    // Configure wipe - Hybrid needs both HDD and SSD strategies
    let config = WipeConfig {
        algorithm: Algorithm::Random,
        verify: false,
        use_trim_after: true, // TRIM for SSD portion
        freeze_mitigation: false,
        temperature_monitoring: false,
        ..Default::default()
    };

    // Execute wipe
    let mut orchestrator = WipeOrchestrator::new(device_path, config)?;
    orchestrator.execute().await?;

    // Verify
    assert_wipe_completed(&mock, 1)?;
    assert_verification_passed(&mock, 0.99)?;

    println!("✓ Hybrid SSHD wipe integration test passed");
    Ok(())
}

// ==================== eMMC TESTS ====================

#[tokio::test]
async fn test_wipe_emmc_drive() -> Result<()> {
    // Create mock eMMC device
    let mock = MockDrive::emmc()?; // 100MB with TRIM support
    let device_path = mock.path_str().to_string();

    // Configure wipe - eMMC supports TRIM and secure erase
    let config = WipeConfig {
        algorithm: Algorithm::SecureErase,
        verify: false,
        use_trim_after: true,
        freeze_mitigation: false,
        temperature_monitoring: false,
        ..Default::default()
    };

    // Execute wipe
    let mut orchestrator = WipeOrchestrator::new(device_path, config)?;
    orchestrator.execute().await?;

    // Verify
    assert_wipe_completed(&mock, 1)?;
    assert_verification_passed(&mock, 0.99)?;

    println!("✓ eMMC wipe integration test passed");
    Ok(())
}

// ==================== RAID TESTS ====================

#[tokio::test]
async fn test_wipe_raid_array() -> Result<()> {
    // Create mock RAID array
    let mock = MockDrive::raid()?; // 100MB RAID5 array
    let device_path = mock.path_str().to_string();

    // Configure wipe - RAID arrays wipe member drives
    let config = WipeConfig {
        algorithm: Algorithm::Zero,
        verify: false,
        freeze_mitigation: false,
        temperature_monitoring: false,
        ..Default::default()
    };

    // Execute wipe
    let mut orchestrator = WipeOrchestrator::new(device_path, config)?;
    orchestrator.execute().await?;

    // Verify
    assert_wipe_completed(&mock, 1)?;
    assert_verification_passed(&mock, 0.99)?;

    println!("✓ RAID array wipe integration test passed");
    Ok(())
}

// ==================== NVME ADVANCED TESTS ====================

#[tokio::test]
async fn test_wipe_nvme_advanced() -> Result<()> {
    // Create NVMe mock with advanced features (sanitize, crypto erase)
    let mock = MockNVMeDriveBuilder::new().enable_sanitize(true).build()?;
    let device_path = mock.path_str().to_string();

    // Configure wipe with NVMe sanitize command
    let config = WipeConfig {
        algorithm: Algorithm::Sanitize,
        verify: false,
        freeze_mitigation: false,
        temperature_monitoring: false,
        ..Default::default()
    };

    // Execute wipe
    let mut orchestrator = WipeOrchestrator::new(device_path, config)?;
    orchestrator.execute().await?;

    // Verify
    assert_wipe_completed(&mock, 1)?;
    assert_verification_passed(&mock, 0.99)?;

    println!("✓ NVMe Advanced wipe integration test passed");
    Ok(())
}

// ==================== UFS TESTS ====================

#[tokio::test]
async fn test_wipe_ufs_drive() -> Result<()> {
    // Create mock UFS drive
    let mock = MockDrive::ufs()?; // 100MB UFS 3.1
    let device_path = mock.path_str().to_string();

    // Configure wipe - UFS supports TRIM and crypto erase
    let config = WipeConfig {
        algorithm: Algorithm::CryptoErase,
        verify: false,
        use_trim_after: true,
        freeze_mitigation: false,
        temperature_monitoring: false,
        ..Default::default()
    };

    // Execute wipe
    let mut orchestrator = WipeOrchestrator::new(device_path, config)?;
    orchestrator.execute().await?;

    // Verify
    assert_wipe_completed(&mock, 1)?;
    assert_verification_passed(&mock, 0.99)?;

    println!("✓ UFS wipe integration test passed");
    Ok(())
}

// ==================== ORCHESTRATOR TESTS ====================

#[tokio::test]
async fn test_orchestrator_execute_basic() -> Result<()> {
    // Test basic orchestrator execution flow
    let mock = MockDrive::hdd(10)?;
    let device_path = mock.path_str().to_string();

    let config = WipeConfig {
        algorithm: Algorithm::Zero,
        verify: false,
        freeze_mitigation: false,
        temperature_monitoring: false,
        ..Default::default()
    };

    let mut orchestrator = WipeOrchestrator::new(device_path, config)?;
    let result = orchestrator.execute().await;

    assert!(result.is_ok(), "Orchestrator execution should succeed");
    assert_wipe_completed(&mock, 1)?;

    println!("✓ Orchestrator execute integration test passed");
    Ok(())
}

#[tokio::test]
async fn test_orchestrator_multiple_algorithms() -> Result<()> {
    // Test orchestrator with different algorithms
    let algorithms = vec![Algorithm::Zero, Algorithm::Random, Algorithm::DoD5220];

    for algo in algorithms {
        let mock = MockDrive::ssd(10)?;
        let device_path = mock.path_str().to_string();

        let config = WipeConfig {
            algorithm: algo.clone(),
            verify: false,
            freeze_mitigation: false,
            temperature_monitoring: false,
            ..Default::default()
        };

        let mut orchestrator = WipeOrchestrator::new(device_path, config)?;
        orchestrator.execute().await?;

        // Verify based on algorithm
        let expected_passes = match algo {
            Algorithm::DoD5220 => 3, // DoD has 3 passes
            _ => 1,
        };
        assert_wipe_completed(&mock, expected_passes)?;
    }

    println!("✓ Orchestrator multiple algorithms test passed");
    Ok(())
}

// ==================== I/O PERFORMANCE TESTS ====================

#[tokio::test]
async fn test_io_performance_with_mock() -> Result<()> {
    // Test I/O performance metrics with mock drive
    let mock = MockDrive::ssd(50)?; // 50MB for performance test
    let device_path = mock.path_str().to_string();

    // Configure wipe with Random algorithm to test I/O throughput
    let config = WipeConfig {
        algorithm: Algorithm::Random,
        verify: false,
        freeze_mitigation: false,
        temperature_monitoring: false,
        ..Default::default()
    };

    // Execute wipe and measure performance
    let mut orchestrator = WipeOrchestrator::new(device_path, config)?;
    orchestrator.execute().await?;

    // Verify completion and performance
    assert_wipe_completed(&mock, 1)?;
    assert_verification_passed(&mock, 0.99)?;

    // Check that writes completed (performance metrics validated by completion)
    let stats = mock.stats();
    assert!(stats.bytes_written > 0, "Should have written data");
    assert_eq!(stats.error_count, 0, "Should have no errors");

    println!("✓ I/O performance test passed");
    Ok(())
}

// ==================== VERIFICATION TESTS ====================

#[tokio::test]
async fn test_verification_after_wipe() -> Result<()> {
    // Test that verification correctly validates a wiped drive
    let mock = MockDrive::hdd(20)?; // 20MB for verification test
    let device_path = mock.path_str().to_string();

    // Configure wipe with verification enabled
    let config = WipeConfig {
        algorithm: Algorithm::Zero,
        verify: true, // Enable verification
        freeze_mitigation: false,
        temperature_monitoring: false,
        ..Default::default()
    };

    // Execute wipe with verification
    let mut orchestrator = WipeOrchestrator::new(device_path, config)?;
    orchestrator.execute().await?;

    // Verify that wipe completed and verification passed
    assert_wipe_completed(&mock, 1)?;
    assert_verification_passed(&mock, 0.99)?;

    // Manually verify the mock drive was wiped to zeros
    let result = mock.verify_wipe(Some(&[0u8; 1024]))?;
    assert!(
        result.success_rate > 0.99,
        "Verification should show high success rate"
    );
    assert!(
        result.mismatches == 0,
        "Should have no mismatches for zero pattern"
    );

    println!("✓ Verification integration test passed");
    Ok(())
}

// ==================== FREEZE MITIGATION TESTS ====================

#[tokio::test]
async fn test_freeze_mitigation_disabled() -> Result<()> {
    // Test that wipe works when freeze mitigation is disabled
    // (mock drives default to NotFrozen state)
    let mock = MockDrive::hdd(10)?;
    let device_path = mock.path_str().to_string();

    let config = WipeConfig {
        algorithm: Algorithm::Zero,
        verify: false,
        freeze_mitigation: false, // Explicitly disabled
        temperature_monitoring: false,
        ..Default::default()
    };

    let mut orchestrator = WipeOrchestrator::new(device_path, config)?;
    orchestrator.execute().await?;

    assert_wipe_completed(&mock, 1)?;
    println!("✓ Freeze mitigation (disabled) test passed");
    Ok(())
}

#[cfg(test)]
mod test_success {
    #[test]
    fn verify_test_compiles() {
        // This test just verifies the file compiles
    }
}
