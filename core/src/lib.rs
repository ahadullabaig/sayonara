// Allow uppercase acronyms for industry-standard terms like HDD, SSD, SMR, EMMC
#![allow(clippy::upper_case_acronyms)]
// Allow complex types where needed for comprehensive error handling and configuration
#![allow(clippy::type_complexity)]

pub mod algorithms;
pub mod crypto;
pub mod drives;
pub mod error;
pub mod io;
pub mod ui;
pub mod verification;
pub mod wipe_orchestrator;

// Re-export main wipe orchestrator for convenience
pub use wipe_orchestrator::{wipe_drive, WipeOrchestrator};

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use thiserror::Error;

// Global flag for handling Ctrl+C interrupts
static INTERRUPTED: AtomicBool = AtomicBool::new(false);

/// Set the interrupt flag (called by signal handler)
pub fn set_interrupted() {
    INTERRUPTED.store(true, Ordering::SeqCst);
}

/// Check if an interrupt has been received
pub fn is_interrupted() -> bool {
    INTERRUPTED.load(Ordering::SeqCst)
}

/// Reset the interrupt flag (primarily for testing)
pub fn reset_interrupted() {
    INTERRUPTED.store(false, Ordering::SeqCst);
}

// Enhanced error types for better error handling
#[derive(Error, Debug)]
pub enum DriveError {
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Drive is frozen and cannot be modified: {0}")]
    DriveFrozen(String),

    #[error("Hardware command failed: {0}")]
    HardwareCommandFailed(String),

    #[error("SMART read failed: {0}")]
    SMARTReadFailed(String),

    #[error("Temperature exceeded safe limits: {0}")]
    TemperatureExceeded(String),

    #[error("TRIM operation failed: {0}")]
    TRIMFailed(String),

    #[error("Cryptographic erase failed: {0}")]
    CryptoEraseFailed(String),

    #[error("Drive unlock failed: {0}")]
    UnlockFailed(String),

    #[error("Operation timeout: {0}")]
    Timeout(String),

    #[error("Insufficient permissions: {0}")]
    PermissionDenied(String),

    #[error("Drive not found: {0}")]
    NotFound(String),

    #[error("Unsupported operation: {0}")]
    Unsupported(String),

    #[error("Operation interrupted by user")]
    Interrupted,
}

// Manual Clone implementation because std::io::Error doesn't implement Clone
impl Clone for DriveError {
    fn clone(&self) -> Self {
        match self {
            DriveError::IoError(e) => {
                DriveError::IoError(std::io::Error::new(e.kind(), e.to_string()))
            }
            DriveError::DriveFrozen(s) => DriveError::DriveFrozen(s.clone()),
            DriveError::HardwareCommandFailed(s) => DriveError::HardwareCommandFailed(s.clone()),
            DriveError::SMARTReadFailed(s) => DriveError::SMARTReadFailed(s.clone()),
            DriveError::TemperatureExceeded(s) => DriveError::TemperatureExceeded(s.clone()),
            DriveError::TRIMFailed(s) => DriveError::TRIMFailed(s.clone()),
            DriveError::CryptoEraseFailed(s) => DriveError::CryptoEraseFailed(s.clone()),
            DriveError::UnlockFailed(s) => DriveError::UnlockFailed(s.clone()),
            DriveError::Timeout(s) => DriveError::Timeout(s.clone()),
            DriveError::PermissionDenied(s) => DriveError::PermissionDenied(s.clone()),
            DriveError::NotFound(s) => DriveError::NotFound(s.clone()),
            DriveError::Unsupported(s) => DriveError::Unsupported(s.clone()),
            DriveError::Interrupted => DriveError::Interrupted,
        }
    }
}

impl From<anyhow::Error> for DriveError {
    fn from(err: anyhow::Error) -> Self {
        // Map to the most appropriate variant based on error message
        DriveError::HardwareCommandFailed(err.to_string())
    }
}

pub type DriveResult<T> = Result<T, DriveError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WipeConfig {
    pub algorithm: Algorithm,
    pub verify: bool,
    pub multiple_passes: Option<u32>,
    pub preserve_partition_table: bool,
    pub unlock_encrypted: bool,
    pub handle_hpa_dco: HPADCOHandling,
    pub use_trim_after: bool,
    pub temperature_monitoring: bool,
    pub max_temperature_celsius: Option<u32>,
    pub freeze_mitigation: bool,
    pub sed_crypto_erase: bool,
}

impl Default for WipeConfig {
    fn default() -> Self {
        Self {
            algorithm: Algorithm::DoD5220,
            verify: true,
            multiple_passes: None,
            preserve_partition_table: false,
            unlock_encrypted: false,
            handle_hpa_dco: HPADCOHandling::Detect,
            use_trim_after: true,
            temperature_monitoring: true,
            max_temperature_celsius: Some(65),
            freeze_mitigation: true,
            sed_crypto_erase: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HPADCOHandling {
    Ignore,          // Don't check for HPA/DCO
    Detect,          // Detect and warn only
    TemporaryRemove, // Remove during wipe, restore after
    PermanentRemove, // Remove permanently (dangerous)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Algorithm {
    DoD5220,     // 3-pass DoD 5220.22-M
    Gutmann,     // 35-pass Gutmann
    Random,      // Single pass random
    Zero,        // Single pass zeros
    SecureErase, // Hardware secure erase
    CryptoErase, // Cryptographic erase (SED)
    Sanitize,    // NVMe sanitize command
    TrimOnly,    // TRIM/discard only (SSD)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriveInfo {
    pub device_path: String,
    pub model: String,
    pub serial: String,
    pub size: u64,
    pub drive_type: DriveType,
    pub encryption_status: EncryptionStatus,
    pub capabilities: DriveCapabilities,
    pub health_status: Option<HealthStatus>,
    pub temperature_celsius: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriveCapabilities {
    pub secure_erase: bool,
    pub enhanced_erase: bool,
    pub crypto_erase: bool,
    pub trim_support: bool,
    pub hpa_enabled: bool,
    pub dco_enabled: bool,
    pub sed_type: Option<SEDType>,
    pub sanitize_options: Vec<SanitizeOption>,
    pub max_temperature: Option<u32>,
    pub is_frozen: bool,
    pub freeze_status: FreezeStatus,
}

impl Default for DriveCapabilities {
    fn default() -> Self {
        Self {
            secure_erase: false,
            enhanced_erase: false,
            crypto_erase: false,
            trim_support: false,
            hpa_enabled: false,
            dco_enabled: false,
            sed_type: None,
            sanitize_options: Vec::new(),
            max_temperature: None,
            is_frozen: false,
            freeze_status: FreezeStatus::NotFrozen,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DriveType {
    HDD,
    SSD,
    NVMe,
    USB,
    RAID,
    SMR,        // Shingled Magnetic Recording (Host-Managed/Aware)
    Optane,     // Intel Optane / 3D XPoint
    HybridSSHD, // Hybrid HDD + SSD cache
    EMMC,       // Embedded MultiMediaCard
    UFS,        // Universal Flash Storage
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EncryptionStatus {
    None,
    OPAL,
    BitLocker,
    LUKS,
    FileVault,
    VeraCrypt,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SEDType {
    OPAL20,
    OPAL10,
    TCGEnterprise,
    ATASecurity,
    EDrive,
    Proprietary(String),
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SanitizeOption {
    BlockErase,
    CryptoErase,
    Overwrite,
    CryptoScramble,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum FreezeStatus {
    NotFrozen,
    Frozen,
    FrozenByBIOS,
    SecurityLocked,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HealthStatus {
    Good,
    Warning,
    Critical,
    Failed,
    Unknown,
}

// Operation status for progress tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationStatus {
    pub phase: WipePhase,
    pub progress: f64,
    pub bytes_processed: Option<u64>,
    pub total_bytes: Option<u64>,
    pub current_temperature: Option<u32>,
    pub estimated_time_remaining: Option<u64>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WipePhase {
    Preparing,
    UnfreezingDrive,
    RemovingHPA,
    RemovingDCO,
    CryptoErase,
    Overwriting,
    TrimOperation,
    Verification,
    RestoringConfig,
    GeneratingCertificate,
    Complete,
}

// Wipe session for tracking multiple drive operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WipeSession {
    pub session_id: String,
    pub start_time: chrono::DateTime<chrono::Utc>,
    pub end_time: Option<chrono::DateTime<chrono::Utc>>,
    pub drives: Vec<DriveWipeRecord>,
    pub config: WipeConfig,
    pub operator_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriveWipeRecord {
    pub drive_info: DriveInfo,
    pub status: WipeStatus,
    pub start_time: chrono::DateTime<chrono::Utc>,
    pub end_time: Option<chrono::DateTime<chrono::Utc>>,
    pub error_message: Option<String>,
    pub certificate_path: Option<String>,
    pub verification_passed: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WipeStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Skipped,
}

// Safety configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyConfig {
    pub require_confirmation: bool,
    pub exclude_system_drives: bool,
    pub exclude_mounted_drives: bool,
    pub temperature_check_interval_secs: u64,
    pub max_retry_attempts: u32,
    pub operation_timeout_secs: u64,
    pub preserve_raid_metadata: bool,
}

impl Default for SafetyConfig {
    fn default() -> Self {
        Self {
            require_confirmation: true,
            exclude_system_drives: true,
            exclude_mounted_drives: true,
            temperature_check_interval_secs: 60,
            max_retry_attempts: 3,
            operation_timeout_secs: 3600,
            preserve_raid_metadata: true,
        }
    }
}

#[cfg(test)]
mod lib_tests;
