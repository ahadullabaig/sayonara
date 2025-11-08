use crate::{DriveError, DriveResult};
use std::process::Command;
use std::str;

#[derive(Debug, Clone)]
pub struct HPAInfo {
    pub enabled: bool,
    pub native_max_sectors: u64,
    pub current_max_sectors: u64,
    pub hidden_sectors: u64,
    pub hidden_size_bytes: u64,
}

#[derive(Debug, Clone)]
pub struct DCOInfo {
    pub enabled: bool,
    pub real_max_sectors: u64,
    pub dco_max_sectors: u64,
    pub hidden_sectors: u64,
    pub hidden_size_bytes: u64,
}

pub struct HPADCOManager;

impl HPADCOManager {
    /// Detect and return HPA information
    pub fn detect_hpa(device_path: &str) -> DriveResult<Option<HPAInfo>> {
        println!("Checking for Hidden Protected Area (HPA) on {}...", device_path);

        // Get native max address
        let native_max = Self::get_native_max_address(device_path)?;

        // Get current max address
        let current_max = Self::get_max_address(device_path)?;

        if native_max > current_max {
            let hidden_sectors = native_max - current_max;
            let hidden_bytes = hidden_sectors * 512; // Assuming 512-byte sectors

            println!("HPA detected: {} sectors ({} bytes) hidden",
                     hidden_sectors, hidden_bytes);

            Ok(Some(HPAInfo {
                enabled: true,
                native_max_sectors: native_max,
                current_max_sectors: current_max,
                hidden_sectors,
                hidden_size_bytes: hidden_bytes,
            }))
        } else {
            println!("No HPA detected");
            Ok(None)
        }
    }

    /// Detect and return DCO information
    pub fn detect_dco(device_path: &str) -> DriveResult<Option<DCOInfo>> {
        println!("Checking for Device Configuration Overlay (DCO) on {}...", device_path);

        // Check if DCO is supported and enabled
        let dco_status = Self::get_dco_status(device_path)?;

        if let Some((real_max, dco_max)) = dco_status {
            if real_max > dco_max {
                let hidden_sectors = real_max - dco_max;
                let hidden_bytes = hidden_sectors * 512;

                println!("DCO detected: {} sectors ({} bytes) hidden",
                         hidden_sectors, hidden_bytes);

                return Ok(Some(DCOInfo {
                    enabled: true,
                    real_max_sectors: real_max,
                    dco_max_sectors: dco_max,
                    hidden_sectors,
                    hidden_size_bytes: hidden_bytes,
                }));
            }
        }

        println!("No DCO detected");
        Ok(None)
    }

    /// Temporarily remove HPA (can be restored)
    pub fn remove_hpa_temporary(device_path: &str) -> DriveResult<()> {
        println!("Temporarily removing HPA on {}...", device_path);

        let native_max = Self::get_native_max_address(device_path)?;

        // Use hdparm to set max address to native max
        let output = Command::new("hdparm")
            .args(["--yes-i-know-what-i-am-doing", "-N", &format!("{}", native_max), device_path])
            .output()
            .map_err(|e| DriveError::HardwareCommandFailed(
                format!("Failed to remove HPA: {}", e)
            ))?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(DriveError::HardwareCommandFailed(
                format!("Failed to remove HPA: {}", error)
            ));
        }

        println!("HPA temporarily removed. Full capacity now accessible.");
        Ok(())
    }

    /// Restore HPA to original settings
    pub fn restore_hpa(device_path: &str, original_max_sectors: u64) -> DriveResult<()> {
        println!("Restoring HPA on {} to {} sectors...", device_path, original_max_sectors);

        let output = Command::new("hdparm")
            .args(["--yes-i-know-what-i-am-doing", "-N", &format!("{}", original_max_sectors), device_path])
            .output()
            .map_err(|e| DriveError::HardwareCommandFailed(
                format!("Failed to restore HPA: {}", e)
            ))?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(DriveError::HardwareCommandFailed(
                format!("Failed to restore HPA: {}", error)
            ));
        }

        println!("HPA restored to original configuration");
        Ok(())
    }

    /// Remove DCO (WARNING: This is typically permanent!)
    pub fn remove_dco(device_path: &str) -> DriveResult<()> {
        println!("WARNING: Removing DCO is typically permanent!");
        println!("Attempting to remove DCO on {}...", device_path);

        // DCO removal requires special ATA commands
        // Using hdparm's DCO features if available
        let output = Command::new("hdparm")
            .args(["--dco-restore", device_path])
            .output()
            .map_err(|e| DriveError::HardwareCommandFailed(
                format!("Failed to remove DCO: {}", e)
            ))?;

        if !output.status.success() {
            // Try alternative method with HDIO_DRIVE_CMD
            return Self::remove_dco_via_ata_command(device_path);
        }

        println!("DCO removed successfully");
        Ok(())
    }

    /// Get the native max address (without HPA)
    fn get_native_max_address(device_path: &str) -> DriveResult<u64> {
        let output = Command::new("hdparm")
            .args(["-N", device_path])
            .output()
            .map_err(|e| DriveError::HardwareCommandFailed(
                format!("Failed to get native max address: {}", e)
            ))?;

        let output_str = String::from_utf8_lossy(&output.stdout);

        // Parse output looking for native max
        for line in output_str.lines() {
            if line.contains("native") && line.contains("max") {
                // Extract number from line like "max sectors = 1234567890/1234567890"
                if let Some(pos) = line.rfind('/') {
                    let native_str = &line[pos + 1..];
                    if let Some(num_end) = native_str.find(|c: char| !c.is_numeric()) {
                        let num_str = &native_str[..num_end];
                        if let Ok(native_max) = num_str.parse::<u64>() {
                            return Ok(native_max);
                        }
                    } else if let Ok(native_max) = native_str.trim().parse::<u64>() {
                        return Ok(native_max);
                    }
                }
            }
        }

        // Fallback: try to get via ATA IDENTIFY
        Self::get_native_max_via_identify(device_path)
    }

    /// Get current max address (with HPA if present)
    fn get_max_address(device_path: &str) -> DriveResult<u64> {
        let output = Command::new("hdparm")
            .args(["-N", device_path])
            .output()
            .map_err(|e| DriveError::HardwareCommandFailed(
                format!("Failed to get max address: {}", e)
            ))?;

        let output_str = String::from_utf8_lossy(&output.stdout);

        // Parse output looking for current max
        for line in output_str.lines() {
            if line.contains("max sectors") {
                // Extract number from line like "max sectors = 1234567890/2345678901"
                if let Some(equals_pos) = line.find('=') {
                    let after_equals = &line[equals_pos + 1..].trim();
                    if let Some(slash_pos) = after_equals.find('/') {
                        let current_str = &after_equals[..slash_pos];
                        if let Ok(current_max) = current_str.trim().parse::<u64>() {
                            return Ok(current_max);
                        }
                    }
                }
            }
        }

        // Fallback to blockdev
        Self::get_size_via_blockdev(device_path)
    }

    /// Get DCO status
    fn get_dco_status(device_path: &str) -> DriveResult<Option<(u64, u64)>> {
        // Try hdparm DCO identify
        let output = Command::new("hdparm")
            .args(["--dco-identify", device_path])
            .output();

        if let Ok(output) = output {
            if output.status.success() {
                let output_str = String::from_utf8_lossy(&output.stdout);
                return Self::parse_dco_output(&output_str);
            }
        }

        // Fallback: check via ATA commands
        Ok(None)
    }

    /// Parse DCO output from hdparm
    pub(crate) fn parse_dco_output(output: &str) -> DriveResult<Option<(u64, u64)>> {
        let mut real_max = None;
        let mut dco_max = None;

        for line in output.lines() {
            if line.contains("Real max sectors") {
                if let Some(num) = Self::extract_number_from_line(line) {
                    real_max = Some(num);
                }
            } else if line.contains("DCO max sectors") {
                if let Some(num) = Self::extract_number_from_line(line) {
                    dco_max = Some(num);
                }
            }
        }

        if let (Some(real), Some(dco)) = (real_max, dco_max) {
            Ok(Some((real, dco)))
        } else {
            Ok(None)
        }
    }

    /// Extract number from a line of text
    pub(crate) fn extract_number_from_line(line: &str) -> Option<u64> {
        // Find all numeric sequences in the line
        let parts: Vec<&str> = line.split(|c: char| !c.is_numeric())
            .filter(|s| !s.is_empty())
            .collect();

        // Return the largest number (likely the sector count)
        parts.iter()
            .filter_map(|s| s.parse::<u64>().ok())
            .max()
    }

    /// Remove DCO via direct ATA command
    fn remove_dco_via_ata_command(device_path: &str) -> DriveResult<()> {
        // This would require low-level ATA commands
        // Feature 0xC6 (DCO RESTORE)

        println!("Attempting DCO removal via ATA command...");

        // Try using smartctl to send ATA command
        let output = Command::new("smartctl")
            .args(["-s", "dco,restore", device_path])
            .output();

        if let Ok(output) = output {
            if output.status.success() {
                println!("DCO removed via smartctl");
                return Ok(());
            }
        }

        Err(DriveError::HardwareCommandFailed(
            "DCO removal not supported or failed".to_string()
        ))
    }

    /// Get native max via ATA IDENTIFY
    fn get_native_max_via_identify(device_path: &str) -> DriveResult<u64> {
        let output = Command::new("smartctl")
            .args(["-i", device_path])
            .output()
            .map_err(|e| DriveError::HardwareCommandFailed(
                format!("Failed to get drive info: {}", e)
            ))?;

        let output_str = String::from_utf8_lossy(&output.stdout);

        // Look for LBA sectors
        for line in output_str.lines() {
            if line.contains("User Capacity") || line.contains("Total NVM Capacity") {
                // Extract sector count from line like:
                // "User Capacity: 512,110,190,592 bytes [512 GB]"
                if let Some(bytes_str) = line.split("bytes").next() {
                    if let Some(num_part) = bytes_str.split(':').nth(1) {
                        let clean_num = num_part.chars()
                            .filter(|c| c.is_numeric())
                            .collect::<String>();
                        if let Ok(bytes) = clean_num.parse::<u64>() {
                            return Ok(bytes / 512); // Convert to sectors
                        }
                    }
                }
            }
        }

        // Last resort: use blockdev
        Self::get_size_via_blockdev(device_path)
    }

    /// Get size via blockdev command
    fn get_size_via_blockdev(device_path: &str) -> DriveResult<u64> {
        let output = Command::new("blockdev")
            .args(["--getsz", device_path])
            .output()
            .map_err(|e| DriveError::HardwareCommandFailed(
                format!("Failed to get block device size: {}", e)
            ))?;

        let size_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        size_str.parse::<u64>()
            .map_err(|e| DriveError::HardwareCommandFailed(
                format!("Failed to parse block device size: {}", e)
            ))
    }

    /// Calculate actual usable space considering HPA and DCO
    pub fn get_true_capacity(device_path: &str) -> DriveResult<u64> {
        let mut capacity = Self::get_size_via_blockdev(device_path)? * 512; // Convert to bytes

        // Check for HPA
        if let Some(hpa_info) = Self::detect_hpa(device_path)? {
            capacity += hpa_info.hidden_size_bytes;
        }

        // Check for DCO
        if let Some(dco_info) = Self::detect_dco(device_path)? {
            capacity += dco_info.hidden_size_bytes;
        }

        Ok(capacity)
    }

    /// Comprehensive check for hidden areas
    pub fn check_hidden_areas(device_path: &str) -> DriveResult<(Option<HPAInfo>, Option<DCOInfo>)> {
        let hpa = Self::detect_hpa(device_path)?;
        let dco = Self::detect_dco(device_path)?;

        if hpa.is_some() || dco.is_some() {
            println!("\n⚠️  Hidden areas detected on drive!");

            if let Some(ref h) = hpa {
                println!("  HPA: {} MB hidden", h.hidden_size_bytes / (1024 * 1024));
            }

            if let Some(ref d) = dco {
                println!("  DCO: {} MB hidden", d.hidden_size_bytes / (1024 * 1024));
            }

            println!("  These areas may contain data that won't be wiped unless removed.\n");
        }

        Ok((hpa, dco))
    }
}
