use crate::{DriveError, DriveResult, DriveType};
use std::process::Command;

pub struct TrimOperations;

impl TrimOperations {
    /// Check if device supports TRIM
    pub fn supports_trim(device_path: &str) -> DriveResult<bool> {
        // Check for SSD/NVMe
        let drive_type = Self::get_drive_type(device_path)?;

        match drive_type {
            DriveType::SSD => Self::check_ata_trim_support(device_path),
            DriveType::NVMe => Ok(true), // NVMe always supports deallocate
            _ => Ok(false),
        }
    }

    /// Perform full-device TRIM
    pub fn trim_entire_device(device_path: &str) -> DriveResult<()> {
        println!("Starting full-device TRIM on {}...", device_path);

        if !Self::supports_trim(device_path)? {
            return Err(DriveError::TRIMFailed(
                "Device does not support TRIM".to_string(),
            ));
        }

        let drive_type = Self::get_drive_type(device_path)?;

        match drive_type {
            DriveType::SSD => Self::trim_ssd_device(device_path),
            DriveType::NVMe => Self::trim_nvme_device(device_path),
            _ => Err(DriveError::TRIMFailed(
                "TRIM not supported for this drive type".to_string(),
            )),
        }
    }

    /// TRIM an SSD using blkdiscard or hdparm
    fn trim_ssd_device(device_path: &str) -> DriveResult<()> {
        // First try blkdiscard (most reliable)
        if Self::trim_via_blkdiscard(device_path).is_ok() {
            println!("TRIM completed successfully via blkdiscard");
            return Ok(());
        }

        // Fallback to hdparm TRIM
        if Self::trim_via_hdparm(device_path).is_ok() {
            println!("TRIM completed successfully via hdparm");
            return Ok(());
        }

        // Last resort: manual TRIM via ioctl
        Self::trim_via_ioctl()
    }

    /// TRIM an NVMe device using nvme-cli
    fn trim_nvme_device(device_path: &str) -> DriveResult<()> {
        // Use nvme deallocate (similar to TRIM)
        println!("Performing NVMe deallocate operation...");

        // First try blkdiscard (works for NVMe too)
        if Self::trim_via_blkdiscard(device_path).is_ok() {
            println!("NVMe deallocate completed successfully");
            return Ok(());
        }

        // Fallback to nvme format with deallocate
        Self::nvme_deallocate(device_path)
    }

    /// TRIM using blkdiscard utility
    fn trim_via_blkdiscard(device_path: &str) -> DriveResult<()> {
        println!("Attempting TRIM via blkdiscard...");

        let output = Command::new("blkdiscard")
            .args(["-v", device_path])
            .output()
            .map_err(|e| DriveError::TRIMFailed(format!("blkdiscard failed: {}", e)))?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(DriveError::TRIMFailed(format!(
                "blkdiscard failed: {}",
                error
            )));
        }

        Ok(())
    }

    /// TRIM using hdparm
    fn trim_via_hdparm(device_path: &str) -> DriveResult<()> {
        println!("Attempting TRIM via hdparm...");

        // Get device size
        let size = Self::get_device_size(device_path)?;
        let sectors = size / 512;

        // Create TRIM command
        // hdparm --trim-sector-ranges START:COUNT
        let output = Command::new("hdparm")
            .args([
                "--trim-sector-ranges",
                &format!("0:{}", sectors),
                device_path,
            ])
            .output()
            .map_err(|e| DriveError::TRIMFailed(format!("hdparm TRIM failed: {}", e)))?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(DriveError::TRIMFailed(format!(
                "hdparm TRIM failed: {}",
                error
            )));
        }

        Ok(())
    }

    /// TRIM using direct ioctl calls
    fn trim_via_ioctl() -> DriveResult<()> {
        println!("Attempting TRIM via ioctl...");

        // This would require unsafe Rust and libc bindings
        // For now, return an error
        Err(DriveError::TRIMFailed(
            "Direct ioctl TRIM not implemented".to_string(),
        ))
    }

    /// NVMe deallocate operation
    fn nvme_deallocate(device_path: &str) -> DriveResult<()> {
        println!("Performing NVMe deallocate...");

        // Get namespace ID
        let nsid = Self::get_nvme_nsid(device_path)?;

        // Create deallocate command
        let output = Command::new("nvme")
            .args([
                "dsm",
                device_path,
                "-n",
                &nsid,
                "-d",
                "-a",
                "0",
                "-b",
                "0",
                "-s",
                "1",
            ])
            .output()
            .map_err(|e| DriveError::TRIMFailed(format!("NVMe deallocate failed: {}", e)))?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(DriveError::TRIMFailed(format!(
                "NVMe deallocate failed: {}",
                error
            )));
        }

        Ok(())
    }

    /// Verify TRIM effectiveness by checking for zeroes
    pub fn verify_trim_effectiveness(device_path: &str, sample_size: usize) -> DriveResult<bool> {
        println!("Verifying TRIM effectiveness...");

        use crate::io::{IOConfig, OptimizedIO};
        let config = IOConfig::small_read_optimized();
        let mut handle = OptimizedIO::open(device_path, config)
            .map_err(|e| DriveError::IoError(std::io::Error::other(e.to_string())))?;

        // Sample random locations
        let device_size = Self::get_device_size(device_path)?;
        let mut zero_count = 0;
        let mut total_checked = 0;

        use rand::Rng;
        let mut rng = rand::thread_rng();

        for _ in 0..sample_size {
            let offset = rng.gen_range(0..device_size - 4096);

            let buffer =
                OptimizedIO::read_range(&mut handle, offset, 4096).unwrap_or_else(|_| vec![]);

            if !buffer.is_empty() {
                // Check if buffer is all zeros or pattern indicating TRIM
                if buffer.iter().all(|&b| b == 0)
                    || buffer.iter().all(|&b| b == 0xFF)
                    || Self::is_trim_pattern(&buffer)
                {
                    zero_count += 1;
                }
                total_checked += 1;
            }
        }

        // If more than 90% of samples show TRIM patterns, consider it effective
        let effectiveness = if total_checked > 0 {
            (zero_count as f64 / total_checked as f64) > 0.9
        } else {
            false
        };

        println!(
            "TRIM verification: {}/{} samples showed TRIM patterns",
            zero_count, total_checked
        );

        Ok(effectiveness)
    }

    /// Check if buffer contains common TRIM patterns
    pub(crate) fn is_trim_pattern(buffer: &[u8]) -> bool {
        // Check for common TRIM patterns
        // Some SSDs return specific patterns after TRIM

        // Pattern 1: All same byte
        let first = buffer[0];
        if buffer.iter().all(|&b| b == first) {
            return true;
        }

        // Pattern 2: Repeating pattern (like DEAD BEEF)
        if buffer.len() >= 8 {
            let pattern = &buffer[0..4];
            let mut matches = true;
            for chunk in buffer.chunks(4) {
                if chunk.len() == 4 && chunk != pattern {
                    matches = false;
                    break;
                }
            }
            if matches {
                return true;
            }
        }

        false
    }

    /// Check ATA TRIM support
    fn check_ata_trim_support(device_path: &str) -> DriveResult<bool> {
        let output = Command::new("hdparm")
            .args(["-I", device_path])
            .output()
            .map_err(|e| {
                DriveError::HardwareCommandFailed(format!("Failed to check TRIM support: {}", e))
            })?;

        let output_str = String::from_utf8_lossy(&output.stdout);

        // Look for TRIM support indicators
        Ok(output_str.contains("Data Set Management TRIM supported")
            || output_str.contains("TRIM supported")
            || output_str.contains("Deterministic read data after TRIM"))
    }

    /// Get device size in bytes
    fn get_device_size(device_path: &str) -> DriveResult<u64> {
        let output = Command::new("blockdev")
            .args(["--getsize64", device_path])
            .output()
            .map_err(|e| {
                DriveError::HardwareCommandFailed(format!("Failed to get device size: {}", e))
            })?;

        let size_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        size_str.parse::<u64>().map_err(|e| {
            DriveError::HardwareCommandFailed(format!("Failed to parse device size: {}", e))
        })
    }

    /// Get drive type
    fn get_drive_type(device_path: &str) -> DriveResult<DriveType> {
        if device_path.contains("nvme") {
            return Ok(DriveType::NVMe);
        }

        // Check rotation rate to distinguish SSD from HDD
        let output = Command::new("smartctl")
            .args(["-i", device_path])
            .output()
            .map_err(|e| {
                DriveError::HardwareCommandFailed(format!("Failed to get drive type: {}", e))
            })?;

        let output_str = String::from_utf8_lossy(&output.stdout);

        if output_str.contains("Solid State Device") || output_str.contains("0 rpm") {
            Ok(DriveType::SSD)
        } else if output_str.contains("rpm") {
            Ok(DriveType::HDD)
        } else {
            Ok(DriveType::Unknown)
        }
    }

    /// Get NVMe namespace ID
    pub(crate) fn get_nvme_nsid(device_path: &str) -> DriveResult<String> {
        // Extract namespace from path like /dev/nvme0n1
        if let Some(n_pos) = device_path.rfind('n') {
            let after_n = &device_path[n_pos + 1..];
            if let Ok(nsid) = after_n.parse::<u32>() {
                return Ok(nsid.to_string());
            }
        }

        Ok("1".to_string()) // Default to namespace 1
    }

    /// Perform secure TRIM with verification
    pub fn secure_trim_with_verify(device_path: &str) -> DriveResult<()> {
        println!("Performing secure TRIM with verification...");

        // Step 1: Initial TRIM
        Self::trim_entire_device(device_path)?;

        // Step 2: Verify effectiveness
        if !Self::verify_trim_effectiveness(device_path, 100)? {
            println!("Warning: TRIM may not be fully effective on this device");
        }

        // Step 3: Multiple TRIM passes for security
        println!("Performing additional TRIM passes for security...");
        for pass in 1..=3 {
            println!("TRIM pass {}/3", pass);
            Self::trim_entire_device(device_path)?;
        }

        println!("Secure TRIM completed");
        Ok(())
    }
}
