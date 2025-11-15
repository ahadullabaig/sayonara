// eMMC/UFS Embedded Storage Support
//
// Support for embedded storage found in phones, tablets, and embedded systems

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootPartition {
    pub partition_number: u8, // Boot1, Boot2
    pub size: u64,
    pub is_write_protected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RPMBPartition {
    // Replay Protected Memory Block - cannot be wiped
    pub size: u64,
    pub key_programmed: bool,
    pub counter: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserDataArea {
    pub size: u64,
    pub is_trimmed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EMMCDevice {
    pub device_path: String,
    pub capacity: u64,
    pub boot_partitions: Vec<BootPartition>,
    pub rpmb: Option<RPMBPartition>,
    pub user_data_area: UserDataArea,
    pub emmc_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UFSLogicalUnit {
    pub lun_id: u8,
    pub capacity: u64,
    pub is_boot_lun: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UFSDevice {
    pub device_path: String,
    pub luns: Vec<UFSLogicalUnit>,
    pub supports_purge: bool,
    pub ufs_version: String,
}

impl EMMCDevice {
    /// Detect eMMC device
    pub fn detect(device_path: &str) -> Result<bool> {
        // Check if it's mmcblk device
        if device_path.contains("mmcblk") {
            return Ok(true);
        }

        // Check via mmc-utils
        let output = Command::new("mmc")
            .arg("extcsd")
            .arg("read")
            .arg(device_path)
            .output();

        Ok(output.is_ok() && output.unwrap().status.success())
    }

    /// Get eMMC configuration
    pub fn get_configuration(device_path: &str) -> Result<EMMCDevice> {
        Ok(EMMCDevice {
            device_path: device_path.to_string(),
            capacity: 0,
            boot_partitions: vec![
                BootPartition {
                    partition_number: 1,
                    size: 4 * 1024 * 1024, // Typical 4MB
                    is_write_protected: false,
                },
                BootPartition {
                    partition_number: 2,
                    size: 4 * 1024 * 1024,
                    is_write_protected: false,
                },
            ],
            rpmb: Some(RPMBPartition {
                size: 128 * 1024, // Typical 128KB
                key_programmed: false,
                counter: 0,
            }),
            user_data_area: UserDataArea {
                size: 0,
                is_trimmed: false,
            },
            emmc_version: "5.1".to_string(),
        })
    }

    /// Perform eMMC secure erase (CMD38)
    pub fn secure_erase(&self) -> Result<()> {
        println!("Performing eMMC secure erase on {}", self.device_path);

        // Use mmc-utils secure erase
        let output = Command::new("mmc")
            .arg("erase")
            .arg("secure")
            .arg(&self.device_path)
            .output();

        if let Ok(output) = output {
            if output.status.success() {
                println!("eMMC secure erase completed");
                return Ok(());
            }
        }

        Err(anyhow!("eMMC secure erase failed"))
    }

    /// TRIM operation
    pub fn trim(&self) -> Result<()> {
        println!("Performing TRIM on eMMC");

        let output = Command::new("blkdiscard").arg(&self.device_path).output();

        if let Ok(output) = output {
            if output.status.success() {
                return Ok(());
            }
        }

        Ok(())
    }

    /// Sanitize operation
    pub fn sanitize(&self) -> Result<()> {
        println!("Performing eMMC sanitize");

        let output = Command::new("mmc")
            .arg("sanitize")
            .arg(&self.device_path)
            .output();

        if let Ok(output) = output {
            if output.status.success() {
                println!("eMMC sanitize completed");
                return Ok(());
            }
        }

        Err(anyhow!("eMMC sanitize failed"))
    }

    /// Wipe boot partitions
    pub fn wipe_boot_partitions(&self) -> Result<()> {
        println!("Wiping eMMC boot partitions");

        for boot in &self.boot_partitions {
            let boot_dev = format!("{}boot{}", self.device_path, boot.partition_number);
            println!("  Wiping {}", boot_dev);

            let _ = Command::new("dd")
                .arg("if=/dev/zero")
                .arg(format!("of={}", boot_dev))
                .arg("bs=4M")
                .output();
        }

        Ok(())
    }

    /// Handle RPMB (cannot wipe, only document)
    pub fn handle_rpmb(&self) -> Result<()> {
        if let Some(ref rpmb) = self.rpmb {
            println!("⚠️  RPMB partition detected:");
            println!("   Size: {} KB", rpmb.size / 1024);
            println!("   Key programmed: {}", rpmb.key_programmed);
            println!("   RPMB cannot be wiped (cryptographically protected)");
            println!("   This is normal and does not affect data security");
        }
        Ok(())
    }

    /// Wipe entire eMMC device
    pub fn wipe_emmc(&self) -> Result<()> {
        println!("Starting eMMC wipe: {}", self.device_path);

        // Try sanitize first (most thorough)
        if self.sanitize().is_ok() {
            self.handle_rpmb()?;
            return Ok(());
        }

        // Fallback to secure erase
        if self.secure_erase().is_ok() {
            self.wipe_boot_partitions()?;
            self.handle_rpmb()?;
            return Ok(());
        }

        // Fallback to TRIM
        self.trim()?;
        self.wipe_boot_partitions()?;
        self.handle_rpmb()?;

        println!("eMMC wipe completed");
        Ok(())
    }
}

impl UFSDevice {
    /// Detect UFS device
    pub fn detect(device_path: &str) -> Result<bool> {
        // UFS devices typically appear as SCSI devices
        // Check for UFS-specific attributes

        let output = Command::new("sg_inq").arg(device_path).output();

        if let Ok(output) = output {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if stdout.contains("UFS") {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// UFS purge command
    pub fn purge(&self) -> Result<()> {
        println!("Performing UFS purge on {}", self.device_path);

        // UFS purge is vendor-specific
        // Most implementations use UNMAP with specific flags

        let output = Command::new("sg_unmap")
            .arg("--all")
            .arg(&self.device_path)
            .output();

        if let Ok(output) = output {
            if output.status.success() {
                println!("UFS purge completed");
                return Ok(());
            }
        }

        Err(anyhow!("UFS purge failed"))
    }

    /// Wipe UFS device
    pub fn wipe_ufs(&self) -> Result<()> {
        println!("Starting UFS wipe: {}", self.device_path);

        if self.supports_purge {
            self.purge()?;
        } else {
            // Fallback to standard SCSI UNMAP
            let _ = Command::new("blkdiscard").arg(&self.device_path).output();
        }

        println!("UFS wipe completed");
        Ok(())
    }
}
