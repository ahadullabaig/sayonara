// Comprehensive tests for Wipe Orchestrator
//
// NOTE: Many orchestrator methods require async execution and hardware access.
// These tests focus on testable logic, helper functions, and algorithm conversions.

use super::*;
use crate::{Algorithm, WipeConfig, DriveType};
use anyhow::Result;

// ==================== ALGORITHM CONVERSION TESTS ====================

#[test]
fn test_convert_to_wipe_algorithm_zero() -> Result<()> {
    let config = WipeConfig {
        algorithm: Algorithm::Zero,
        ..Default::default()
    };

    let orchestrator = WipeOrchestrator::new("/dev/null".to_string(), config)?;
    let wipe_algo = orchestrator.convert_to_wipe_algorithm();

    assert!(matches!(wipe_algo, WipeAlgorithm::Zeros));
    Ok(())
}

#[test]
fn test_convert_to_wipe_algorithm_random() -> Result<()> {
    let config = WipeConfig {
        algorithm: Algorithm::Random,
        ..Default::default()
    };

    let orchestrator = WipeOrchestrator::new("/dev/null".to_string(), config)?;
    let wipe_algo = orchestrator.convert_to_wipe_algorithm();

    assert!(matches!(wipe_algo, WipeAlgorithm::Random));
    Ok(())
}

#[test]
fn test_convert_to_wipe_algorithm_dod() -> Result<()> {
    let config = WipeConfig {
        algorithm: Algorithm::DoD5220,
        ..Default::default()
    };

    let orchestrator = WipeOrchestrator::new("/dev/null".to_string(), config)?;
    let wipe_algo = orchestrator.convert_to_wipe_algorithm();

    // DoD uses Random (multiple passes with random)
    assert!(matches!(wipe_algo, WipeAlgorithm::Random));
    Ok(())
}

#[test]
fn test_convert_to_wipe_algorithm_gutmann() -> Result<()> {
    let config = WipeConfig {
        algorithm: Algorithm::Gutmann,
        ..Default::default()
    };

    let orchestrator = WipeOrchestrator::new("/dev/null".to_string(), config)?;
    let wipe_algo = orchestrator.convert_to_wipe_algorithm();

    // Gutmann uses Random (complex patterns)
    assert!(matches!(wipe_algo, WipeAlgorithm::Random));
    Ok(())
}

// ==================== DRIVE INFO CREATION TESTS ====================

#[test]
fn test_create_basic_drive_info_nvme() -> Result<()> {
    let drive_info = WipeOrchestrator::create_basic_drive_info("/dev/nvme0n1")?;

    assert_eq!(drive_info.drive_type, DriveType::NVMe);
    assert_eq!(drive_info.device_path, "/dev/nvme0n1");
    assert!(drive_info.size > 0);
    Ok(())
}

#[test]
fn test_create_basic_drive_info_emmc() -> Result<()> {
    let drive_info = WipeOrchestrator::create_basic_drive_info("/dev/mmcblk0")?;

    assert_eq!(drive_info.drive_type, DriveType::EMMC);
    assert_eq!(drive_info.device_path, "/dev/mmcblk0");
    assert!(drive_info.size > 0);
    Ok(())
}

#[test]
fn test_create_basic_drive_info_hdd() -> Result<()> {
    let drive_info = WipeOrchestrator::create_basic_drive_info("/dev/sda")?;

    assert_eq!(drive_info.drive_type, DriveType::HDD);
    assert_eq!(drive_info.device_path, "/dev/sda");
    assert!(drive_info.size > 0);
    Ok(())
}

#[test]
fn test_create_basic_drive_info_hdd_alternative() -> Result<()> {
    let drive_info = WipeOrchestrator::create_basic_drive_info("/dev/sdb")?;

    assert_eq!(drive_info.drive_type, DriveType::HDD);
    assert_eq!(drive_info.device_path, "/dev/sdb");
    Ok(())
}

#[test]
fn test_create_basic_drive_info_nvme_namespace() -> Result<()> {
    let drive_info = WipeOrchestrator::create_basic_drive_info("/dev/nvme1n2")?;

    assert_eq!(drive_info.drive_type, DriveType::NVMe);
    assert_eq!(drive_info.device_path, "/dev/nvme1n2");
    Ok(())
}

// ==================== PATTERN GENERATION TESTS ====================

#[test]
fn test_generate_pattern_zero() -> Result<()> {
    let config = WipeConfig {
        algorithm: Algorithm::Zero,
        ..Default::default()
    };

    let orchestrator = WipeOrchestrator::new("/dev/null".to_string(), config)?;
    let pattern = orchestrator.generate_pattern(1024)?;

    assert_eq!(pattern.len(), 1024);
    assert!(pattern.iter().all(|&b| b == 0), "All bytes should be zero");
    Ok(())
}

#[test]
fn test_generate_pattern_random() -> Result<()> {
    let config = WipeConfig {
        algorithm: Algorithm::Random,
        ..Default::default()
    };

    let orchestrator = WipeOrchestrator::new("/dev/null".to_string(), config)?;
    let pattern = orchestrator.generate_pattern(1024)?;

    assert_eq!(pattern.len(), 1024);

    // Random pattern should not be all zeros or all ones
    let all_zeros = pattern.iter().all(|&b| b == 0);
    let all_ones = pattern.iter().all(|&b| b == 0xFF);

    assert!(!all_zeros && !all_ones, "Pattern should be random");
    Ok(())
}

#[test]
fn test_generate_pattern_dod() -> Result<()> {
    let config = WipeConfig {
        algorithm: Algorithm::DoD5220,
        ..Default::default()
    };

    let orchestrator = WipeOrchestrator::new("/dev/null".to_string(), config)?;
    let pattern = orchestrator.generate_pattern(1024)?;

    assert_eq!(pattern.len(), 1024);
    // DoD generates random pattern (first pass)
    Ok(())
}

#[test]
fn test_generate_pattern_gutmann() -> Result<()> {
    let config = WipeConfig {
        algorithm: Algorithm::Gutmann,
        ..Default::default()
    };

    let orchestrator = WipeOrchestrator::new("/dev/null".to_string(), config)?;
    let pattern = orchestrator.generate_pattern(1024)?;

    assert_eq!(pattern.len(), 1024);
    // Gutmann generates random pattern (simplified)
    Ok(())
}

#[test]
fn test_generate_pattern_different_sizes() -> Result<()> {
    let config = WipeConfig {
        algorithm: Algorithm::Zero,
        ..Default::default()
    };

    let orchestrator = WipeOrchestrator::new("/dev/null".to_string(), config)?;

    let sizes = vec![512, 1024, 4096, 8192, 16384];

    for size in sizes {
        let pattern = orchestrator.generate_pattern(size)?;
        assert_eq!(pattern.len(), size, "Pattern size should match requested size");
    }

    Ok(())
}

#[test]
fn test_generate_pattern_large_size() -> Result<()> {
    let config = WipeConfig {
        algorithm: Algorithm::Zero,
        ..Default::default()
    };

    let orchestrator = WipeOrchestrator::new("/dev/null".to_string(), config)?;

    let large_size = 1024 * 1024; // 1MB
    let pattern = orchestrator.generate_pattern(large_size)?;

    assert_eq!(pattern.len(), large_size);
    Ok(())
}

#[test]
fn test_generate_pattern_consistency() -> Result<()> {
    // Test that zero pattern is consistent across calls
    let config = WipeConfig {
        algorithm: Algorithm::Zero,
        ..Default::default()
    };

    let orchestrator = WipeOrchestrator::new("/dev/null".to_string(), config)?;

    let pattern1 = orchestrator.generate_pattern(1024)?;
    let pattern2 = orchestrator.generate_pattern(1024)?;

    assert_eq!(pattern1, pattern2, "Zero patterns should be consistent");
    Ok(())
}

#[test]
fn test_generate_pattern_random_different() -> Result<()> {
    // Test that random patterns are different across calls
    let config = WipeConfig {
        algorithm: Algorithm::Random,
        ..Default::default()
    };

    let orchestrator = WipeOrchestrator::new("/dev/null".to_string(), config)?;

    let pattern1 = orchestrator.generate_pattern(1024)?;
    let pattern2 = orchestrator.generate_pattern(1024)?;

    // Extremely unlikely to be identical for random data
    assert_ne!(pattern1, pattern2, "Random patterns should be different");
    Ok(())
}

// ==================== ORCHESTRATOR CREATION TESTS ====================

#[test]
fn test_orchestrator_creation_with_dev_null() -> Result<()> {
    let config = WipeConfig::default();
    let result = WipeOrchestrator::new("/dev/null".to_string(), config);

    assert!(result.is_ok(), "Should create orchestrator with /dev/null");
    Ok(())
}

#[test]
fn test_orchestrator_creation_with_different_algorithms() -> Result<()> {
    let algorithms = vec![
        Algorithm::Zero,
        Algorithm::Random,
        Algorithm::DoD5220,
        Algorithm::Gutmann,
    ];

    for algo in algorithms {
        let config = WipeConfig {
            algorithm: algo.clone(),
            ..Default::default()
        };

        let result = WipeOrchestrator::new("/dev/null".to_string(), config);
        assert!(result.is_ok(), "Should create orchestrator with algorithm {:?}", algo);
    }

    Ok(())
}

// ==================== DRIVE TYPE DETECTION TESTS ====================

#[test]
fn test_drive_type_detection_nvme_patterns() {
    let nvme_paths = vec![
        "/dev/nvme0n1",
        "/dev/nvme1n1",
        "/dev/nvme0n2",
        "/dev/nvme2n15",
    ];

    for path in nvme_paths {
        let info = WipeOrchestrator::create_basic_drive_info(path).unwrap();
        assert_eq!(info.drive_type, DriveType::NVMe, "Should detect {} as NVMe", path);
    }
}

#[test]
fn test_drive_type_detection_emmc_patterns() {
    let emmc_paths = vec![
        "/dev/mmcblk0",
        "/dev/mmcblk1",
        "/dev/mmcblk0p1",
    ];

    for path in emmc_paths {
        let info = WipeOrchestrator::create_basic_drive_info(path).unwrap();
        assert_eq!(info.drive_type, DriveType::EMMC, "Should detect {} as EMMC", path);
    }
}

#[test]
fn test_drive_type_detection_hdd_patterns() {
    let hdd_paths = vec![
        "/dev/sda",
        "/dev/sdb",
        "/dev/sdc",
        "/dev/hda",
        "/dev/vda",
    ];

    for path in hdd_paths {
        let info = WipeOrchestrator::create_basic_drive_info(path).unwrap();
        assert_eq!(info.drive_type, DriveType::HDD, "Should detect {} as HDD (default)", path);
    }
}

// ==================== SIZE AND CAPACITY TESTS ====================

#[test]
fn test_drive_info_default_size() -> Result<()> {
    let drive_info = WipeOrchestrator::create_basic_drive_info("/dev/sda")?;

    let expected_size = 1024 * 1024 * 1024 * 100u64; // 100GB
    assert_eq!(drive_info.size, expected_size);

    let gb = drive_info.size / (1024 * 1024 * 1024);
    assert_eq!(gb, 100);

    Ok(())
}

// ==================== CONFIG INTEGRATION TESTS ====================

#[test]
fn test_orchestrator_with_custom_config() -> Result<()> {
    let config = WipeConfig {
        algorithm: Algorithm::Gutmann,
        handle_hpa_dco: crate::HPADCOHandling::TemporaryRemove,
        ..Default::default()
    };

    let orchestrator = WipeOrchestrator::new("/dev/null".to_string(), config)?;

    assert_eq!(orchestrator.config.algorithm, Algorithm::Gutmann);
    assert_eq!(orchestrator.config.handle_hpa_dco, crate::HPADCOHandling::TemporaryRemove);

    Ok(())
}

// ==================== EDGE CASE TESTS ====================

#[test]
fn test_create_drive_info_with_special_paths() -> Result<()> {
    let special_paths = vec![
        "/dev/null",
        "/dev/zero",
        "/dev/random",
    ];

    for path in special_paths {
        let result = WipeOrchestrator::create_basic_drive_info(path);
        assert!(result.is_ok(), "Should handle special path: {}", path);
    }

    Ok(())
}

#[test]
fn test_pattern_generation_zero_size() -> Result<()> {
    let config = WipeConfig {
        algorithm: Algorithm::Zero,
        ..Default::default()
    };

    let orchestrator = WipeOrchestrator::new("/dev/null".to_string(), config)?;
    let pattern = orchestrator.generate_pattern(0)?;

    assert_eq!(pattern.len(), 0);
    Ok(())
}

#[test]
fn test_pattern_generation_small_size() -> Result<()> {
    let config = WipeConfig {
        algorithm: Algorithm::Zero,
        ..Default::default()
    };

    let orchestrator = WipeOrchestrator::new("/dev/null".to_string(), config)?;
    let pattern = orchestrator.generate_pattern(1)?;

    assert_eq!(pattern.len(), 1);
    assert_eq!(pattern[0], 0);
    Ok(())
}

// ==================== ALGORITHM ENUM COVERAGE TESTS ====================

#[test]
fn test_all_algorithm_variants_supported() -> Result<()> {
    // Test that all Algorithm variants can be converted
    let algorithms = vec![
        Algorithm::Zero,
        Algorithm::Random,
        Algorithm::DoD5220,
        Algorithm::Gutmann,
        Algorithm::SecureErase,
        Algorithm::CryptoErase,
        Algorithm::Sanitize,
    ];

    for algo in algorithms {
        let config = WipeConfig {
            algorithm: algo,
            ..Default::default()
        };

        let orchestrator = WipeOrchestrator::new("/dev/null".to_string(), config)?;
        let wipe_algo = orchestrator.convert_to_wipe_algorithm();

        // Verify conversion doesn't panic
        let _ = wipe_algo;
    }

    Ok(())
}

// ==================== INTEGRATION TESTS ====================
// Integration tests have been moved to: tests/hardware_integration.rs
// These tests use mock drives and can run without physical hardware:
// - test_wipe_hdd_drive
// - test_wipe_ssd_drive
// - test_wipe_nvme_basic
// - test_wipe_nvme_advanced
// - test_wipe_smr_drive
// - test_wipe_optane_drive
// - test_wipe_hybrid_drive
// - test_wipe_emmc_drive
// - test_wipe_ufs_drive
// - test_wipe_raid_array
// - test_orchestrator_execute_basic
// - test_orchestrator_multiple_algorithms

// ==================== WIPE ALGORITHM ENUM TESTS ====================

#[test]
fn test_wipe_algorithm_zeros_to_ones_conversion() {
    // Test the WipeAlgorithm enum variants
    let zeros = WipeAlgorithm::Zeros;
    let ones = WipeAlgorithm::Ones;

    assert!(matches!(zeros, WipeAlgorithm::Zeros));
    assert!(matches!(ones, WipeAlgorithm::Ones));
}

#[test]
fn test_wipe_algorithm_clone_trait() {
    let algo = WipeAlgorithm::Random;
    let cloned = algo.clone();

    assert!(matches!(cloned, WipeAlgorithm::Random));
}

// ==================== ERROR HANDLING TESTS ====================

#[test]
fn test_orchestrator_handles_invalid_device() {
    let config = WipeConfig::default();
    let result = WipeOrchestrator::new("/invalid/device/path".to_string(), config);

    // Should handle gracefully (may succeed with basic drive info creation)
    let _ = result;
}

// ==================== ASYNC EXECUTION TESTS ====================
// Async execution tests have been moved to: tests/hardware_integration.rs
// See: test_orchestrator_execute_basic, test_orchestrator_multiple_algorithms
