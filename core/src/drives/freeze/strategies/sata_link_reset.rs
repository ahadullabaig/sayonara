// SATA link power management reset strategy

use super::{StrategyResult, UnfreezeStrategy};
use crate::drives::freeze::detection::FreezeReason;
use anyhow::{anyhow, Result};
use std::fs;
use std::path::Path;
use std::thread;
use std::time::Duration;

pub struct SataLinkReset;

impl Default for SataLinkReset {
    fn default() -> Self {
        Self::new()
    }
}

impl SataLinkReset {
    pub fn new() -> Self {
        Self
    }

    /// Find the SATA link power management control file
    fn find_link_pm_path(&self, device_path: &str) -> Result<String> {
        let device_name = Path::new(device_path)
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| anyhow!("Invalid device path"))?;

        // Try to find via sysfs
        let sysfs_path = format!("/sys/block/{}/device", device_name);

        if !Path::new(&sysfs_path).exists() {
            return Err(anyhow!("Device not found in sysfs"));
        }

        // Resolve to find the real device path
        let real_path = fs::read_link(&sysfs_path)?;
        let path_str = real_path.to_string_lossy();

        // Extract host number (e.g., /devices/.../ata1/host0/...)
        let host_regex = regex::Regex::new(r"ata(\d+)").unwrap();
        if let Some(captures) = host_regex.captures(&path_str) {
            let ata_num = &captures[1];
            let link_pm_path = format!(
                "/sys/class/ata_port/ata{}/link_power_management_policy",
                ata_num
            );

            if Path::new(&link_pm_path).exists() {
                return Ok(link_pm_path);
            }
        }

        Err(anyhow!(
            "Link power management not available for this device"
        ))
    }

    /// Perform link power cycle
    fn cycle_link_power(&self, link_pm_path: &str) -> Result<()> {
        // Read current policy
        let current_policy = fs::read_to_string(link_pm_path)?;
        let current = current_policy.trim();

        println!("      Current link PM policy: {}", current);

        // Cycle through different power states to force link reset
        let policies = [
            "min_power",       // Deepest power save
            "medium_power",    // Medium
            "max_performance", // No power save
        ];

        for (idx, policy) in policies.iter().enumerate() {
            if *policy == current {
                continue; // Skip current policy
            }

            println!("      Setting link PM to: {}", policy);
            fs::write(link_pm_path, policy.as_bytes())?;

            // Wait for link to transition
            thread::sleep(Duration::from_millis(500));

            // If this is the last policy, wait longer
            if idx == policies.len() - 1 {
                thread::sleep(Duration::from_secs(2));
            }
        }

        // Restore original policy or set to max_performance
        let restore_policy = if current == "min_power" || current == "medium_power" {
            current
        } else {
            "max_performance"
        };

        println!("      Restoring link PM to: {}", restore_policy);
        fs::write(link_pm_path, restore_policy.as_bytes())?;

        thread::sleep(Duration::from_secs(1));

        Ok(())
    }

    /// Trigger SATA link reset via sysfs
    fn trigger_link_reset(&self, device_path: &str) -> Result<()> {
        let device_name = Path::new(device_path)
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| anyhow!("Invalid device path"))?;

        // Find the SCSI host
        let host_path = format!("/sys/block/{}/device/scsi_device", device_name);

        if Path::new(&host_path).exists() {
            // Trigger rescan
            let entries = fs::read_dir(&host_path)?;
            for entry in entries {
                let entry = entry?;
                let rescan_path = format!("{}/device/rescan", entry.path().display());

                if Path::new(&rescan_path).exists() {
                    println!("      Triggering SCSI rescan");
                    fs::write(&rescan_path, b"1")?;
                    thread::sleep(Duration::from_secs(2));
                }
            }
        }

        Ok(())
    }
}

impl UnfreezeStrategy for SataLinkReset {
    fn name(&self) -> &str {
        "SATA Link Reset"
    }

    fn description(&self) -> &str {
        "Cycles SATA link power states to reset the link, clearing freeze state"
    }

    fn is_compatible_with(&self, reason: &FreezeReason) -> bool {
        matches!(
            reason,
            FreezeReason::BiosSetFrozen | FreezeReason::ControllerPolicy | FreezeReason::Unknown
        )
    }

    fn is_available(&self) -> bool {
        // Check if we have a SATA device with link PM support
        Path::new("/sys/class/ata_port").exists()
    }

    fn execute(&self, device_path: &str, _reason: &FreezeReason) -> Result<StrategyResult> {
        println!("      🔗 Executing SATA link reset");

        // Method 1: Link power management cycling
        match self.find_link_pm_path(device_path) {
            Ok(link_pm_path) => {
                println!("      Found link PM control: {}", link_pm_path);

                if let Err(e) = self.cycle_link_power(&link_pm_path) {
                    println!("      ⚠️  Link PM cycling failed: {}", e);
                } else {
                    println!("      ✅ Link PM cycle complete");
                    return Ok(StrategyResult::success(
                        "SATA link reset via power management cycling",
                    ));
                }
            }
            Err(e) => {
                println!("      ℹ️  Link PM not available: {}", e);
            }
        }

        // Method 2: SCSI rescan trigger
        match self.trigger_link_reset(device_path) {
            Ok(_) => {
                println!("      ✅ SCSI rescan triggered");
                Ok(StrategyResult::success_with_warning(
                    "SATA link reset via SCSI rescan",
                    "Link PM cycling was not available",
                ))
            }
            Err(e) => Err(anyhow!("SATA link reset failed: {}", e)),
        }
    }

    fn estimated_duration(&self) -> u64 {
        5 // 5 seconds
    }

    fn risk_level(&self) -> u8 {
        2 // Low risk
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sata_link_reset_compatibility() {
        let strategy = SataLinkReset::new();

        assert!(strategy.is_compatible_with(&FreezeReason::BiosSetFrozen));
        assert!(strategy.is_compatible_with(&FreezeReason::ControllerPolicy));
        assert!(!strategy.is_compatible_with(&FreezeReason::RaidController));
    }

    #[test]
    fn test_sata_link_reset_properties() {
        let strategy = SataLinkReset::new();

        assert_eq!(strategy.name(), "SATA Link Reset");
        assert_eq!(strategy.risk_level(), 2);
        assert_eq!(strategy.estimated_duration(), 5);
    }
}
