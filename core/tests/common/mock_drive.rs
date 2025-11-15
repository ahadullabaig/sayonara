// Allow uppercase acronyms in test code (HDD, SSD, SMR, EMMC)
#![allow(clippy::upper_case_acronyms)]

use std::io::{Seek, SeekFrom, Write};
/// Mock drive infrastructure for testing
///
/// Provides simulated drives for testing wipe operations without requiring
/// actual hardware. Supports various drive types and error injection.
use tempfile::{NamedTempFile, TempDir};

/// Simulated drive types for testing
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)]
pub enum MockDriveType {
    HDD,
    SSD,
    NVMe,
    SMR,
    Optane,
    Hybrid,
    EMMC,
}

/// Mock drive configuration
#[allow(dead_code)]
pub struct MockDriveConfig {
    pub drive_type: MockDriveType,
    pub size_mb: u64,
    pub sector_size: u32,
    pub simulate_errors: bool,
    pub freeze_state: bool,
}

impl Default for MockDriveConfig {
    fn default() -> Self {
        Self {
            drive_type: MockDriveType::HDD,
            size_mb: 100, // 100MB default
            sector_size: 512,
            simulate_errors: false,
            freeze_state: false,
        }
    }
}

/// Mock drive instance
pub struct MockDrive {
    pub config: MockDriveConfig,
    pub temp_file: NamedTempFile,
    pub _temp_dir: Option<TempDir>,
}

impl MockDrive {
    /// Create a new mock drive with the specified configuration
    pub fn new(config: MockDriveConfig) -> std::io::Result<Self> {
        let mut temp_file = NamedTempFile::new()?;

        // Initialize drive with random data to simulate used drive
        let size_bytes = config.size_mb * 1024 * 1024;
        let mut written = 0u64;
        let chunk_size = 1024 * 1024; // 1MB chunks

        while written < size_bytes {
            let remaining = size_bytes - written;
            let write_size = remaining.min(chunk_size);
            let chunk = vec![0xAB; write_size as usize]; // Pattern to simulate data
            temp_file.write_all(&chunk)?;
            written += write_size;
        }

        temp_file.flush()?;
        temp_file.seek(SeekFrom::Start(0))?;

        Ok(Self {
            config,
            temp_file,
            _temp_dir: None,
        })
    }

    /// Create a mock HDD
    pub fn create_hdd(size_mb: u64) -> std::io::Result<Self> {
        let config = MockDriveConfig {
            drive_type: MockDriveType::HDD,
            size_mb,
            ..Default::default()
        };
        Self::new(config)
    }

    /// Create a mock SSD
    pub fn create_ssd(size_mb: u64) -> std::io::Result<Self> {
        let config = MockDriveConfig {
            drive_type: MockDriveType::SSD,
            size_mb,
            ..Default::default()
        };
        Self::new(config)
    }

    /// Create a mock NVMe drive
    pub fn create_nvme(size_mb: u64) -> std::io::Result<Self> {
        let config = MockDriveConfig {
            drive_type: MockDriveType::NVMe,
            size_mb,
            ..Default::default()
        };
        Self::new(config)
    }

    /// Get the path to the mock drive file
    pub fn path(&self) -> &std::path::Path {
        self.temp_file.path()
    }

    /// Get the path as a string
    #[allow(dead_code)]
    pub fn path_str(&self) -> &str {
        self.path().to_str().unwrap()
    }

    /// Get the size in bytes
    pub fn size_bytes(&self) -> u64 {
        self.config.size_mb * 1024 * 1024
    }
}

#[cfg(target_os = "linux")]
pub mod loopback {
    //! Linux loopback device utilities for more realistic testing
    //!
    //! These require root privileges and are only enabled with the
    //! `integration-tests` feature flag.

    use std::process::Command;

    /// Create a loopback device from a file
    ///
    /// Requires root privileges. Use `sudo` when running tests.
    #[allow(dead_code)]
    pub fn create_loopback(file_path: &str, size_mb: u64) -> std::io::Result<String> {
        // Create a sparse file
        let output = Command::new("truncate")
            .args(["-s", &format!("{}M", size_mb), file_path])
            .output()?;

        if !output.status.success() {
            return Err(std::io::Error::other(
                format!(
                    "Failed to create sparse file: {}",
                    String::from_utf8_lossy(&output.stderr)
                ),
            ));
        }

        // Attach to loop device
        let output = Command::new("losetup")
            .args(["-f", "--show", file_path])
            .output()?;

        if !output.status.success() {
            return Err(std::io::Error::other(
                format!(
                    "Failed to create loop device: {}",
                    String::from_utf8_lossy(&output.stderr)
                ),
            ));
        }

        let loop_device = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(loop_device)
    }

    /// Detach a loopback device
    #[allow(dead_code)]
    pub fn detach_loopback(loop_device: &str) -> std::io::Result<()> {
        let output = Command::new("losetup").args(["-d", loop_device]).output()?;

        if !output.status.success() {
            return Err(std::io::Error::other(
                format!(
                    "Failed to detach loop device: {}",
                    String::from_utf8_lossy(&output.stderr)
                ),
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_mock_hdd() {
        let mock = MockDrive::create_hdd(10).unwrap();
        assert_eq!(mock.config.drive_type, MockDriveType::HDD);
        assert_eq!(mock.size_bytes(), 10 * 1024 * 1024);
        assert!(mock.path().exists());
    }

    #[test]
    fn test_create_mock_ssd() {
        let mock = MockDrive::create_ssd(10).unwrap();
        assert_eq!(mock.config.drive_type, MockDriveType::SSD);
        assert!(mock.path().exists());
    }

    #[test]
    fn test_create_mock_nvme() {
        let mock = MockDrive::create_nvme(10).unwrap();
        assert_eq!(mock.config.drive_type, MockDriveType::NVMe);
        assert!(mock.path().exists());
    }
}
