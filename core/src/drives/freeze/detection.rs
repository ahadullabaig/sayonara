// Detect the reason why a drive is frozen

use anyhow::{anyhow, Result};
use std::fs;
use std::path::Path;
use std::process::Command;

/// Reasons why a drive might be frozen
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FreezeReason {
    /// BIOS set the frozen bit during POST
    BiosSetFrozen,
    /// RAID or HBA controller policy
    RaidController,
    /// Operating system security policy
    OsSecurity,
    /// Controller-specific freeze policy
    ControllerPolicy,
    /// Unknown reason
    Unknown,
}

/// Detector for freeze reasons
pub struct FreezeDetector;

impl FreezeDetector {
    /// Detect why a drive is frozen
    pub fn detect_reason(device_path: &str) -> Result<FreezeReason> {
        println!("  🔍 Analyzing freeze reason...");

        // Check 1: RAID controller
        if Self::is_raid_member(device_path)? {
            println!("     → Drive is part of RAID array");
            return Ok(FreezeReason::RaidController);
        }

        // Check 2: BIOS freeze detection
        if Self::is_bios_frozen(device_path)? {
            println!("     → BIOS set frozen bit during POST");
            return Ok(FreezeReason::BiosSetFrozen);
        }

        // Check 3: Controller-specific policies
        if let Some(controller_type) = Self::detect_controller_type(device_path)? {
            println!("     → Controller type: {}", controller_type);

            if Self::has_controller_freeze_policy(&controller_type) {
                println!("     → Controller has known freeze policy");
                return Ok(FreezeReason::ControllerPolicy);
            }
        }

        // Check 4: OS-level security
        if Self::has_os_security_freeze(device_path)? {
            println!("     → OS security policy detected");
            return Ok(FreezeReason::OsSecurity);
        }

        println!("     → Unable to determine specific reason");
        Ok(FreezeReason::Unknown)
    }

    /// Check if drive is BIOS frozen
    fn is_bios_frozen(device_path: &str) -> Result<bool> {
        let output = Command::new("hdparm").args(["-I", device_path]).output()?;

        let output_str = String::from_utf8_lossy(&output.stdout);

        // Look for specific BIOS freeze indicators
        Ok(output_str.contains("frozen") && output_str.contains("BIOS"))
    }

    /// Check if drive is part of RAID
    fn is_raid_member(device_path: &str) -> Result<bool> {
        // Check mdadm
        let mdadm_check = Command::new("mdadm")
            .args(["--examine", device_path])
            .output();

        if let Ok(output) = mdadm_check {
            if output.status.success() {
                let output_str = String::from_utf8_lossy(&output.stdout);
                if output_str.contains("Raid Level") || output_str.contains("Array UUID") {
                    return Ok(true);
                }
            }
        }

        // Check for hardware RAID signatures
        let device_name = Path::new(device_path)
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| anyhow!("Invalid device path"))?;

        let sysfs_path = format!("/sys/block/{}/device/vendor", device_name);
        if let Ok(vendor) = fs::read_to_string(&sysfs_path) {
            let vendor_lower = vendor.to_lowercase();
            if vendor_lower.contains("lsi")
                || vendor_lower.contains("megaraid")
                || vendor_lower.contains("adaptec")
                || vendor_lower.contains("hp")
                || vendor_lower.contains("dell")
            {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Detect controller type
    fn detect_controller_type(device_path: &str) -> Result<Option<String>> {
        let device_name = Path::new(device_path)
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| anyhow!("Invalid device path"))?;

        // Try to get controller info from sysfs
        let sysfs_device = format!("/sys/block/{}/device", device_name);

        if !Path::new(&sysfs_device).exists() {
            return Ok(None);
        }

        let real_path = fs::read_link(&sysfs_device)?;
        let path_str = real_path.to_string_lossy();

        // Parse controller type from path
        if path_str.contains("ahci") {
            return Ok(Some("AHCI".to_string()));
        } else if path_str.contains("ata") {
            return Ok(Some("ATA".to_string()));
        } else if path_str.contains("nvme") {
            return Ok(Some("NVMe".to_string()));
        } else if path_str.contains("usb") {
            return Ok(Some("USB".to_string()));
        }

        // Try lspci for more details
        let lspci_output = Command::new("lspci").args(["-v"]).output();

        if let Ok(output) = lspci_output {
            let output_str = String::from_utf8_lossy(&output.stdout);

            if output_str.contains("Intel") && output_str.contains("SATA") {
                return Ok(Some("Intel RST".to_string()));
            } else if output_str.contains("LSI") || output_str.contains("MegaRAID") {
                return Ok(Some("LSI MegaRAID".to_string()));
            } else if output_str.contains("Adaptec") {
                return Ok(Some("Adaptec".to_string()));
            } else if output_str.contains("PERC") {
                return Ok(Some("Dell PERC".to_string()));
            } else if output_str.contains("SmartArray") {
                return Ok(Some("HP SmartArray".to_string()));
            }
        }

        Ok(None)
    }

    /// Check if controller has known freeze policy
    fn has_controller_freeze_policy(controller_type: &str) -> bool {
        // Known controllers that aggressively freeze drives
        matches!(
            controller_type,
            "Intel RST" | "Dell PERC" | "HP SmartArray" | "LSI MegaRAID"
        )
    }

    /// Check for OS-level security freeze
    fn has_os_security_freeze(device_path: &str) -> Result<bool> {
        let _device_name = Path::new(device_path)
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| anyhow!("Invalid device path"))?;

        // Check udev rules that might freeze drives
        let udev_rules_dirs = ["/lib/udev/rules.d", "/etc/udev/rules.d"];

        for rules_dir in &udev_rules_dirs {
            if let Ok(entries) = fs::read_dir(rules_dir) {
                for entry in entries.flatten() {
                    if let Ok(content) = fs::read_to_string(entry.path()) {
                        if content.contains("hdparm") && content.contains("--security-freeze") {
                            return Ok(true);
                        }
                    }
                }
            }
        }

        // Check systemd services
        let systemd_dirs = ["/etc/systemd/system", "/lib/systemd/system"];

        for systemd_dir in &systemd_dirs {
            if let Ok(entries) = fs::read_dir(systemd_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|s| s.to_str()) == Some("service") {
                        if let Ok(content) = fs::read_to_string(&path) {
                            if content.contains("hdparm") && content.contains("freeze") {
                                return Ok(true);
                            }
                        }
                    }
                }
            }
        }

        Ok(false)
    }

    /// Get human-readable description of freeze reason
    pub fn describe_reason(reason: &FreezeReason) -> String {
        match reason {
            FreezeReason::BiosSetFrozen => {
                "Drive was frozen by BIOS during system boot (POST). \
                 This is a common security feature. Solutions: \
                 1) SATA link reset, 2) System sleep/wake, 3) Cold boot"
            }
            FreezeReason::RaidController => {
                "Drive is managed by a RAID controller which enforces freeze policy. \
                 Solutions: 1) Use controller-specific commands, \
                 2) Temporarily remove from RAID array, 3) Controller reset"
            }
            FreezeReason::OsSecurity => {
                "Operating system has a security policy that freezes drives. \
                 Solutions: 1) Disable udev rules, 2) Boot from Live USB, \
                 3) Modify systemd services"
            }
            FreezeReason::ControllerPolicy => {
                "Storage controller has a built-in freeze policy. \
                 Solutions: 1) Controller-specific unlock commands, \
                 2) PCIe hot-reset, 3) Update controller firmware"
            }
            FreezeReason::Unknown => {
                "Freeze reason could not be determined. \
                 Try all available unfreeze methods in sequence."
            }
        }
        .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_freeze_reason_description() {
        let desc = FreezeDetector::describe_reason(&FreezeReason::BiosSetFrozen);
        assert!(desc.contains("BIOS"));
        assert!(desc.contains("POST"));

        let desc = FreezeDetector::describe_reason(&FreezeReason::RaidController);
        assert!(desc.contains("RAID"));
    }

    #[test]
    fn test_controller_freeze_policy() {
        assert!(FreezeDetector::has_controller_freeze_policy("Intel RST"));
        assert!(FreezeDetector::has_controller_freeze_policy("Dell PERC"));
        assert!(!FreezeDetector::has_controller_freeze_policy(
            "Generic AHCI"
        ));
    }
}
