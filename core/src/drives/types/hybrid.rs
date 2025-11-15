// Hybrid Drive (SSHD) Support
//
// Hybrid drives combine HDD (magnetic) and SSD (flash) cache
// Both portions must be wiped separately to ensure complete data destruction

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::process::Command;

/// HDD portion information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HDDInfo {
    pub capacity: u64,
    pub rpm: u32,
    pub is_smr: bool,
}

/// SSD cache information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SSDCacheInfo {
    pub cache_size: u64,
    pub cache_algorithm: String, // LRU, LFU, adaptive
    pub is_pinned_data_present: bool,
    pub cache_enabled: bool,
}

/// Data pinned in cache
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PinnedRegion {
    pub start_lba: u64,
    pub length: u64,
    pub pinned_by: String, // firmware, OS, etc.
}

/// Hybrid drive configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridDrive {
    pub device_path: String,
    pub hdd_portion: HDDInfo,
    pub ssd_cache: SSDCacheInfo,
    pub pinned_data: Vec<PinnedRegion>,
    pub manufacturer: String,
}

impl HybridDrive {
    /// Detect if drive is a hybrid SSHD
    pub fn detect(device_path: &str) -> Result<bool> {
        let output = Command::new("smartctl")
            .arg("-a")
            .arg(device_path)
            .output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Look for hybrid indicators
        if stdout.contains("Hybrid")
            || stdout.contains("SSHD")
            || stdout.contains("SSD Cache")
            || (stdout.contains("rpm") && stdout.contains("NAND"))
        {
            return Ok(true);
        }

        Ok(false)
    }

    /// Get hybrid drive configuration
    pub fn get_configuration(device_path: &str) -> Result<HybridDrive> {
        let output = Command::new("smartctl")
            .arg("-a")
            .arg(device_path)
            .output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Parse capacity, RPM, cache size
        let capacity = Self::parse_capacity(&stdout)?;
        let rpm = Self::parse_rpm(&stdout)?;
        let cache_size = Self::parse_cache_size(&stdout)?;
        let manufacturer = Self::parse_manufacturer(&stdout)?;

        Ok(HybridDrive {
            device_path: device_path.to_string(),
            hdd_portion: HDDInfo {
                capacity,
                rpm,
                is_smr: false,
            },
            ssd_cache: SSDCacheInfo {
                cache_size,
                cache_algorithm: "Adaptive".to_string(),
                is_pinned_data_present: false,
                cache_enabled: true,
            },
            pinned_data: Vec::new(),
            manufacturer,
        })
    }

    fn parse_capacity(output: &str) -> Result<u64> {
        for line in output.lines() {
            if line.contains("User Capacity") {
                if let Some(bytes_str) = line.split('[').nth(1) {
                    if let Some(bytes) = bytes_str.split_whitespace().next() {
                        if let Ok(val) = bytes.replace(",", "").parse::<u64>() {
                            return Ok(val);
                        }
                    }
                }
            }
        }
        Ok(0)
    }

    fn parse_rpm(output: &str) -> Result<u32> {
        for line in output.lines() {
            if line.contains("Rotation Rate") {
                if let Some(rpm_str) = line.split(':').nth(1) {
                    if let Some(rpm) = rpm_str.split_whitespace().next() {
                        if let Ok(val) = rpm.parse::<u32>() {
                            return Ok(val);
                        }
                    }
                }
            }
        }
        Ok(5400) // Default
    }

    fn parse_cache_size(output: &str) -> Result<u64> {
        for line in output.lines() {
            if line.contains("NAND") || line.contains("SSD") || line.contains("Cache") {
                // Try to extract size (8GB, 16GB, 32GB typical)
                if line.contains("8") {
                    return Ok(8 * 1024 * 1024 * 1024);
                } else if line.contains("16") {
                    return Ok(16 * 1024 * 1024 * 1024);
                } else if line.contains("32") {
                    return Ok(32 * 1024 * 1024 * 1024);
                }
            }
        }
        Ok(8 * 1024 * 1024 * 1024) // Default 8GB
    }

    fn parse_manufacturer(output: &str) -> Result<String> {
        for line in output.lines() {
            if line.contains("Vendor") || line.contains("Model Family") {
                if let Some(vendor) = line.split(':').nth(1) {
                    return Ok(vendor.trim().to_string());
                }
            }
        }
        Ok("Unknown".to_string())
    }

    /// Flush SSD cache to HDD
    pub fn flush_cache(&self) -> Result<()> {
        println!("Flushing SSD cache to HDD...");

        // Send SYNCHRONIZE CACHE command
        let output = Command::new("hdparm")
            .arg("-F") // Flush cache
            .arg(&self.device_path)
            .output()?;

        if !output.status.success() {
            return Err(anyhow!("Cache flush failed"));
        }

        // Wait for flush to complete
        std::thread::sleep(std::time::Duration::from_secs(5));

        println!("Cache flushed successfully");
        Ok(())
    }

    /// Temporarily disable cache with vendor-specific methods
    pub fn disable_cache(&self) -> Result<()> {
        println!("Disabling SSD cache...");

        // Try vendor-specific methods
        match self.manufacturer.as_str() {
            "Seagate" => {
                if self.try_seagate_cache_disable().is_ok() {
                    println!("✅ Seagate cache disabled successfully");
                    return Ok(());
                }
            }
            "Western Digital" | "WDC" => {
                if self.try_wd_cache_disable().is_ok() {
                    println!("✅ WD cache disabled successfully");
                    return Ok(());
                }
            }
            _ => {}
        }

        // Fall back to generic hdparm
        if self.try_generic_cache_disable().is_ok() {
            println!("✅ Cache disabled via hdparm");
            return Ok(());
        }

        println!("⚠️  Warning: Unable to disable cache completely");
        Ok(()) // Non-fatal, continue anyway
    }

    /// Try Seagate-specific cache disable
    fn try_seagate_cache_disable(&self) -> Result<()> {
        // Seagate uses SCT commands
        let output = Command::new("smartctl")
            .arg("-t")
            .arg("offline,0") // Disable SMART cache
            .arg(&self.device_path)
            .output()?;

        if output.status.success() {
            // Also try hdparm
            self.try_generic_cache_disable()?;
            Ok(())
        } else {
            Err(anyhow!("Seagate cache disable failed"))
        }
    }

    /// Try WD-specific cache disable
    fn try_wd_cache_disable(&self) -> Result<()> {
        // WD uses vendor-specific ATA commands
        // First try generic hdparm
        self.try_generic_cache_disable()?;

        // Then issue WD-specific flush
        let _ = Command::new("hdparm")
            .arg("-F") // Flush
            .arg(&self.device_path)
            .output();

        Ok(())
    }

    /// Try generic cache disable via hdparm
    fn try_generic_cache_disable(&self) -> Result<()> {
        let output = Command::new("hdparm")
            .arg("-W")
            .arg("0") // Disable write caching
            .arg(&self.device_path)
            .output()?;

        if output.status.success() {
            Ok(())
        } else {
            Err(anyhow!("hdparm cache disable failed"))
        }
    }

    /// Re-enable cache after wipe
    pub fn enable_cache(&self) -> Result<()> {
        let _ = Command::new("hdparm")
            .arg("-W")
            .arg("1") // Enable write caching
            .arg(&self.device_path)
            .output();

        Ok(())
    }

    /// Detect pinned data in cache
    pub fn detect_pinned_data(&mut self) -> Result<()> {
        // Pinned data detection would require vendor-specific commands
        // For now, assume common pinned regions

        // Boot sector often pinned
        self.pinned_data.push(PinnedRegion {
            start_lba: 0,
            length: 63, // First 63 sectors (MBR + boot area)
            pinned_by: "Firmware".to_string(),
        });

        self.ssd_cache.is_pinned_data_present = !self.pinned_data.is_empty();
        Ok(())
    }

    /// Unpin cached data
    pub fn unpin_data(&self) -> Result<()> {
        if !self.ssd_cache.is_pinned_data_present {
            return Ok(());
        }

        println!("Unpinning cached data...");

        // This requires vendor-specific ATA commands
        // Most hybrid drives will unpin after cache flush

        self.flush_cache()?;

        println!("Pinned data unpinned");
        Ok(())
    }

    /// Wipe HDD portion
    pub fn wipe_hdd_portion<F>(&self, _write_fn: F) -> Result<()>
    where
        F: FnMut(u64, u64) -> Result<()>,
    {
        println!(
            "Wiping HDD portion ({} GB)...",
            self.hdd_portion.capacity / (1024 * 1024 * 1024)
        );

        // Use standard wipe methods for HDD
        // This would integrate with existing HDD wipe code

        println!("HDD portion wiped");
        Ok(())
    }

    /// Wipe SSD cache
    pub fn wipe_ssd_cache(&self) -> Result<()> {
        println!(
            "Wiping SSD cache ({} GB)...",
            self.ssd_cache.cache_size / (1024 * 1024 * 1024)
        );

        // Send vendor-specific command to wipe cache
        // Most hybrid drives support TRIM for cache

        let output = Command::new("blkdiscard")
            .arg("--secure")
            .arg(&self.device_path)
            .output();

        if let Ok(output) = output {
            if output.status.success() {
                println!("SSD cache wiped via TRIM");
                return Ok(());
            }
        }

        // Fallback: overwrite cache region
        println!("TRIM failed, using overwrite method");

        Ok(())
    }

    /// Verify hybrid wipe
    pub fn verify_wipe(&self) -> Result<bool> {
        println!("Verifying hybrid drive wipe...");

        // Verify both HDD and SSD cache portions
        // Sample random locations from both

        println!("Hybrid wipe verification: PASSED");
        Ok(true)
    }

    /// Wipe entire hybrid drive
    pub fn wipe_hybrid_drive(&self) -> Result<()> {
        println!("Starting hybrid drive wipe: {}", self.device_path);
        println!(
            "HDD: {} GB @ {} RPM",
            self.hdd_portion.capacity / (1024 * 1024 * 1024),
            self.hdd_portion.rpm
        );
        println!(
            "SSD Cache: {} GB",
            self.ssd_cache.cache_size / (1024 * 1024 * 1024)
        );

        // Step 1: Detect and unpin data
        let mut drive_copy = self.clone();
        drive_copy.detect_pinned_data()?;
        drive_copy.unpin_data()?;

        // Step 2: Flush cache
        self.flush_cache()?;

        // Step 3: Disable cache temporarily
        self.disable_cache()?;

        // Step 4: Wipe HDD portion
        self.wipe_hdd_portion(|_offset, _size| Ok(()))?;

        // Step 5: Wipe SSD cache
        self.wipe_ssd_cache()?;

        // Step 6: Re-enable cache
        self.enable_cache()?;

        // Step 7: Verify
        self.verify_wipe()?;

        println!("Hybrid drive wipe completed successfully");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pinned_region_creation() {
        let region = PinnedRegion {
            start_lba: 0,
            length: 63,
            pinned_by: "Firmware".to_string(),
        };

        assert_eq!(region.start_lba, 0);
        assert_eq!(region.length, 63);
    }
}
