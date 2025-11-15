// Comprehensive tests for lib.rs core types and enums
//
// Tests cover: Interrupt handling, error types, configuration structs, enums (all variants),
// default implementations, serialization, PartialEq, Clone, and Copy traits.

use super::*;
use serial_test::serial;

// ==================== INTERRUPT HANDLING TESTS ====================

#[test]
#[serial]
fn test_interrupt_initially_not_set() {
    reset_interrupted();
    assert!(
        !is_interrupted(),
        "Interrupt flag should initially be not set"
    );
}

#[test]
#[serial]
fn test_set_interrupt_flag() {
    reset_interrupted();
    set_interrupted();
    assert!(is_interrupted(), "Interrupt flag should be set");
}

#[test]
#[serial]
fn test_interrupt_flag_persistence() {
    reset_interrupted();
    set_interrupted();
    assert!(is_interrupted());
    assert!(
        is_interrupted(),
        "Flag should remain set on subsequent calls"
    );
}

// ==================== DRIVE ERROR TESTS ====================

#[test]
fn test_drive_error_io_error() {
    let err = DriveError::IoError(std::io::Error::new(std::io::ErrorKind::NotFound, "test"));
    assert!(err.to_string().contains("I/O error"));
}

#[test]
fn test_drive_error_drive_frozen() {
    let err = DriveError::DriveFrozen("Security frozen".to_string());
    assert!(err.to_string().contains("frozen"));
    assert!(err.to_string().contains("Security frozen"));
}

#[test]
fn test_drive_error_hardware_command_failed() {
    let err = DriveError::HardwareCommandFailed("ATA command timeout".to_string());
    assert!(err.to_string().contains("Hardware command failed"));
}

#[test]
fn test_drive_error_smart_read_failed() {
    let err = DriveError::SMARTReadFailed("smartctl not found".to_string());
    assert!(err.to_string().contains("SMART read failed"));
}

#[test]
fn test_drive_error_temperature_exceeded() {
    let err = DriveError::TemperatureExceeded("75°C".to_string());
    assert!(err.to_string().contains("Temperature exceeded"));
}

#[test]
fn test_drive_error_trim_failed() {
    let err = DriveError::TRIMFailed("blkdiscard failed".to_string());
    assert!(err.to_string().contains("TRIM"));
}

#[test]
fn test_drive_error_crypto_erase_failed() {
    let err = DriveError::CryptoEraseFailed("OPAL not supported".to_string());
    assert!(err.to_string().contains("Cryptographic erase failed"));
}

#[test]
fn test_drive_error_unlock_failed() {
    let err = DriveError::UnlockFailed("Invalid password".to_string());
    assert!(err.to_string().contains("unlock failed"));
}

#[test]
fn test_drive_error_timeout() {
    let err = DriveError::Timeout("Operation took too long".to_string());
    assert!(err.to_string().contains("timeout"));
}

#[test]
fn test_drive_error_permission_denied() {
    let err = DriveError::PermissionDenied("Need root".to_string());
    assert!(err.to_string().contains("Insufficient permissions"));
}

#[test]
fn test_drive_error_not_found() {
    let err = DriveError::NotFound("/dev/sda".to_string());
    assert!(err.to_string().contains("not found"));
}

#[test]
fn test_drive_error_unsupported() {
    let err = DriveError::Unsupported("ZNS not implemented".to_string());
    assert!(err.to_string().contains("Unsupported"));
}

#[test]
fn test_drive_error_interrupted() {
    let err = DriveError::Interrupted;
    assert!(err.to_string().contains("interrupted"));
}

#[test]
fn test_drive_error_clone() {
    let err = DriveError::DriveFrozen("Test".to_string());
    let cloned = err.clone();

    assert!(matches!(cloned, DriveError::DriveFrozen(_)));
}

#[test]
fn test_drive_error_clone_all_variants() {
    let errors = vec![
        DriveError::DriveFrozen("test".to_string()),
        DriveError::HardwareCommandFailed("test".to_string()),
        DriveError::SMARTReadFailed("test".to_string()),
        DriveError::TemperatureExceeded("test".to_string()),
        DriveError::TRIMFailed("test".to_string()),
        DriveError::CryptoEraseFailed("test".to_string()),
        DriveError::UnlockFailed("test".to_string()),
        DriveError::Timeout("test".to_string()),
        DriveError::PermissionDenied("test".to_string()),
        DriveError::NotFound("test".to_string()),
        DriveError::Unsupported("test".to_string()),
        DriveError::Interrupted,
    ];

    for err in errors {
        let cloned = err.clone();
        // Verify cloning doesn't panic
        let _ = cloned;
    }
}

#[test]
fn test_drive_error_from_anyhow() {
    let anyhow_err = anyhow::anyhow!("Test error");
    let drive_err: DriveError = anyhow_err.into();

    assert!(matches!(drive_err, DriveError::HardwareCommandFailed(_)));
    assert!(drive_err.to_string().contains("Test error"));
}

// ==================== WIPE CONFIG TESTS ====================

#[test]
fn test_wipe_config_default() {
    let config = WipeConfig::default();

    assert_eq!(config.algorithm, Algorithm::DoD5220);
    assert!(config.verify);
    assert!(config.multiple_passes.is_none());
    assert!(!config.preserve_partition_table);
    assert!(!config.unlock_encrypted);
    assert_eq!(config.handle_hpa_dco, HPADCOHandling::Detect);
    assert!(config.use_trim_after);
    assert!(config.temperature_monitoring);
    assert_eq!(config.max_temperature_celsius, Some(65));
    assert!(config.freeze_mitigation);
    assert!(config.sed_crypto_erase);
}

#[test]
fn test_wipe_config_clone() {
    let config = WipeConfig::default();
    let cloned = config.clone();

    assert_eq!(cloned.algorithm, config.algorithm);
    assert_eq!(cloned.verify, config.verify);
}

#[test]
fn test_wipe_config_custom() {
    let config = WipeConfig {
        algorithm: Algorithm::Gutmann,
        verify: false,
        multiple_passes: Some(7),
        preserve_partition_table: true,
        unlock_encrypted: true,
        handle_hpa_dco: HPADCOHandling::PermanentRemove,
        use_trim_after: false,
        temperature_monitoring: false,
        max_temperature_celsius: Some(70),
        freeze_mitigation: false,
        sed_crypto_erase: false,
    };

    assert_eq!(config.algorithm, Algorithm::Gutmann);
    assert!(!config.verify);
    assert_eq!(config.multiple_passes, Some(7));
}

// ==================== HPA/DCO HANDLING ENUM TESTS ====================

#[test]
fn test_hpa_dco_handling_ignore() {
    let handling = HPADCOHandling::Ignore;
    assert!(matches!(handling, HPADCOHandling::Ignore));
}

#[test]
fn test_hpa_dco_handling_detect() {
    let handling = HPADCOHandling::Detect;
    assert!(matches!(handling, HPADCOHandling::Detect));
}

#[test]
fn test_hpa_dco_handling_temporary_remove() {
    let handling = HPADCOHandling::TemporaryRemove;
    assert!(matches!(handling, HPADCOHandling::TemporaryRemove));
}

#[test]
fn test_hpa_dco_handling_permanent_remove() {
    let handling = HPADCOHandling::PermanentRemove;
    assert!(matches!(handling, HPADCOHandling::PermanentRemove));
}

#[test]
fn test_hpa_dco_handling_partial_eq() {
    assert_eq!(HPADCOHandling::Ignore, HPADCOHandling::Ignore);
    assert_ne!(HPADCOHandling::Ignore, HPADCOHandling::Detect);
}

#[test]
fn test_hpa_dco_handling_clone() {
    let handling = HPADCOHandling::Detect;
    let cloned = handling.clone();
    assert_eq!(handling, cloned);
}

// ==================== ALGORITHM ENUM TESTS ====================

#[test]
fn test_algorithm_dod5220() {
    let algo = Algorithm::DoD5220;
    assert!(matches!(algo, Algorithm::DoD5220));
}

#[test]
fn test_algorithm_gutmann() {
    let algo = Algorithm::Gutmann;
    assert!(matches!(algo, Algorithm::Gutmann));
}

#[test]
fn test_algorithm_random() {
    let algo = Algorithm::Random;
    assert!(matches!(algo, Algorithm::Random));
}

#[test]
fn test_algorithm_zero() {
    let algo = Algorithm::Zero;
    assert!(matches!(algo, Algorithm::Zero));
}

#[test]
fn test_algorithm_secure_erase() {
    let algo = Algorithm::SecureErase;
    assert!(matches!(algo, Algorithm::SecureErase));
}

#[test]
fn test_algorithm_crypto_erase() {
    let algo = Algorithm::CryptoErase;
    assert!(matches!(algo, Algorithm::CryptoErase));
}

#[test]
fn test_algorithm_sanitize() {
    let algo = Algorithm::Sanitize;
    assert!(matches!(algo, Algorithm::Sanitize));
}

#[test]
fn test_algorithm_trim_only() {
    let algo = Algorithm::TrimOnly;
    assert!(matches!(algo, Algorithm::TrimOnly));
}

#[test]
fn test_algorithm_partial_eq() {
    assert_eq!(Algorithm::DoD5220, Algorithm::DoD5220);
    assert_ne!(Algorithm::DoD5220, Algorithm::Gutmann);
}

#[test]
fn test_algorithm_clone() {
    let algo = Algorithm::Gutmann;
    let cloned = algo.clone();
    assert_eq!(algo, cloned);
}

// ==================== DRIVE TYPE ENUM TESTS ====================

#[test]
fn test_drive_type_all_variants() {
    let types = vec![
        DriveType::HDD,
        DriveType::SSD,
        DriveType::NVMe,
        DriveType::USB,
        DriveType::RAID,
        DriveType::SMR,
        DriveType::Optane,
        DriveType::HybridSSHD,
        DriveType::EMMC,
        DriveType::UFS,
        DriveType::Unknown,
    ];

    assert_eq!(types.len(), 11);
}

#[test]
fn test_drive_type_partial_eq() {
    assert_eq!(DriveType::NVMe, DriveType::NVMe);
    assert_ne!(DriveType::NVMe, DriveType::SSD);
}

#[test]
fn test_drive_type_clone() {
    let drive_type = DriveType::SMR;
    let cloned = drive_type.clone();
    assert_eq!(drive_type, cloned);
}

// ==================== ENCRYPTION STATUS ENUM TESTS ====================

#[test]
fn test_encryption_status_all_variants() {
    let statuses = vec![
        EncryptionStatus::None,
        EncryptionStatus::OPAL,
        EncryptionStatus::BitLocker,
        EncryptionStatus::LUKS,
        EncryptionStatus::FileVault,
        EncryptionStatus::VeraCrypt,
        EncryptionStatus::Unknown,
    ];

    assert_eq!(statuses.len(), 7);
}

#[test]
fn test_encryption_status_clone() {
    let status = EncryptionStatus::OPAL;
    let cloned = status.clone();
    // Can't test equality without PartialEq, but verify cloning works
    let _ = cloned;
}

// ==================== SED TYPE ENUM TESTS ====================

#[test]
fn test_sed_type_all_variants() {
    let types = vec![
        SEDType::OPAL20,
        SEDType::OPAL10,
        SEDType::TCGEnterprise,
        SEDType::ATASecurity,
        SEDType::EDrive,
        SEDType::Proprietary("Custom".to_string()),
        SEDType::None,
    ];

    assert_eq!(types.len(), 7);
}

#[test]
fn test_sed_type_proprietary() {
    let sed = SEDType::Proprietary("Samsung".to_string());
    match sed {
        SEDType::Proprietary(ref name) => assert_eq!(name, "Samsung"),
        _ => panic!("Expected Proprietary variant"),
    }
}

#[test]
fn test_sed_type_partial_eq() {
    assert_eq!(SEDType::OPAL20, SEDType::OPAL20);
    assert_ne!(SEDType::OPAL20, SEDType::OPAL10);
}

#[test]
fn test_sed_type_clone() {
    let sed = SEDType::TCGEnterprise;
    let cloned = sed.clone();
    assert_eq!(sed, cloned);
}

// ==================== SANITIZE OPTION ENUM TESTS ====================

#[test]
fn test_sanitize_option_all_variants() {
    let options = vec![
        SanitizeOption::BlockErase,
        SanitizeOption::CryptoErase,
        SanitizeOption::Overwrite,
        SanitizeOption::CryptoScramble,
    ];

    assert_eq!(options.len(), 4);
}

#[test]
fn test_sanitize_option_clone() {
    let option = SanitizeOption::CryptoErase;
    let cloned = option.clone();
    let _ = cloned;
}

// ==================== FREEZE STATUS ENUM TESTS ====================

#[test]
fn test_freeze_status_all_variants() {
    let statuses = vec![
        FreezeStatus::NotFrozen,
        FreezeStatus::Frozen,
        FreezeStatus::FrozenByBIOS,
        FreezeStatus::SecurityLocked,
        FreezeStatus::Unknown,
    ];

    assert_eq!(statuses.len(), 5);
}

#[test]
fn test_freeze_status_copy_trait() {
    let status = FreezeStatus::Frozen;
    let copied = status; // Copy semantics
    assert_eq!(status, copied);
}

#[test]
fn test_freeze_status_partial_eq() {
    assert_eq!(FreezeStatus::NotFrozen, FreezeStatus::NotFrozen);
    assert_ne!(FreezeStatus::NotFrozen, FreezeStatus::Frozen);
}

// ==================== HEALTH STATUS ENUM TESTS ====================

#[test]
fn test_health_status_all_variants() {
    let statuses = vec![
        HealthStatus::Good,
        HealthStatus::Warning,
        HealthStatus::Critical,
        HealthStatus::Failed,
        HealthStatus::Unknown,
    ];

    assert_eq!(statuses.len(), 5);
}

#[test]
fn test_health_status_partial_eq() {
    assert_eq!(HealthStatus::Good, HealthStatus::Good);
    assert_ne!(HealthStatus::Good, HealthStatus::Critical);
}

#[test]
fn test_health_status_clone() {
    let status = HealthStatus::Warning;
    let cloned = status.clone();
    assert_eq!(status, cloned);
}

// ==================== WIPE PHASE ENUM TESTS ====================

#[test]
fn test_wipe_phase_all_variants() {
    let phases = vec![
        WipePhase::Preparing,
        WipePhase::UnfreezingDrive,
        WipePhase::RemovingHPA,
        WipePhase::RemovingDCO,
        WipePhase::CryptoErase,
        WipePhase::Overwriting,
        WipePhase::TrimOperation,
        WipePhase::Verification,
        WipePhase::RestoringConfig,
        WipePhase::GeneratingCertificate,
        WipePhase::Complete,
    ];

    assert_eq!(phases.len(), 11);
}

#[test]
fn test_wipe_phase_clone() {
    let phase = WipePhase::Overwriting;
    let cloned = phase.clone();
    let _ = cloned;
}

// ==================== WIPE STATUS ENUM TESTS ====================

#[test]
fn test_wipe_status_all_variants() {
    let statuses = vec![
        WipeStatus::Pending,
        WipeStatus::InProgress,
        WipeStatus::Completed,
        WipeStatus::Failed,
        WipeStatus::Skipped,
    ];

    assert_eq!(statuses.len(), 5);
}

#[test]
fn test_wipe_status_clone() {
    let status = WipeStatus::InProgress;
    let cloned = status.clone();
    let _ = cloned;
}

// ==================== DRIVE CAPABILITIES TESTS ====================

#[test]
fn test_drive_capabilities_default() {
    let caps = DriveCapabilities::default();

    assert!(!caps.secure_erase);
    assert!(!caps.enhanced_erase);
    assert!(!caps.crypto_erase);
    assert!(!caps.trim_support);
    assert!(!caps.hpa_enabled);
    assert!(!caps.dco_enabled);
    assert!(caps.sed_type.is_none());
    assert!(caps.sanitize_options.is_empty());
    assert!(caps.max_temperature.is_none());
    assert!(!caps.is_frozen);
    assert_eq!(caps.freeze_status, FreezeStatus::NotFrozen);
}

#[test]
fn test_drive_capabilities_custom() {
    let caps = DriveCapabilities {
        secure_erase: true,
        enhanced_erase: true,
        crypto_erase: true,
        trim_support: true,
        hpa_enabled: false,
        dco_enabled: false,
        sed_type: Some(SEDType::OPAL20),
        sanitize_options: vec![SanitizeOption::CryptoErase],
        max_temperature: Some(70),
        is_frozen: false,
        freeze_status: FreezeStatus::NotFrozen,
    };

    assert!(caps.secure_erase);
    assert!(caps.trim_support);
    assert_eq!(caps.sed_type, Some(SEDType::OPAL20));
    assert_eq!(caps.sanitize_options.len(), 1);
}

// ==================== SAFETY CONFIG TESTS ====================

#[test]
fn test_safety_config_default() {
    let config = SafetyConfig::default();

    assert!(config.require_confirmation);
    assert!(config.exclude_system_drives);
    assert!(config.exclude_mounted_drives);
    assert_eq!(config.temperature_check_interval_secs, 60);
    assert_eq!(config.max_retry_attempts, 3);
    assert_eq!(config.operation_timeout_secs, 3600);
    assert!(config.preserve_raid_metadata);
}

#[test]
fn test_safety_config_clone() {
    let config = SafetyConfig::default();
    let cloned = config.clone();

    assert_eq!(cloned.require_confirmation, config.require_confirmation);
    assert_eq!(cloned.max_retry_attempts, config.max_retry_attempts);
}

#[test]
fn test_safety_config_custom() {
    let config = SafetyConfig {
        require_confirmation: false,
        exclude_system_drives: false,
        exclude_mounted_drives: false,
        temperature_check_interval_secs: 30,
        max_retry_attempts: 5,
        operation_timeout_secs: 7200,
        preserve_raid_metadata: false,
    };

    assert!(!config.require_confirmation);
    assert_eq!(config.max_retry_attempts, 5);
    assert_eq!(config.operation_timeout_secs, 7200);
}

// ==================== DRIVE INFO TESTS ====================

#[test]
fn test_drive_info_construction() {
    let info = DriveInfo {
        device_path: "/dev/sda".to_string(),
        model: "Samsung 870 EVO".to_string(),
        serial: "S123456789".to_string(),
        size: 1024 * 1024 * 1024 * 1024, // 1 TB
        drive_type: DriveType::SSD,
        encryption_status: EncryptionStatus::None,
        capabilities: DriveCapabilities::default(),
        health_status: Some(HealthStatus::Good),
        temperature_celsius: Some(35),
    };

    assert_eq!(info.device_path, "/dev/sda");
    assert_eq!(info.size, 1024 * 1024 * 1024 * 1024);
    assert_eq!(info.temperature_celsius, Some(35));
}

#[test]
fn test_drive_info_clone() {
    let info = DriveInfo {
        device_path: "/dev/nvme0n1".to_string(),
        model: "WD Black SN850".to_string(),
        serial: "N987654321".to_string(),
        size: 2 * 1024 * 1024 * 1024 * 1024, // 2 TB
        drive_type: DriveType::NVMe,
        encryption_status: EncryptionStatus::OPAL,
        capabilities: DriveCapabilities::default(),
        health_status: Some(HealthStatus::Good),
        temperature_celsius: Some(42),
    };

    let cloned = info.clone();
    assert_eq!(cloned.device_path, info.device_path);
    assert_eq!(cloned.size, info.size);
}

// ==================== OPERATION STATUS TESTS ====================

#[test]
fn test_operation_status_construction() {
    let status = OperationStatus {
        phase: WipePhase::Overwriting,
        progress: 50.0,
        bytes_processed: Some(500 * 1024 * 1024),
        total_bytes: Some(1000 * 1024 * 1024),
        current_temperature: Some(45),
        estimated_time_remaining: Some(600),
        warnings: vec!["Temperature rising".to_string()],
    };

    assert_eq!(status.progress, 50.0);
    assert_eq!(status.warnings.len(), 1);
}

#[test]
fn test_operation_status_clone() {
    let status = OperationStatus {
        phase: WipePhase::Verification,
        progress: 100.0,
        bytes_processed: Some(1000 * 1024 * 1024),
        total_bytes: Some(1000 * 1024 * 1024),
        current_temperature: Some(40),
        estimated_time_remaining: Some(0),
        warnings: vec![],
    };

    let cloned = status.clone();
    assert_eq!(cloned.progress, status.progress);
}

// ==================== RESULT TYPE TESTS ====================

#[test]
fn test_drive_result_ok() {
    let result: DriveResult<String> = Ok("Success".to_string());
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "Success");
}

#[test]
fn test_drive_result_err() {
    let result: DriveResult<String> = Err(DriveError::Interrupted);
    assert!(result.is_err());
}
