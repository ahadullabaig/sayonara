use crate::{DriveError, DriveResult, FreezeStatus};
use anyhow::{anyhow, Result};
use std::fs;
use std::path::Path;
use std::process::Command;
use std::thread;
use std::time::Duration;

pub struct FreezeMitigation;

impl FreezeMitigation {
    /// Attempt to unfreeze a drive using multiple methods
    pub fn unfreeze_drive(device_path: &str) -> DriveResult<()> {
        println!("Checking drive freeze status for {}...", device_path);

        let status = Self::get_freeze_status(device_path)?;

        match status {
            FreezeStatus::NotFrozen => {
                println!("Drive is not frozen, proceeding...");
                return Ok(());
            }
            FreezeStatus::FrozenByBIOS => {
                println!("Drive is frozen by BIOS, attempting mitigation...");
            }
            FreezeStatus::Frozen => {
                println!("Drive is frozen, attempting mitigation...");
            }
            FreezeStatus::SecurityLocked => {
                return Err(DriveError::DriveFrozen(
                    "Drive is security locked and cannot be unfrozen without password".to_string(),
                ));
            }
            _ => {}
        }

        // Try multiple unfreezing methods in order of preference
        let methods: Vec<(&str, fn(&str) -> Result<()>)> = vec![
            (
                "Sleep/Wake",
                Self::unfreeze_via_sleep as fn(&str) -> Result<()>,
            ),
            (
                "Hot-plug",
                Self::unfreeze_via_hotplug as fn(&str) -> Result<()>,
            ),
            (
                "Link PM",
                Self::unfreeze_via_link_power as fn(&str) -> Result<()>,
            ),
            (
                "Power Cycle",
                Self::unfreeze_via_power_cycle as fn(&str) -> Result<()>,
            ),
        ];

        for (method_name, method_fn) in methods {
            println!("Attempting {} method...", method_name);

            if method_fn(device_path).is_ok() {
                // Verify unfreeze was successful
                thread::sleep(Duration::from_secs(2));
                let new_status = Self::get_freeze_status(device_path)?;

                if new_status == FreezeStatus::NotFrozen {
                    println!("Successfully unfrozen drive using {} method", method_name);
                    return Ok(());
                }
            }

            println!("{} method failed, trying next...", method_name);
        }

        Err(DriveError::DriveFrozen(format!(
            "Failed to unfreeze drive {} after trying all methods",
            device_path
        )))
    }

    /// Get the freeze status of a drive
    pub fn get_freeze_status(device_path: &str) -> DriveResult<FreezeStatus> {
        let output = Command::new("hdparm")
            .args(["-I", device_path])
            .output()
            .map_err(|e| DriveError::HardwareCommandFailed(format!("hdparm failed: {}", e)))?;

        let output_str = String::from_utf8_lossy(&output.stdout);

        if output_str.contains("frozen") && output_str.contains("not") {
            Ok(FreezeStatus::NotFrozen)
        } else if output_str.contains("frozen") {
            if output_str.contains("BIOS") {
                Ok(FreezeStatus::FrozenByBIOS)
            } else {
                Ok(FreezeStatus::Frozen)
            }
        } else if output_str.contains("locked") {
            Ok(FreezeStatus::SecurityLocked)
        } else {
            Ok(FreezeStatus::NotFrozen)
        }
    }

    /// Method 1: System sleep/wake cycle
    fn unfreeze_via_sleep(device_path: &str) -> Result<()> {
        // Check if sleep is supported
        if !Path::new("/sys/power/state").exists() {
            return Err(anyhow!("System sleep not supported"));
        }

        // Save device info for verification
        Path::new(device_path)
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| anyhow!("Invalid device path"))?;

        println!("Initiating system sleep/wake cycle...");

        // Write to /sys/power/state to trigger sleep
        // Note: This requires root and may need additional configuration
        let sleep_result = fs::write("/sys/power/state", b"mem");

        if sleep_result.is_err() {
            // Try standby if mem fails
            fs::write("/sys/power/state", b"standby")
                .map_err(|e| anyhow!("Failed to enter sleep state: {}", e))?;
        }

        // System will resume here after wake
        thread::sleep(Duration::from_secs(3));

        // Verify device is still accessible
        if !Path::new(device_path).exists() {
            return Err(anyhow!("Device not found after wake"));
        }

        Ok(())
    }

    /// Method 2: Hot-plug simulation via sysfs
    fn unfreeze_via_hotplug(device_path: &str) -> Result<()> {
        let device_name = Path::new(device_path)
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| anyhow!("Invalid device path"))?;

        // Find the SCSI host for this device
        let host_path = Self::find_scsi_host(device_name)?;

        println!("Simulating hot-unplug/replug for {}...", device_name);

        // Offline the device
        let offline_path = format!("/sys/block/{}/device/state", device_name);
        if Path::new(&offline_path).exists() {
            fs::write(&offline_path, b"offline")
                .map_err(|e| anyhow!("Failed to offline device: {}", e))?;

            thread::sleep(Duration::from_secs(2));

            // Online the device
            fs::write(&offline_path, b"running")
                .map_err(|e| anyhow!("Failed to online device: {}", e))?;
        }

        // Trigger SCSI rescan
        let scan_path = format!("{}/scan", host_path);
        if Path::new(&scan_path).exists() {
            fs::write(&scan_path, b"- - -")
                .map_err(|e| anyhow!("Failed to trigger SCSI rescan: {}", e))?;
        }

        thread::sleep(Duration::from_secs(3));

        Ok(())
    }

    /// Method 3: SATA link power management toggle
    fn unfreeze_via_link_power(device_path: &str) -> Result<()> {
        let device_name = Path::new(device_path)
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| anyhow!("Invalid device path"))?;

        // Find the ATA port for this device
        let ata_port = Self::find_ata_port(device_name)?;
        let link_pm_path = format!(
            "/sys/class/ata_port/{}/link_power_management_policy",
            ata_port
        );

        if !Path::new(&link_pm_path).exists() {
            return Err(anyhow!("Link power management not available"));
        }

        println!("Toggling SATA link power management...");

        // Read current policy
        let current_policy = fs::read_to_string(&link_pm_path)
            .map_err(|e| anyhow!("Failed to read link PM policy: {}", e))?;

        // Toggle between policies to trigger link reset
        let policies = vec!["max_performance", "medium_power", "min_power"];

        for policy in policies {
            if !current_policy.contains(policy) {
                fs::write(&link_pm_path, policy.as_bytes())
                    .map_err(|e| anyhow!("Failed to set link PM policy: {}", e))?;
                thread::sleep(Duration::from_millis(500));
            }
        }

        // Restore original policy
        fs::write(&link_pm_path, current_policy.trim().as_bytes())
            .map_err(|e| anyhow!("Failed to restore link PM policy: {}", e))?;

        thread::sleep(Duration::from_secs(2));

        Ok(())
    }

    /// Method 4: Power cycle via USB (if applicable)
    fn unfreeze_via_power_cycle(device_path: &str) -> Result<()> {
        let device_name = Path::new(device_path)
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| anyhow!("Invalid device path"))?;

        // Check if this is a USB device
        let usb_path = format!("/sys/block/{}/device", device_name);
        let real_path = fs::read_link(&usb_path)
            .map_err(|_| anyhow!("Not a USB device or cannot resolve path"))?;

        let path_str = real_path.to_string_lossy();

        if !path_str.contains("usb") {
            return Err(anyhow!("Not a USB device"));
        }

        println!("Power cycling USB device...");

        // Find USB authorize file
        let mut current_path = real_path.as_path();
        let mut authorize_path = None;

        while let Some(parent) = current_path.parent() {
            let test_path = parent.join("authorized");
            if test_path.exists() {
                authorize_path = Some(test_path);
                break;
            }
            current_path = parent;
        }

        let authorize_path =
            authorize_path.ok_or_else(|| anyhow!("Cannot find USB authorize control"))?;

        // Power cycle
        fs::write(&authorize_path, b"0")
            .map_err(|e| anyhow!("Failed to deauthorize USB: {}", e))?;

        thread::sleep(Duration::from_secs(3));

        fs::write(&authorize_path, b"1")
            .map_err(|e| anyhow!("Failed to reauthorize USB: {}", e))?;

        thread::sleep(Duration::from_secs(5));

        // Verify device reappeared
        if !Path::new(device_path).exists() {
            return Err(anyhow!("Device did not reappear after power cycle"));
        }

        Ok(())
    }

    /// Find SCSI host for a device
    fn find_scsi_host(device_name: &str) -> Result<String> {
        let device_path = format!("/sys/block/{}/device", device_name);
        let real_path = fs::read_link(&device_path)
            .map_err(|e| anyhow!("Failed to resolve device path: {}", e))?;

        let path_str = real_path.to_string_lossy();

        // Extract host number from path like ../../devices/pci0000:00/0000:00:1f.2/ata1/host0/...
        if let Some(host_match) = path_str.find("host") {
            let host_part = &path_str[host_match..];
            if let Some(end) = host_part.find('/') {
                let host_num = &host_part[4..end]; // Skip "host"
                return Ok(format!("/sys/class/scsi_host/host{}", host_num));
            }
        }

        Err(anyhow!("Could not find SCSI host"))
    }

    /// Find ATA port for a device
    fn find_ata_port(device_name: &str) -> Result<String> {
        let device_path = format!("/sys/block/{}/device", device_name);
        let real_path = fs::read_link(&device_path)
            .map_err(|e| anyhow!("Failed to resolve device path: {}", e))?;

        let path_str = real_path.to_string_lossy();

        // Extract ata port from path
        if let Some(ata_match) = path_str.find("ata") {
            let ata_part = &path_str[ata_match..];
            if let Some(end) = ata_part.find('/') {
                return Ok(ata_part[..end].to_string());
            }
        }

        Err(anyhow!("Could not find ATA port"))
    }

    /// Check if secure erase is blocked due to freeze
    pub fn is_secure_erase_blocked(device_path: &str) -> DriveResult<bool> {
        let status = Self::get_freeze_status(device_path)?;
        Ok(matches!(
            status,
            FreezeStatus::Frozen | FreezeStatus::FrozenByBIOS
        ))
    }
}
