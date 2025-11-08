use crate::{
    DriveInfo, DriveType, EncryptionStatus, DriveCapabilities,
    FreezeStatus, SEDType, SanitizeOption
};
use anyhow::Result;
use std::process::Command;
use std::fs;
use std::path::Path;

// Import submodules for capability detection
use super::freeze::FreezeMitigation;
use super::operations::hpa_dco::HPADCOManager;
use super::operations::sed::SEDManager;
use super::operations::trim::TrimOperations;
use super::operations::smart::SMARTMonitor;

pub struct DriveDetector;

impl DriveDetector {
    /// Comprehensive drive detection with all capability checks
    pub fn detect_all_drives() -> Result<Vec<DriveInfo>> {
        let mut drives = Vec::new();

        // Scan /sys/block for block devices
        let block_devices = fs::read_dir("/sys/block")?;

        for entry in block_devices {
            let entry = entry?;
            let device_name = entry.file_name();
            let device_name = device_name.to_string_lossy();

            // Skip non-physical devices
            if Self::should_skip_device(&device_name) {
                continue;
            }

            let device_path = format!("/dev/{}", device_name);

            // Check if device is accessible
            if !Path::new(&device_path).exists() {
                continue;
            }

            // Comprehensive analysis with error recovery
            match Self::analyze_drive_comprehensive(&device_path) {
                Ok(drive_info) => drives.push(drive_info),
                Err(e) => {
                    eprintln!("Warning: Failed to analyze {}: {}", device_path, e);
                    // Try basic detection as fallback
                    if let Ok(basic_info) = Self::analyze_drive_basic(&device_path) {
                        drives.push(basic_info);
                    }
                }
            }
        }

        Ok(drives)
    }

    /// Check if device should be skipped
    pub(crate) fn should_skip_device(device_name: &str) -> bool {
        // Skip loop devices, ram disks, device mapper, etc.
        device_name.starts_with("loop") ||
            device_name.starts_with("ram") ||
            device_name.starts_with("dm-") ||
            device_name.starts_with("sr") ||    // CD/DVD drives
            device_name.starts_with("zram")
    }

    /// Comprehensive drive analysis with all capabilities
    fn analyze_drive_comprehensive(device_path: &str) -> Result<DriveInfo> {
        // Get basic information first
        let mut drive_info = Self::analyze_drive_basic(device_path)?;

        // Detect all capabilities (non-destructive operations only)
        let mut capabilities = DriveCapabilities::default();

        // Check freeze status
        if let Ok(freeze_status) = FreezeMitigation::get_freeze_status(device_path) {
            capabilities.freeze_status = freeze_status;
            capabilities.is_frozen = matches!(
                freeze_status,
                FreezeStatus::Frozen | FreezeStatus::FrozenByBIOS
            );
        }

        // Check for HPA/DCO (only for ATA/SATA drives - HDD and SSD)
        // NVMe, USB, RAID drives don't support HPA/DCO
        if matches!(drive_info.drive_type, DriveType::HDD | DriveType::SSD) {
            if let Ok((hpa, dco)) = HPADCOManager::check_hidden_areas(device_path) {
                capabilities.hpa_enabled = hpa.is_some();
                capabilities.dco_enabled = dco.is_some();
            }
        }

        // Check SED capabilities
        if let Ok(sed_info) = SEDManager::detect_sed(device_path) {
            capabilities.sed_type = Some(sed_info.sed_type.clone());
            capabilities.crypto_erase = sed_info.supports_crypto_erase;
        }

        // Check TRIM support
        if let Ok(trim_supported) = TrimOperations::supports_trim(device_path) {
            capabilities.trim_support = trim_supported;
        }

        // Check secure erase support
        capabilities.secure_erase = Self::check_secure_erase_support(device_path)?;
        capabilities.enhanced_erase = Self::check_enhanced_erase_support(device_path)?;

        // Check NVMe sanitize options
        if drive_info.drive_type == DriveType::NVMe {
            capabilities.sanitize_options = Self::get_nvme_sanitize_options(device_path)?;
        }

        // Get SMART health and temperature
        if let Ok(health) = SMARTMonitor::get_health(device_path) {
            drive_info.health_status = Some(health.overall_health);
            drive_info.temperature_celsius = health.temperature_celsius;
            capabilities.max_temperature = Some(70); // Default safe max
        }

        drive_info.capabilities = capabilities;

        Ok(drive_info)
    }

    /// Basic drive analysis (fallback)
    fn analyze_drive_basic(device_path: &str) -> Result<DriveInfo> {
        let smartctl_output = Command::new("smartctl")
            .args(["-i", device_path])
            .output()?;

        let output_str = String::from_utf8_lossy(&smartctl_output.stdout);

        let model = Self::extract_field(&output_str, "Device Model:")
            .or_else(|| Self::extract_field(&output_str, "Model Number:"))
            .unwrap_or_else(|| "Unknown".to_string());

        let serial = Self::extract_field(&output_str, "Serial Number:")
            .unwrap_or_else(|| "Unknown".to_string());

        let size = Self::get_drive_size(device_path)?;
        let drive_type = Self::determine_drive_type(device_path, &output_str)?;
        let encryption_status = Self::detect_encryption(device_path)?;

        Ok(DriveInfo {
            device_path: device_path.to_string(),
            model,
            serial,
            size,
            drive_type,
            encryption_status,
            capabilities: DriveCapabilities::default(),
            health_status: None,
            temperature_celsius: None,
        })
    }

    /// Extract field from smartctl output
    pub(crate) fn extract_field(output: &str, field_name: &str) -> Option<String> {
        output
            .lines()
            .find(|line| line.contains(field_name))?
            .split(':')
            .nth(1)?
            .trim()
            .to_string()
            .into()
    }

    /// Get drive size in bytes
    fn get_drive_size(device_path: &str) -> Result<u64> {
        let output = Command::new("blockdev")
            .args(["--getsize64", device_path])
            .output()?;

        let size_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(size_str.parse()?)
    }

    /// Determine drive type from various indicators
    pub(crate) fn determine_drive_type(device_path: &str, smartctl_output: &str) -> Result<DriveType> {
        // Check for NVMe
        if device_path.contains("nvme") {
            return Ok(DriveType::NVMe);
        }

        // Check for USB
        if Self::is_usb_device(device_path)? {
            return Ok(DriveType::USB);
        }

        // Check for RAID member
        if Self::is_raid_member(device_path)? {
            return Ok(DriveType::RAID);
        }

        // Check SMART output for rotation rate
        if smartctl_output.contains("Rotation Rate:") {
            if smartctl_output.contains("Solid State Device") ||
                smartctl_output.contains("0 rpm") {
                Ok(DriveType::SSD)
            } else {
                Ok(DriveType::HDD)
            }
        } else if smartctl_output.contains("SSD") ||
            smartctl_output.contains("Solid State") {
            Ok(DriveType::SSD)
        } else if smartctl_output.contains("rpm") {
            Ok(DriveType::HDD)
        } else {
            Ok(DriveType::Unknown)
        }
    }

    /// Check if device is connected via USB
    fn is_usb_device(device_path: &str) -> Result<bool> {
        let device_name = Path::new(device_path)
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| anyhow::anyhow!("Invalid device path"))?;

        let sys_path = format!("/sys/block/{}/device", device_name);
        if let Ok(real_path) = fs::read_link(&sys_path) {
            let path_str = real_path.to_string_lossy();
            return Ok(path_str.contains("usb"));
        }

        Ok(false)
    }

    /// Check if device is a RAID member
    fn is_raid_member(device_path: &str) -> Result<bool> {
        // Check for MD RAID
        let mdadm_output = Command::new("mdadm")
            .args(["--examine", device_path])
            .output();

        if let Ok(output) = mdadm_output {
            if output.status.success() {
                let output_str = String::from_utf8_lossy(&output.stdout);
                if output_str.contains("Raid Level") ||
                    output_str.contains("Array UUID") {
                    return Ok(true);
                }
            }
        }

        // Check for hardware RAID via sg_inq
        let sg_output = Command::new("sg_inq")
            .args([device_path])
            .output();

        if let Ok(output) = sg_output {
            let output_str = String::from_utf8_lossy(&output.stdout);
            if output_str.contains("RAID") {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Detect encryption status
    fn detect_encryption(device_path: &str) -> Result<EncryptionStatus> {
        // Check for OPAL (handled by SEDManager)
        if let Ok(sed_info) = SEDManager::detect_sed(device_path) {
            if sed_info.sed_type != SEDType::None {
                return Ok(EncryptionStatus::OPAL);
            }
        }

        // Check for LUKS
        let luks_check = Command::new("cryptsetup")
            .args(["isLuks", device_path])
            .output();

        if let Ok(output) = luks_check {
            if output.status.success() {
                return Ok(EncryptionStatus::LUKS);
            }
        }

        // Check for BitLocker
        if Self::check_bitlocker(device_path)? {
            return Ok(EncryptionStatus::BitLocker);
        }

        // Check for FileVault (macOS)
        if Self::check_filevault(device_path)? {
            return Ok(EncryptionStatus::FileVault);
        }

        // Check for VeraCrypt
        if Self::check_veracrypt(device_path)? {
            return Ok(EncryptionStatus::VeraCrypt);
        }

        Ok(EncryptionStatus::None)
    }

    /// Check for BitLocker encryption
    fn check_bitlocker(device_path: &str) -> Result<bool> {
        // Read first sectors to check for BitLocker signature
        use crate::io::{OptimizedIO, IOConfig};

        let config = IOConfig::small_read_optimized();
        let mut handle = OptimizedIO::open(device_path, config)?;
        let buffer = OptimizedIO::read_range(&mut handle, 0, 512)?;

        // BitLocker signature "-FVE-FS-"
        let signature = b"-FVE-FS-";
        Ok(buffer.windows(signature.len()).any(|w| w == signature))
    }

    /// Check for FileVault encryption
    fn check_filevault(device_path: &str) -> Result<bool> {
        // Check for Core Storage or APFS encryption
        use crate::io::{OptimizedIO, IOConfig};

        let config = IOConfig::small_read_optimized();
        let mut handle = OptimizedIO::open(device_path, config)?;
        let buffer = OptimizedIO::read_range(&mut handle, 0, 4096)?;

        // Check for Core Storage or APFS signatures
        Ok(buffer.windows(8).any(|w| {
            w == b"CS\x00\x00\x00\x00\x00\x00" || // Core Storage
                w == b"NXSB\x00\x00\x00\x00"          // APFS
        }))
    }

    /// Check for VeraCrypt encryption
    fn check_veracrypt(device_path: &str) -> Result<bool> {
        // VeraCrypt doesn't have a clear signature, but we can check for high entropy
        use crate::io::{OptimizedIO, IOConfig};

        let config = IOConfig::small_read_optimized();
        let mut handle = OptimizedIO::open(device_path, config)?;
        let buffer = OptimizedIO::read_range(&mut handle, 0, 65536)?; // Read 64KB

        // Calculate entropy
        let entropy = Self::calculate_entropy(&buffer);

        // High entropy (>7.5) in first sectors suggests encryption
        Ok(entropy > 7.5)
    }

    /// Calculate Shannon entropy
    pub(crate) fn calculate_entropy(data: &[u8]) -> f64 {
        let mut counts = [0u64; 256];
        for &byte in data {
            counts[byte as usize] += 1;
        }

        let length = data.len() as f64;
        let mut entropy = 0.0;

        for &count in &counts {
            if count > 0 {
                let probability = count as f64 / length;
                entropy -= probability * probability.log2();
            }
        }

        entropy
    }

    /// Check secure erase support
    fn check_secure_erase_support(device_path: &str) -> Result<bool> {
        let output = Command::new("hdparm")
            .args(["-I", device_path])
            .output()?;

        let output_str = String::from_utf8_lossy(&output.stdout);
        Ok(output_str.contains("supported: enhanced erase") ||
            output_str.contains("SECURITY ERASE UNIT"))
    }

    /// Check enhanced secure erase support
    fn check_enhanced_erase_support(device_path: &str) -> Result<bool> {
        let output = Command::new("hdparm")
            .args(["-I", device_path])
            .output()?;

        let output_str = String::from_utf8_lossy(&output.stdout);
        Ok(output_str.contains("enhanced erase"))
    }

    /// Get NVMe sanitize options
    fn get_nvme_sanitize_options(device_path: &str) -> Result<Vec<SanitizeOption>> {
        let mut options = Vec::new();

        let output = Command::new("nvme")
            .args(["id-ctrl", device_path])
            .output()?;

        let output_str = String::from_utf8_lossy(&output.stdout);

        // Parse sanitize capabilities
        if output_str.contains("Crypto Erase Supported") {
            options.push(SanitizeOption::CryptoErase);
        }
        if output_str.contains("Block Erase Supported") {
            options.push(SanitizeOption::BlockErase);
        }
        if output_str.contains("Overwrite Supported") {
            options.push(SanitizeOption::Overwrite);
        }
        if output_str.contains("Crypto Scramble Supported") {
            options.push(SanitizeOption::CryptoScramble);
        }

        // If no specific info, assume basic support
        if options.is_empty() && device_path.contains("nvme") {
            options.push(SanitizeOption::CryptoErase);
            options.push(SanitizeOption::BlockErase);
        }

        Ok(options)
    }

    /// Check if drive is system drive
    pub fn is_system_drive(device_path: &str) -> Result<bool> {
        // Check if root filesystem is on this device
        let mounts = fs::read_to_string("/proc/mounts")?;

        for line in mounts.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                if parts[1] == "/" && parts[0].starts_with(device_path) {
                    return Ok(true);
                }
            }
        }

        // Check if boot partition is on this device
        if let Ok(cmdline) = fs::read_to_string("/proc/cmdline") {
            if cmdline.contains(device_path) {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Check if drive is currently mounted
    pub fn is_mounted(device_path: &str) -> Result<bool> {
        let mounts = fs::read_to_string("/proc/mounts")?;

        for line in mounts.lines() {
            if line.starts_with(device_path) {
                return Ok(true);
            }
        }

        Ok(false)
    }
}
