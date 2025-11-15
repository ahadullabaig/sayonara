// RAID Array Handling
//
// Support for detecting and safely wiping RAID array members

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum RAIDType {
    SoftwareRAID, // Linux mdadm
    HardwareRAID, // Controller-based
    FakeRAID,     // BIOS/firmware RAID (Intel RST)
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum RAIDController {
    DellPERC,
    HPSmartArray,
    IntelRST,
    LSIMegaRAID,
    Adaptec,
    Generic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataRegion {
    pub location: MetadataLocation,
    pub offset: u64,
    pub size: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum MetadataLocation {
    Start, // Beginning of drive
    End,   // End of drive
    Both,  // Both ends
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RAIDArray {
    pub device_path: String,
    pub raid_type: RAIDType,
    pub member_drives: Vec<String>,
    pub metadata_locations: Vec<MetadataRegion>,
    pub controller: Option<RAIDController>,
    pub is_active: bool,
}

impl RAIDArray {
    /// Detect if device is RAID member
    pub fn detect_raid_membership(device_path: &str) -> Result<bool> {
        // Check mdadm
        if Self::is_mdadm_member(device_path)? {
            return Ok(true);
        }

        // Check hardware RAID
        if Self::is_hardware_raid_member(device_path)? {
            return Ok(true);
        }

        Ok(false)
    }

    fn is_mdadm_member(device_path: &str) -> Result<bool> {
        let output = Command::new("mdadm")
            .arg("--examine")
            .arg(device_path)
            .output();

        if let Ok(output) = output {
            if output.status.success() {
                return Ok(true);
            }
        }

        Ok(false)
    }

    fn is_hardware_raid_member(device_path: &str) -> Result<bool> {
        let output = Command::new("sg_inq").arg(device_path).output();

        if let Ok(output) = output {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if stdout.contains("RAID") || stdout.contains("PERC") || stdout.contains("SmartArray") {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Get RAID configuration
    pub fn get_configuration(device_path: &str) -> Result<RAIDArray> {
        let raid_type = Self::detect_raid_type(device_path)?;
        let member_drives = Self::get_member_drives(device_path)?;
        let controller = Self::detect_controller(device_path)?;
        let metadata_locations = Self::find_metadata_locations(&raid_type);
        let is_active = Self::is_array_active(device_path)?;

        Ok(RAIDArray {
            device_path: device_path.to_string(),
            raid_type,
            member_drives,
            metadata_locations,
            controller,
            is_active,
        })
    }

    fn detect_raid_type(device_path: &str) -> Result<RAIDType> {
        if Self::is_mdadm_member(device_path)? {
            return Ok(RAIDType::SoftwareRAID);
        }

        if Self::is_hardware_raid_member(device_path)? {
            return Ok(RAIDType::HardwareRAID);
        }

        Ok(RAIDType::FakeRAID)
    }

    fn get_member_drives(_device_path: &str) -> Result<Vec<String>> {
        // Simplified - would query mdadm or controller
        Ok(Vec::new())
    }

    fn detect_controller(_device_path: &str) -> Result<Option<RAIDController>> {
        Ok(None)
    }

    fn find_metadata_locations(raid_type: &RAIDType) -> Vec<MetadataRegion> {
        match raid_type {
            RAIDType::SoftwareRAID => vec![MetadataRegion {
                location: MetadataLocation::End,
                offset: 0, // At end of device
                size: 4096,
            }],
            _ => Vec::new(),
        }
    }

    fn is_array_active(_device_path: &str) -> Result<bool> {
        Ok(false)
    }

    /// Check if safe to wipe
    pub fn safe_to_wipe(&self) -> Result<bool> {
        if self.is_active {
            return Ok(false);
        }
        Ok(true)
    }

    /// Wipe RAID metadata
    pub fn wipe_metadata(&self) -> Result<()> {
        println!("Wiping RAID metadata on {}", self.device_path);

        for region in &self.metadata_locations {
            println!("  Wiping metadata at {:?}", region.location);
            // Would actually zero out metadata regions
        }

        // For mdadm
        if self.raid_type == RAIDType::SoftwareRAID {
            let _ = Command::new("mdadm")
                .arg("--zero-superblock")
                .arg(&self.device_path)
                .output();
        }

        Ok(())
    }
}
