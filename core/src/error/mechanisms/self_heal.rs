/// Self-healing mechanisms for automatic device/driver recovery
///
/// This module provides automatic recovery from device/driver issues through:
/// - Driver reload (SATA/NVMe module reload)
/// - Device reset via sysfs
/// - Controller reset commands
/// - IPMI power cycling (for servers)
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use std::process::Command;
use std::thread;
use std::time::Duration;

/// Self-healing method types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealMethod {
    /// Reload SATA/NVMe kernel driver
    ReloadDriver,

    /// Reset device via sysfs
    ResetDevice,

    /// Reset RAID/HBA controller
    ResetController,

    /// Power cycle via IPMI (servers only)
    PowerCycle,
}

impl HealMethod {
    /// Get human-readable description
    pub fn description(&self) -> &'static str {
        match self {
            HealMethod::ReloadDriver => "Reload kernel driver",
            HealMethod::ResetDevice => "Reset device via sysfs",
            HealMethod::ResetController => "Reset RAID/HBA controller",
            HealMethod::PowerCycle => "Power cycle via IPMI",
        }
    }

    /// Estimated recovery time
    pub fn estimated_recovery_time(&self) -> Duration {
        match self {
            HealMethod::ReloadDriver => Duration::from_secs(5),
            HealMethod::ResetDevice => Duration::from_secs(3),
            HealMethod::ResetController => Duration::from_secs(10),
            HealMethod::PowerCycle => Duration::from_secs(60),
        }
    }

    /// Risk level (0-10, where 10 is highest risk)
    pub fn risk_level(&self) -> u8 {
        match self {
            HealMethod::ReloadDriver => 3,
            HealMethod::ResetDevice => 2,
            HealMethod::ResetController => 5,
            HealMethod::PowerCycle => 8,
        }
    }
}

/// Self-healing mechanism executor
pub struct SelfHealer;

impl SelfHealer {
    /// Create new self-healer
    pub fn new() -> Self {
        Self
    }

    /// Execute healing method
    pub fn heal(&self, device: &str, method: HealMethod) -> Result<()> {
        tracing::info!(
            device = %device,
            method = ?method,
            description = method.description(),
            risk = method.risk_level(),
            "Attempting self-healing"
        );

        let result = match method {
            HealMethod::ReloadDriver => self.reload_driver(device),
            HealMethod::ResetDevice => self.reset_device(device),
            HealMethod::ResetController => self.reset_controller(device),
            HealMethod::PowerCycle => self.power_cycle_ipmi(),
        };

        match &result {
            Ok(_) => {
                tracing::info!(
                    device = %device,
                    method = ?method,
                    "Self-healing successful"
                );
            }
            Err(e) => {
                tracing::error!(
                    device = %device,
                    method = ?method,
                    error = %e,
                    "Self-healing failed"
                );
            }
        }

        result
    }

    /// Reload SATA/NVMe kernel driver
    ///
    /// Approaches:
    /// 1. Identify driver (ahci, nvme, etc.)
    /// 2. rmmod driver
    /// 3. modprobe driver
    /// 4. Wait for re-enumeration
    pub fn reload_driver(&self, device: &str) -> Result<()> {
        let driver = self.identify_driver(device)?;

        tracing::info!(driver = %driver, "Reloading driver");

        // Check if driver is loaded
        let lsmod_output = Command::new("lsmod")
            .output()
            .context("Failed to run lsmod")?;

        let lsmod_str = String::from_utf8_lossy(&lsmod_output.stdout);
        if !lsmod_str.contains(&driver) {
            return Err(anyhow::anyhow!("Driver {} not currently loaded", driver));
        }

        // Remove driver module
        tracing::debug!("Running: rmmod {}", driver);
        let status = Command::new("rmmod")
            .arg(&driver)
            .status()
            .context("Failed to execute rmmod")?;

        if !status.success() {
            return Err(anyhow::anyhow!("Failed to remove driver module {}", driver));
        }

        // Wait a moment
        thread::sleep(Duration::from_millis(500));

        // Reload driver module
        tracing::debug!("Running: modprobe {}", driver);
        let status = Command::new("modprobe")
            .arg(&driver)
            .status()
            .context("Failed to execute modprobe")?;

        if !status.success() {
            return Err(anyhow::anyhow!("Failed to reload driver module {}", driver));
        }

        // Wait for device re-enumeration
        thread::sleep(Duration::from_secs(2));

        // Verify device reappeared
        if !Path::new(device).exists() {
            return Err(anyhow::anyhow!(
                "Device {} did not reappear after driver reload",
                device
            ));
        }

        Ok(())
    }

    /// Identify driver for device
    fn identify_driver(&self, device: &str) -> Result<String> {
        // Extract device name (e.g., "sda" from "/dev/sda")
        let dev_name = device.strip_prefix("/dev/").unwrap_or(device);

        // Try to read driver from sysfs
        let driver_path = format!("/sys/block/{}/device/driver", dev_name);
        if let Ok(link) = fs::read_link(&driver_path) {
            if let Some(driver_name) = link.file_name() {
                return Ok(driver_name.to_string_lossy().to_string());
            }
        }

        // Fallback: guess based on device name
        if dev_name.starts_with("sd") || dev_name.starts_with("sr") {
            Ok("ahci".to_string()) // Most common SATA driver
        } else if dev_name.starts_with("nvme") {
            Ok("nvme".to_string())
        } else if dev_name.starts_with("hd") {
            Ok("ide".to_string())
        } else {
            Err(anyhow::anyhow!(
                "Cannot determine driver for device {}",
                device
            ))
        }
    }

    /// Reset device via sysfs
    ///
    /// Approaches:
    /// 1. Delete device from sysfs
    /// 2. Rescan SCSI/PCI bus
    /// 3. Wait for device to reappear
    pub fn reset_device(&self, device: &str) -> Result<()> {
        let dev_name = device.strip_prefix("/dev/").unwrap_or(device);

        tracing::info!(device = %dev_name, "Resetting device via sysfs");

        // For SCSI/SATA devices
        if dev_name.starts_with("sd") {
            return self.reset_scsi_device(dev_name);
        }

        // For NVMe devices
        if dev_name.starts_with("nvme") {
            return self.reset_nvme_device(dev_name);
        }

        Err(anyhow::anyhow!(
            "Device type not supported for sysfs reset: {}",
            device
        ))
    }

    /// Reset SCSI/SATA device
    fn reset_scsi_device(&self, dev_name: &str) -> Result<()> {
        // Delete device
        let delete_path = format!("/sys/block/{}/device/delete", dev_name);
        if Path::new(&delete_path).exists() {
            tracing::debug!("Writing 1 to {}", delete_path);
            fs::write(&delete_path, "1").context("Failed to delete device")?;
        }

        thread::sleep(Duration::from_secs(1));

        // Rescan SCSI bus
        let scan_pattern = "/sys/class/scsi_host/host*/scan";
        let scan_paths = glob::glob(scan_pattern).context("Failed to glob scan paths")?;

        for path in scan_paths.flatten() {
            tracing::debug!("Writing '- - -' to {}", path.display());
            if fs::write(&path, "- - -").is_ok() {
                // Give it a moment
                thread::sleep(Duration::from_millis(500));
            }
        }

        // Wait for device to reappear
        thread::sleep(Duration::from_secs(2));

        let device_path = format!("/dev/{}", dev_name);
        if !Path::new(&device_path).exists() {
            return Err(anyhow::anyhow!("Device did not reappear after reset"));
        }

        Ok(())
    }

    /// Reset NVMe device
    fn reset_nvme_device(&self, dev_name: &str) -> Result<()> {
        // Extract controller name (e.g., "nvme0" from "nvme0n1")
        let ctrl_name = dev_name
            .split('n')
            .next()
            .ok_or_else(|| anyhow::anyhow!("Invalid NVMe device name"))?;

        // Reset via sysfs
        let reset_path = format!("/sys/class/nvme/{}/reset_controller", ctrl_name);
        if Path::new(&reset_path).exists() {
            tracing::debug!("Writing 1 to {}", reset_path);
            fs::write(&reset_path, "1").context("Failed to reset NVMe controller")?;

            thread::sleep(Duration::from_secs(3));

            let device_path = format!("/dev/{}", dev_name);
            if !Path::new(&device_path).exists() {
                return Err(anyhow::anyhow!("NVMe device did not reappear after reset"));
            }

            return Ok(());
        }

        // Fallback: try PCI remove/rescan
        self.reset_pci_device(ctrl_name)
    }

    /// Reset PCI device (works for any PCI device)
    fn reset_pci_device(&self, device_name: &str) -> Result<()> {
        // Find PCI address
        let device_link = format!("/sys/class/block/{}/device", device_name);
        if let Ok(real_path) = fs::read_link(&device_link) {
            // Extract PCI address from path
            if let Some(pci_addr) = real_path
                .to_str()
                .and_then(|s| s.split('/').find(|p| p.contains(":")))
            {
                // Remove device
                let remove_path = format!("/sys/bus/pci/devices/{}/remove", pci_addr);
                if Path::new(&remove_path).exists() {
                    tracing::debug!("Writing 1 to {}", remove_path);
                    fs::write(&remove_path, "1").context("Failed to remove PCI device")?;
                }

                thread::sleep(Duration::from_secs(1));

                // Rescan PCI bus
                let rescan_path = "/sys/bus/pci/rescan";
                tracing::debug!("Writing 1 to {}", rescan_path);
                fs::write(rescan_path, "1").context("Failed to rescan PCI bus")?;

                thread::sleep(Duration::from_secs(2));
                return Ok(());
            }
        }

        Err(anyhow::anyhow!("Could not find PCI address for device"))
    }

    /// Reset RAID/HBA controller
    ///
    /// Reuses freeze mitigation vendor-specific reset commands
    pub fn reset_controller(&self, device: &str) -> Result<()> {
        tracing::info!(device = %device, "Resetting controller");

        // Detect controller type
        let controller = self.detect_controller_type(device)?;

        match controller.as_str() {
            "megaraid" => self.reset_megaraid(),
            "hpsa" => self.reset_hpsa(),
            "mpt" => self.reset_mpt(),
            _ => Err(anyhow::anyhow!(
                "Controller reset not supported for type: {}",
                controller
            )),
        }
    }

    /// Detect controller type
    fn detect_controller_type(&self, _device: &str) -> Result<String> {
        // Check for MegaRAID
        if Path::new("/opt/MegaRAID/MegaCli/MegaCli64").exists() {
            return Ok("megaraid".to_string());
        }

        // Check for HP SmartArray
        if Path::new("/usr/sbin/hpssacli").exists() || Path::new("/usr/sbin/ssacli").exists() {
            return Ok("hpsa".to_string());
        }

        // Check for LSI MPT
        if Command::new("lspci")
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.contains("LSI") || s.contains("MPT"))
            .unwrap_or(false)
        {
            return Ok("mpt".to_string());
        }

        Err(anyhow::anyhow!("No supported controller detected"))
    }

    /// Reset MegaRAID controller
    fn reset_megaraid(&self) -> Result<()> {
        let output = Command::new("/opt/MegaRAID/MegaCli/MegaCli64")
            .args(&["-AdpReset", "-a0"])
            .output()
            .context("Failed to reset MegaRAID controller")?;

        if !output.status.success() {
            return Err(anyhow::anyhow!("MegaRAID reset command failed"));
        }

        thread::sleep(Duration::from_secs(10));
        Ok(())
    }

    /// Reset HP SmartArray controller
    fn reset_hpsa(&self) -> Result<()> {
        let cmd = if Path::new("/usr/sbin/hpssacli").exists() {
            "/usr/sbin/hpssacli"
        } else {
            "/usr/sbin/ssacli"
        };

        let output = Command::new(cmd)
            .args(&["ctrl", "all", "diag", "file=/dev/null"])
            .output()
            .context("Failed to reset HP SmartArray")?;

        if !output.status.success() {
            return Err(anyhow::anyhow!("HP SmartArray reset failed"));
        }

        thread::sleep(Duration::from_secs(5));
        Ok(())
    }

    /// Reset LSI MPT controller
    fn reset_mpt(&self) -> Result<()> {
        // MPT controllers can be reset via sysfs or driver reload
        // This is a simplified version
        let output = Command::new("modprobe")
            .args(&["-r", "mpt3sas"])
            .output()
            .context("Failed to unload mpt3sas")?;

        if !output.status.success() {
            return Err(anyhow::anyhow!("Failed to unload mpt3sas driver"));
        }

        thread::sleep(Duration::from_secs(1));

        let output = Command::new("modprobe")
            .arg("mpt3sas")
            .output()
            .context("Failed to load mpt3sas")?;

        if !output.status.success() {
            return Err(anyhow::anyhow!("Failed to reload mpt3sas driver"));
        }

        thread::sleep(Duration::from_secs(5));
        Ok(())
    }

    /// Power cycle system via IPMI
    ///
    /// WARNING: This resets the entire system!
    pub fn power_cycle_ipmi(&self) -> Result<()> {
        tracing::warn!("Attempting IPMI power cycle - this will reset the entire system!");

        // Verify ipmitool is available
        if !self.is_ipmi_available() {
            return Err(anyhow::anyhow!("IPMI not available (ipmitool not found)"));
        }

        // Try chassis power cycle
        let output = Command::new("ipmitool")
            .args(&["chassis", "power", "cycle"])
            .output()
            .context("Failed to execute IPMI power cycle")?;

        if !output.status.success() {
            return Err(anyhow::anyhow!("IPMI power cycle command failed"));
        }

        Ok(())
    }

    /// Check if IPMI is available
    fn is_ipmi_available(&self) -> bool {
        Command::new("which")
            .arg("ipmitool")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}

impl Default for SelfHealer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heal_method_properties() {
        assert_eq!(
            HealMethod::ReloadDriver.description(),
            "Reload kernel driver"
        );
        assert_eq!(HealMethod::ReloadDriver.risk_level(), 3);
        assert!(HealMethod::ReloadDriver.estimated_recovery_time() > Duration::from_secs(0));
    }

    #[test]
    fn test_identify_driver_sata() {
        let healer = SelfHealer::new();

        // Should work with /dev/ prefix
        let result = healer.identify_driver("/dev/sda");
        assert!(result.is_ok());

        // Should default to ahci for sd* devices
        assert_eq!(result.unwrap(), "ahci");
    }

    #[test]
    fn test_identify_driver_nvme() {
        let healer = SelfHealer::new();
        let result = healer.identify_driver("/dev/nvme0n1");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "nvme");
    }

    #[test]
    fn test_heal_method_ordering_by_risk() {
        let mut methods = vec![
            HealMethod::ReloadDriver,
            HealMethod::ResetDevice,
            HealMethod::ResetController,
            HealMethod::PowerCycle,
        ];

        // Sort by risk level
        methods.sort_by_key(|m| m.risk_level());

        // Verify sorted order matches expected
        assert_eq!(methods[0], HealMethod::ResetDevice); // Risk 2
        assert_eq!(methods[1], HealMethod::ReloadDriver); // Risk 3
        assert_eq!(methods[2], HealMethod::ResetController); // Risk 5
        assert_eq!(methods[3], HealMethod::PowerCycle); // Risk 8
    }

    #[test]
    fn test_is_ipmi_available() {
        let healer = SelfHealer::new();
        // Just test that it doesn't panic
        let _ = healer.is_ipmi_available();
    }

    // Note: Most other tests require root privileges and actual hardware
    // Those should be run in integration test suite with appropriate setup
}
