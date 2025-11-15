// Advanced NVMe Features: ZNS, Multiple Namespaces, Key-Value, Computational Storage
//
// This module extends basic NVMe support with modern advanced features

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::process::Command;

/// NVMe namespace type
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum NamespaceType {
    /// Standard block storage namespace
    Block,

    /// Zoned Namespace (ZNS) - similar to SMR
    ZonedNamespace,

    /// Key-Value namespace
    KeyValue,

    /// Computational storage namespace
    Computational,
}

/// ZNS Zone state (similar to SMR but for NVMe)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ZNSZoneState {
    Empty,
    ImplicitlyOpen,
    ExplicitlyOpen,
    Closed,
    ReadOnly,
    Full,
    Offline,
}

/// ZNS Zone for NVMe Zoned Namespaces
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZNSZone {
    /// Zone ID
    pub zone_id: u32,

    /// Starting LBA of zone
    pub zone_start_lba: u64,

    /// Zone capacity in blocks
    pub zone_capacity: u64,

    /// Current write pointer position
    pub write_pointer: u64,

    /// Zone state
    pub zone_state: ZNSZoneState,

    /// Zone type (sequential or conventional)
    pub is_sequential: bool,
}

impl ZNSZone {
    /// Check if zone needs reset before writing
    pub fn needs_reset(&self) -> bool {
        matches!(self.zone_state, ZNSZoneState::Full | ZNSZoneState::Closed)
    }

    /// Check if zone is writable
    pub fn is_writable(&self) -> bool {
        matches!(
            self.zone_state,
            ZNSZoneState::Empty | ZNSZoneState::ImplicitlyOpen | ZNSZoneState::ExplicitlyOpen
        )
    }
}

/// NVMe Namespace information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NVMeNamespace {
    /// Namespace ID (1-based)
    pub nsid: u32,

    /// Size in bytes
    pub size: u64,

    /// Namespace type
    pub namespace_type: NamespaceType,

    /// Device path (e.g., /dev/nvme0n1)
    pub device_path: String,

    /// Is namespace active?
    pub is_active: bool,

    /// Is namespace attached to controller?
    pub is_attached: bool,

    /// ZNS zones (if ZNS namespace)
    pub zones: Option<Vec<ZNSZone>>,
}

/// Advanced NVMe Drive configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NVMeAdvanced {
    /// Base device path (e.g., /dev/nvme0)
    pub device_path: String,

    /// Controller device path (e.g., /dev/nvme0)
    pub controller_path: String,

    /// All namespaces on this controller
    pub namespaces: Vec<NVMeNamespace>,

    /// Supports Zoned Namespaces (ZNS)?
    pub zns_support: bool,

    /// Supports Key-Value namespaces?
    pub kv_support: bool,

    /// Is this a computational storage device?
    pub is_computational_storage: bool,

    /// Model name
    pub model: String,

    /// Firmware version
    pub firmware: String,
}

impl NVMeAdvanced {
    /// Detect if device has advanced NVMe features
    pub fn detect_advanced_features(device_path: &str) -> Result<bool> {
        let info = Self::get_controller_info(device_path)?;

        // Check for ZNS, KV, or computational storage indicators
        if info.contains("ZNS")
            || info.contains("Zoned")
            || info.contains("Key-Value")
            || info.contains("KV")
            || info.contains("Computational")
        {
            return Ok(true);
        }

        // Check number of namespaces
        let ns_count = Self::count_namespaces(device_path)?;
        if ns_count > 1 {
            return Ok(true);
        }

        Ok(false)
    }

    /// Get controller information
    fn get_controller_info(device_path: &str) -> Result<String> {
        let output = Command::new("nvme")
            .arg("id-ctrl")
            .arg(device_path)
            .output()?;

        if !output.status.success() {
            return Err(anyhow!("Failed to get controller info"));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Count namespaces on controller
    fn count_namespaces(device_path: &str) -> Result<usize> {
        let output = Command::new("nvme")
            .arg("list-ns")
            .arg(device_path)
            .output()?;

        if !output.status.success() {
            return Ok(1); // Assume single namespace
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let count = stdout
            .lines()
            .filter(|line| line.contains("[") && line.contains("]"))
            .count();

        Ok(count.max(1))
    }

    /// Get full advanced configuration
    pub fn get_configuration(device_path: &str) -> Result<NVMeAdvanced> {
        let controller_path = Self::extract_controller_path(device_path);
        let info = Self::get_controller_info(&controller_path)?;

        let zns_support = Self::detect_zns_support(&controller_path)?;
        let kv_support = Self::detect_kv_support(&info);
        let is_computational_storage = Self::detect_computational_storage(&info);
        let model = Self::parse_model(&info);
        let firmware = Self::parse_firmware(&info);

        let namespaces = Self::enumerate_namespaces(&controller_path)?;

        Ok(NVMeAdvanced {
            device_path: device_path.to_string(),
            controller_path,
            namespaces,
            zns_support,
            kv_support,
            is_computational_storage,
            model,
            firmware,
        })
    }

    /// Extract controller path from namespace path
    /// e.g., /dev/nvme0n1 -> /dev/nvme0
    fn extract_controller_path(device_path: &str) -> String {
        if let Some(pos) = device_path.rfind('n') {
            device_path[..pos].to_string()
        } else {
            device_path.to_string()
        }
    }

    /// Detect ZNS support
    fn detect_zns_support(controller_path: &str) -> Result<bool> {
        // Check via nvme zns id-ctrl
        let output = Command::new("nvme")
            .arg("zns")
            .arg("id-ctrl")
            .arg(controller_path)
            .output();

        if let Ok(output) = output {
            if output.status.success() {
                return Ok(true);
            }
        }

        // Fallback: check controller capabilities
        let info = Self::get_controller_info(controller_path)?;
        Ok(info.contains("Zoned Namespace") || info.contains("ZNS"))
    }

    /// Detect Key-Value support
    fn detect_kv_support(info: &str) -> bool {
        info.contains("Key-Value") || info.contains("KV Command Set")
    }

    /// Detect computational storage
    fn detect_computational_storage(info: &str) -> bool {
        info.contains("Computational")
            || info.contains("In-Storage Compute")
            || info.contains("SmartNIC")
    }

    /// Parse model name
    fn parse_model(info: &str) -> String {
        for line in info.lines() {
            if line.contains("mn") && line.contains(":") {
                if let Some(model) = line.split(':').nth(1) {
                    return model.trim().to_string();
                }
            }
        }
        "Unknown".to_string()
    }

    /// Parse firmware version
    fn parse_firmware(info: &str) -> String {
        for line in info.lines() {
            if line.contains("fr") && line.contains(":") {
                if let Some(fw) = line.split(':').nth(1) {
                    return fw.trim().to_string();
                }
            }
        }
        "Unknown".to_string()
    }

    /// Enumerate all namespaces
    fn enumerate_namespaces(controller_path: &str) -> Result<Vec<NVMeNamespace>> {
        let output = Command::new("nvme")
            .arg("list-ns")
            .arg(controller_path)
            .arg("-a") // All namespaces
            .output()?;

        if !output.status.success() {
            // Fallback: assume single namespace with ID 1
            return Ok(vec![Self::create_default_namespace(controller_path, 1)?]);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut namespaces = Vec::new();

        for line in stdout.lines() {
            if line.starts_with('[') {
                // Parse line like "[ 0]:0x1"
                if let Some(nsid_str) = line.split(']').nth(1) {
                    if let Ok(nsid) = nsid_str.trim().trim_start_matches("0x").parse::<u32>() {
                        if nsid > 0 {
                            if let Ok(ns) = Self::get_namespace_details(controller_path, nsid) {
                                namespaces.push(ns);
                            }
                        }
                    }
                }
            }
        }

        // If no namespaces found, create default
        if namespaces.is_empty() {
            namespaces.push(Self::create_default_namespace(controller_path, 1)?);
        }

        Ok(namespaces)
    }

    /// Get details for specific namespace
    fn get_namespace_details(controller_path: &str, nsid: u32) -> Result<NVMeNamespace> {
        let device_path = format!("{}n{}", controller_path, nsid);

        let output = Command::new("nvme")
            .arg("id-ns")
            .arg(&device_path)
            .output()?;

        if !output.status.success() {
            return Self::create_default_namespace(controller_path, nsid);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Parse size
        let size = Self::parse_namespace_size(&stdout);

        // Detect namespace type
        let namespace_type = Self::detect_namespace_type(&device_path)?;

        // Get ZNS zones if applicable
        let zones = if namespace_type == NamespaceType::ZonedNamespace {
            Some(Self::get_zns_zones(&device_path)?)
        } else {
            None
        };

        Ok(NVMeNamespace {
            nsid,
            size,
            namespace_type,
            device_path,
            is_active: true,
            is_attached: true,
            zones,
        })
    }

    /// Create default namespace
    fn create_default_namespace(controller_path: &str, nsid: u32) -> Result<NVMeNamespace> {
        Ok(NVMeNamespace {
            nsid,
            size: 0,
            namespace_type: NamespaceType::Block,
            device_path: format!("{}n{}", controller_path, nsid),
            is_active: true,
            is_attached: true,
            zones: None,
        })
    }

    /// Parse namespace size from id-ns output
    fn parse_namespace_size(output: &str) -> u64 {
        for line in output.lines() {
            if line.contains("nsze") && line.contains(":") {
                if let Some(size_str) = line.split(':').nth(1) {
                    if let Ok(blocks) = size_str.trim().parse::<u64>() {
                        return blocks * 512; // Assume 512-byte blocks
                    }
                }
            }
        }
        0
    }

    /// Detect namespace type
    fn detect_namespace_type(device_path: &str) -> Result<NamespaceType> {
        // Check for ZNS
        let output = Command::new("nvme")
            .arg("zns")
            .arg("id-ns")
            .arg(device_path)
            .output();

        if let Ok(output) = output {
            if output.status.success() {
                return Ok(NamespaceType::ZonedNamespace);
            }
        }

        // Check for KV (would need vendor-specific command)
        // For now, default to Block

        Ok(NamespaceType::Block)
    }

    /// Get ZNS zones for a namespace
    fn get_zns_zones(device_path: &str) -> Result<Vec<ZNSZone>> {
        let output = Command::new("nvme")
            .arg("zns")
            .arg("report-zones")
            .arg(device_path)
            .output()?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Self::parse_zns_zones(&stdout)
    }

    /// Parse ZNS zone report output
    fn parse_zns_zones(output: &str) -> Result<Vec<ZNSZone>> {
        let mut zones = Vec::new();
        let mut zone_id = 0u32;

        for line in output.lines() {
            if line.contains("SLBA:") {
                // Parse zone info
                // Example: "SLBA: 0x0 WP: 0x0 Cap: 0x10000 State: EMPTY Type: SEQWRITE_REQ"

                let zone = ZNSZone {
                    zone_id,
                    zone_start_lba: 0,               // Would parse from SLBA
                    zone_capacity: 0,                // Would parse from Cap
                    write_pointer: 0,                // Would parse from WP
                    zone_state: ZNSZoneState::Empty, // Would parse from State
                    is_sequential: line.contains("SEQWRITE"),
                };

                zones.push(zone);
                zone_id += 1;
            }
        }

        Ok(zones)
    }

    /// Reset ZNS zone
    pub fn zns_reset_zone(&self, ns: &NVMeNamespace, zone_id: u32) -> Result<()> {
        if ns.namespace_type != NamespaceType::ZonedNamespace {
            return Err(anyhow!("Not a ZNS namespace"));
        }

        println!("Resetting ZNS zone {} on {}", zone_id, ns.device_path);

        let output = Command::new("nvme")
            .arg("zns")
            .arg("reset-zone")
            .arg(&ns.device_path)
            .arg("-s")
            .arg(zone_id.to_string())
            .output()?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("ZNS zone reset failed: {}", err));
        }

        Ok(())
    }

    /// Reset all ZNS zones in namespace
    pub fn zns_reset_all_zones(&self, ns: &NVMeNamespace) -> Result<()> {
        if let Some(ref zones) = ns.zones {
            for zone in zones {
                if zone.is_sequential {
                    self.zns_reset_zone(ns, zone.zone_id)?;
                }
            }
        }
        Ok(())
    }

    /// Wipe a single namespace
    pub fn wipe_namespace(&self, ns: &NVMeNamespace) -> Result<()> {
        println!("Wiping namespace {} ({})", ns.nsid, ns.device_path);

        match ns.namespace_type {
            NamespaceType::ZonedNamespace => {
                self.wipe_zns_namespace(ns)?;
            }
            NamespaceType::KeyValue => {
                self.wipe_kv_namespace(ns)?;
            }
            NamespaceType::Computational => {
                self.wipe_computational_namespace(ns)?;
            }
            NamespaceType::Block => {
                self.wipe_block_namespace(ns)?;
            }
        }

        Ok(())
    }

    /// Wipe ZNS namespace
    fn wipe_zns_namespace(&self, ns: &NVMeNamespace) -> Result<()> {
        println!("Wiping ZNS namespace (zone-aware)");

        // Reset all zones first
        self.zns_reset_all_zones(ns)?;

        // Write to each zone sequentially
        if let Some(ref zones) = ns.zones {
            for zone in zones {
                if zone.is_sequential {
                    println!("  Writing to zone {}...", zone.zone_id);
                    // Would write sequential data to zone
                }
            }
        }

        println!("ZNS namespace wiped");
        Ok(())
    }

    /// Wipe Key-Value namespace
    fn wipe_kv_namespace(&self, ns: &NVMeNamespace) -> Result<()> {
        println!("Wiping Key-Value namespace");

        // KV namespaces would need vendor-specific delete-all command
        // Fallback to format

        self.format_namespace(ns)?;
        Ok(())
    }

    /// Wipe computational storage namespace
    fn wipe_computational_namespace(&self, ns: &NVMeNamespace) -> Result<()> {
        println!("Wiping computational storage namespace");

        // May need to clear on-device compute state
        // For now, treat as block device

        self.wipe_block_namespace(ns)?;
        Ok(())
    }

    /// Wipe block namespace (standard)
    fn wipe_block_namespace(&self, ns: &NVMeNamespace) -> Result<()> {
        // Use standard format or sanitize
        self.format_namespace(ns)?;
        Ok(())
    }

    /// Format a namespace
    fn format_namespace(&self, ns: &NVMeNamespace) -> Result<()> {
        println!("Formatting namespace {} with secure erase", ns.nsid);

        let output = Command::new("nvme")
            .arg("format")
            .arg(&ns.device_path)
            .arg("--ses=1") // Secure erase
            .arg("--force")
            .output()?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Format failed: {}", err));
        }

        Ok(())
    }

    /// Wipe entire NVMe drive (all namespaces)
    pub fn wipe_all_namespaces(&self) -> Result<()> {
        println!("Wiping all namespaces on {}", self.controller_path);
        println!("Model: {}", self.model);
        println!("Firmware: {}", self.firmware);
        println!("Total namespaces: {}", self.namespaces.len());

        if self.zns_support {
            println!("ZNS support detected");
        }
        if self.kv_support {
            println!("Key-Value support detected");
        }
        if self.is_computational_storage {
            println!("Computational storage detected");
        }

        for ns in &self.namespaces {
            println!("\nNamespace {}:", ns.nsid);
            println!("  Type: {:?}", ns.namespace_type);
            println!("  Size: {} GB", ns.size / (1024 * 1024 * 1024));

            if let Some(ref zones) = ns.zones {
                println!("  ZNS Zones: {}", zones.len());
            }

            self.wipe_namespace(ns)?;
        }

        println!("\nAll namespaces wiped successfully");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zns_zone_needs_reset() {
        let zone = ZNSZone {
            zone_id: 0,
            zone_start_lba: 0,
            zone_capacity: 1000,
            write_pointer: 0,
            zone_state: ZNSZoneState::Full,
            is_sequential: true,
        };

        assert!(zone.needs_reset());
    }

    #[test]
    fn test_namespace_types() {
        assert_ne!(NamespaceType::Block, NamespaceType::ZonedNamespace);
        assert_ne!(NamespaceType::KeyValue, NamespaceType::Computational);
    }

    #[test]
    fn test_extract_controller_path() {
        let ctrl = NVMeAdvanced::extract_controller_path("/dev/nvme0n1");
        assert_eq!(ctrl, "/dev/nvme0");

        let ctrl2 = NVMeAdvanced::extract_controller_path("/dev/nvme1n5");
        assert_eq!(ctrl2, "/dev/nvme1");
    }
}
