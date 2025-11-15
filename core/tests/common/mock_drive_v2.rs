use sayonara_wipe::{
    DriveCapabilities, DriveInfo, DriveType, EncryptionStatus, FreezeStatus, HealthStatus, SEDType,
    SanitizeOption,
};
/// Enhanced Mock Drive Infrastructure v2
///
/// Provides comprehensive mock drives for testing wipe operations without physical hardware.
/// Supports all drive types with behavioral simulation, error injection, and verification.
///
/// Features:
/// - File-backed storage for realistic I/O
/// - Drive-specific behavioral simulation (zones, namespaces, etc.)
/// - Temperature simulation
/// - Bad sector and error injection
/// - Operation tracking and statistics
/// - Post-wipe verification
use std::io::{Read, Seek, SeekFrom, Write};
use std::sync::{Arc, Mutex};
use tempfile::NamedTempFile;

/// Zone model for SMR drives
#[derive(Clone, Debug, PartialEq)]
#[allow(dead_code)]
pub enum ZoneModel {
    HostManaged,
    HostAware,
    DeviceManaged,
}

/// Enhanced mock drive configuration
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct MockDriveConfig {
    // Basic properties
    pub drive_type: DriveType,
    pub size: u64,
    pub sector_size: u32,
    pub model: String,
    pub serial: String,

    // Capabilities
    pub supports_trim: bool,
    pub supports_secure_erase: bool,
    pub supports_crypto_erase: bool,
    pub freeze_state: FreezeStatus,
    pub sed_type: Option<SEDType>,
    pub sanitize_options: Vec<SanitizeOption>,

    // Temperature simulation
    pub initial_temperature: u32,
    pub max_temperature: u32,
    pub temperature_rise_rate: f32, // °C per GB written

    // Error injection
    pub bad_sector_positions: Vec<u64>, // LBA positions
    pub error_rate: f64,                // Probability of transient error (0.0-1.0)
    pub fail_after_bytes: Option<u64>,  // Simulate catastrophic failure

    // Drive-specific config
    pub smr_config: Option<SMRMockConfig>,
    pub optane_config: Option<OptaneMockConfig>,
    pub hybrid_config: Option<HybridMockConfig>,
    pub emmc_config: Option<EMMCMockConfig>,
    pub nvme_config: Option<NVMeMockConfig>,
    pub raid_config: Option<RAIDMockConfig>,
}

impl Default for MockDriveConfig {
    fn default() -> Self {
        Self {
            drive_type: DriveType::HDD,
            size: 10 * 1024 * 1024, // 10MB for fast tests
            sector_size: 512,
            model: "Mock HDD 10MB".to_string(),
            serial: "MOCK12345678".to_string(),
            supports_trim: false,
            supports_secure_erase: true,
            supports_crypto_erase: false,
            freeze_state: FreezeStatus::NotFrozen,
            sed_type: None,
            sanitize_options: Vec::new(),
            initial_temperature: 35,
            max_temperature: 70,
            temperature_rise_rate: 0.5, // 0.5°C per GB written
            bad_sector_positions: Vec::new(),
            error_rate: 0.0,
            fail_after_bytes: None,
            smr_config: None,
            optane_config: None,
            hybrid_config: None,
            emmc_config: None,
            nvme_config: None,
            raid_config: None,
        }
    }
}

/// SMR-specific configuration
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct SMRMockConfig {
    pub zone_model: ZoneModel,
    pub zone_size: u64, // bytes per zone
    pub conventional_zone_count: u32,
    pub sequential_zone_count: u32,
}

/// Optane-specific configuration
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct OptaneMockConfig {
    pub is_pmem: bool,
    pub supports_ise: bool,
    pub generation: String,
    pub namespace_count: u32,
}

/// Hybrid SSHD configuration
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct HybridMockConfig {
    pub hdd_capacity: u64,
    pub ssd_cache_size: u64,
}

/// eMMC-specific configuration
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct EMMCMockConfig {
    pub boot_partition_size: u64,
    pub rpmb_size: u64,
    pub emmc_version: String,
}

/// NVMe-specific configuration
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct NVMeMockConfig {
    pub namespace_count: u32,
    pub supports_sanitize: bool,
    pub sanitize_crypto_erase: bool,
    pub sanitize_block_erase: bool,
    pub supports_format_nvm: bool,
}

/// RAID-specific configuration
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct RAIDMockConfig {
    pub raid_level: String, // "raid0", "raid1", "raid5", etc.
    pub member_count: u32,
    pub array_uuid: String,
}

/// Runtime state of mock drive
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct MockDriveState {
    pub bytes_written: u64,
    pub bytes_read: u64,
    pub write_count: u64,
    pub read_count: u64,
    pub current_temperature: u32,
    pub is_frozen: bool,
    pub error_count: u64,
}

impl Default for MockDriveState {
    fn default() -> Self {
        Self {
            bytes_written: 0,
            bytes_read: 0,
            write_count: 0,
            read_count: 0,
            current_temperature: 35,
            is_frozen: false,
            error_count: 0,
        }
    }
}

/// Enhanced Mock Drive with behavioral simulation
pub struct MockDrive {
    pub config: MockDriveConfig,
    pub temp_file: NamedTempFile,
    pub state: Arc<Mutex<MockDriveState>>,
}

impl MockDrive {
    /// Create a new mock drive
    pub fn new(config: MockDriveConfig) -> std::io::Result<Self> {
        // Create backing file
        let mut temp_file = NamedTempFile::new()?;

        // Pre-fill with initial pattern (0xAB to simulate used drive)
        let mut written = 0u64;
        let chunk_size = 1024 * 1024; // 1MB chunks
        while written < config.size {
            let remaining = config.size - written;
            let write_size = remaining.min(chunk_size);
            let chunk = vec![0xAB; write_size as usize];
            temp_file.write_all(&chunk)?;
            written += write_size;
        }
        temp_file.flush()?;
        temp_file.seek(SeekFrom::Start(0))?;

        // Initialize state
        let state = Arc::new(Mutex::new(MockDriveState {
            current_temperature: config.initial_temperature,
            is_frozen: matches!(
                config.freeze_state,
                FreezeStatus::Frozen | FreezeStatus::FrozenByBIOS | FreezeStatus::SecurityLocked
            ),
            ..Default::default()
        }));

        Ok(Self {
            config,
            temp_file,
            state,
        })
    }

    /// Get path to the mock drive file (for use with WipeOrchestrator)
    pub fn path(&self) -> &std::path::Path {
        self.temp_file.path()
    }

    /// Get path as string
    pub fn path_str(&self) -> &str {
        self.path().to_str().unwrap()
    }

    /// Generate DriveInfo for this mock drive
    pub fn to_drive_info(&self) -> DriveInfo {
        DriveInfo {
            device_path: self.path_str().to_string(),
            model: self.config.model.clone(),
            serial: self.config.serial.clone(),
            size: self.config.size,
            drive_type: self.config.drive_type.clone(),
            encryption_status: EncryptionStatus::None,
            capabilities: DriveCapabilities {
                secure_erase: self.config.supports_secure_erase,
                enhanced_erase: false,
                crypto_erase: self.config.supports_crypto_erase,
                trim_support: self.config.supports_trim,
                hpa_enabled: false,
                dco_enabled: false,
                sed_type: self.config.sed_type.clone(),
                sanitize_options: self.config.sanitize_options.clone(),
                max_temperature: Some(self.config.max_temperature),
                is_frozen: matches!(
                    self.config.freeze_state,
                    FreezeStatus::Frozen
                        | FreezeStatus::FrozenByBIOS
                        | FreezeStatus::SecurityLocked
                ),
                freeze_status: self.config.freeze_state,
            },
            health_status: Some(HealthStatus::Good),
            temperature_celsius: Some(self.config.initial_temperature),
        }
    }

    /// Get current statistics
    pub fn stats(&self) -> MockDriveStats {
        let state = self.state.lock().unwrap();
        MockDriveStats {
            bytes_written: state.bytes_written,
            bytes_read: state.bytes_read,
            write_count: state.write_count,
            read_count: state.read_count,
            current_temperature: state.current_temperature,
            error_count: state.error_count,
        }
    }

    /// Update statistics after wipe (estimate based on file size)
    pub fn update_stats_post_wipe(&self, passes: u32) -> std::io::Result<()> {
        let mut state = self.state.lock().unwrap();

        // Get actual file size
        let metadata = std::fs::metadata(self.path())?;
        let file_size = metadata.len();

        // Estimate writes based on passes
        state.bytes_written = file_size * passes as u64;
        state.write_count = passes as u64;

        // Update temperature based on writes
        let gb_written = state.bytes_written as f32 / (1024.0 * 1024.0 * 1024.0);
        let temp_increase = (gb_written * self.config.temperature_rise_rate) as u32;
        state.current_temperature =
            (self.config.initial_temperature + temp_increase).min(self.config.max_temperature);

        Ok(())
    }

    /// Verify wipe completion
    pub fn verify_wipe(
        &self,
        expected_pattern: Option<&[u8]>,
    ) -> std::io::Result<VerificationResult> {
        let mut file = std::fs::File::open(self.path())?;
        let mut buffer = vec![0u8; 4096];
        let mut total_checked = 0u64;
        let mut mismatches = 0u64;

        loop {
            let bytes_read = file.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }

            for &byte in &buffer[..bytes_read] {
                if let Some(pattern) = expected_pattern {
                    let expected = pattern[total_checked as usize % pattern.len()];
                    if byte != expected {
                        mismatches += 1;
                    }
                } else {
                    // Check if wiped (not original pattern)
                    if byte == 0xAB {
                        // Original pattern
                        mismatches += 1;
                    }
                }
                total_checked += 1;
            }
        }

        Ok(VerificationResult {
            total_bytes: total_checked,
            mismatches,
            success_rate: if total_checked > 0 {
                1.0 - (mismatches as f64 / total_checked as f64)
            } else {
                0.0
            },
        })
    }
}

/// Statistics from mock drive
#[derive(Debug, Clone)]
pub struct MockDriveStats {
    pub bytes_written: u64,
    pub bytes_read: u64,
    pub write_count: u64,
    #[allow(dead_code)]
    pub read_count: u64,
    pub current_temperature: u32,
    pub error_count: u64,
}

/// Wipe verification result
#[derive(Debug, Clone)]
pub struct VerificationResult {
    pub total_bytes: u64,
    pub mismatches: u64,
    pub success_rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_mock_drive() -> std::io::Result<()> {
        let config = MockDriveConfig::default();
        let mock = MockDrive::new(config)?;

        assert!(mock.path().exists());
        assert_eq!(mock.config.size, 10 * 1024 * 1024);

        Ok(())
    }

    #[test]
    fn test_to_drive_info() -> std::io::Result<()> {
        let config = MockDriveConfig {
            drive_type: DriveType::NVMe,
            model: "Test NVMe".to_string(),
            serial: "TEST123".to_string(),
            size: 256 * 1024 * 1024,
            ..Default::default()
        };
        let mock = MockDrive::new(config)?;
        let drive_info = mock.to_drive_info();

        assert_eq!(drive_info.drive_type, DriveType::NVMe);
        assert_eq!(drive_info.model, "Test NVMe");
        assert_eq!(drive_info.serial, "TEST123");
        assert_eq!(drive_info.size, 256 * 1024 * 1024);

        Ok(())
    }

    #[test]
    fn test_stats_tracking() -> std::io::Result<()> {
        let config = MockDriveConfig::default();
        let mock = MockDrive::new(config)?;

        let stats = mock.stats();
        assert_eq!(stats.bytes_written, 0);
        assert_eq!(stats.error_count, 0);
        assert_eq!(stats.current_temperature, 35);

        // Update after simulated wipe
        mock.update_stats_post_wipe(3)?;
        let stats_after = mock.stats();
        assert!(stats_after.bytes_written > 0);

        Ok(())
    }
}
