/// Mock command execution infrastructure for testing
///
/// This module provides utilities for mocking external command execution
/// (smartctl, hdparm, blockdev, etc.) without actually running them.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Mock command output
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct MockCommandOutput {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub success: bool,
}

impl MockCommandOutput {
    pub fn success(stdout: &str) -> Self {
        Self {
            stdout: stdout.as_bytes().to_vec(),
            stderr: Vec::new(),
            success: true,
        }
    }

    #[allow(dead_code)]
    pub fn failure(stderr: &str) -> Self {
        Self {
            stdout: Vec::new(),
            stderr: stderr.as_bytes().to_vec(),
            success: false,
        }
    }
}

/// Mock command registry
pub struct MockCommandRegistry {
    commands: Arc<Mutex<HashMap<String, MockCommandOutput>>>,
}

impl MockCommandRegistry {
    pub fn new() -> Self {
        Self {
            commands: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Register a mock command response
    pub fn register(&mut self, command_key: &str, output: MockCommandOutput) {
        self.commands.lock().unwrap().insert(command_key.to_string(), output);
    }

    /// Get mock output for a command
    pub fn get(&self, command_key: &str) -> Option<MockCommandOutput> {
        self.commands.lock().unwrap().get(command_key).cloned()
    }

    /// Clear all registered commands
    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.commands.lock().unwrap().clear();
    }
}

impl Default for MockCommandRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Mock smartctl output for various drive types
pub struct MockSmartctlData;

impl MockSmartctlData {
    /// HDD smartctl output
    pub fn hdd_output(_device: &str, model: &str, serial: &str, size_gb: u64) -> String {
        format!(
            r#"smartctl 7.2 2020-12-30 r5155 [x86_64-linux-5.10.0] (local build)
Copyright (C) 2002-20, Bruce Allen, Christian Franke, www.smartmontools.org

=== START OF INFORMATION SECTION ===
Device Model:     {}
Serial Number:    {}
LU WWN Device Id: 5 000c50 0a1b2c3d4
Firmware Version: CC45
User Capacity:    {} bytes [{} GB]
Sector Size:      512 bytes logical/physical
Rotation Rate:    7200 rpm
Form Factor:      3.5 inches
Device is:        Not in smartctl database
ATA Version is:   ACS-3 T13/2161-D revision 5
SATA Version is:  SATA 3.2, 6.0 Gb/s (current: 6.0 Gb/s)
Local Time is:    Thu Oct 30 12:00:00 2025 UTC
SMART support is: Available - device has SMART capability.
SMART support is: Enabled"#,
            model, serial, size_gb * 1024 * 1024 * 1024, size_gb
        )
    }

    /// SSD smartctl output
    pub fn ssd_output(_device: &str, model: &str, serial: &str, size_gb: u64) -> String {
        format!(
            r#"smartctl 7.2 2020-12-30 r5155 [x86_64-linux-5.10.0] (local build)
Copyright (C) 2002-20, Bruce Allen, Christian Franke, www.smartmontools.org

=== START OF INFORMATION SECTION ===
Device Model:     {}
Serial Number:    {}
LU WWN Device Id: 5 002538 e402345678
Firmware Version: 2.0.0
User Capacity:    {} bytes [{} GB]
Sector Size:      512 bytes logical/physical
Rotation Rate:    Solid State Device
Form Factor:      2.5 inches
TRIM Command:     Available
Device is:        Not in smartctl database
ATA Version is:   ACS-4 T13/BSR INCITS 529 revision 5
SATA Version is:  SATA 3.3, 6.0 Gb/s (current: 6.0 Gb/s)
Local Time is:    Thu Oct 30 12:00:00 2025 UTC
SMART support is: Available - device has SMART capability.
SMART support is: Enabled"#,
            model, serial, size_gb * 1024 * 1024 * 1024, size_gb
        )
    }

    /// NVMe smartctl output
    pub fn nvme_output(_device: &str, model: &str, serial: &str, size_gb: u64) -> String {
        format!(
            r#"smartctl 7.2 2020-12-30 r5155 [x86_64-linux-5.10.0] (local build)
Copyright (C) 2002-20, Bruce Allen, Christian Franke, www.smartmontools.org

=== START OF INFORMATION SECTION ===
Model Number:     {}
Serial Number:    {}
Firmware Version: 1.0.0.0
PCI Vendor/Subsystem ID: 0x144d
IEEE OUI Identifier: 0x002538
Total NVM Capacity: {} [{} GB]
Unallocated NVM Capacity: 0
Controller ID: 1
NVMe Version: 1.4
Number of Namespaces: 1
Namespace 1 Size/Capacity: {} [{} GB]
Namespace 1 Formatted LBA Size: 512
Namespace 1 IEEE EUI-64: 002538 0123456789
Local Time is:    Thu Oct 30 12:00:00 2025 UTC"#,
            model, serial,
            size_gb * 1024 * 1024 * 1024, size_gb,
            size_gb * 1024 * 1024 * 1024, size_gb
        )
    }

    /// USB drive smartctl output
    #[allow(dead_code)]
    pub fn usb_output(model: &str, serial: &str, size_gb: u64) -> String {
        format!(
            r#"smartctl 7.2 2020-12-30 r5155 [x86_64-linux-5.10.0] (local build)
Copyright (C) 2002-20, Bruce Allen, Christian Franke, www.smartmontools.org

=== START OF INFORMATION SECTION ===
Device Model:     {}
Serial Number:    {}
Firmware Version: 1.00
User Capacity:    {} bytes [{} GB]
Sector Size:      512 bytes logical/physical
Rotation Rate:    Solid State Device
Device is:        Not in smartctl database
ATA Version is:   ACS-2 T13/2015-D revision 3
Local Time is:    Thu Oct 30 12:00:00 2025 UTC"#,
            model, serial, size_gb * 1024 * 1024 * 1024, size_gb
        )
    }
}

/// Mock hdparm output
pub struct MockHdparmData;

impl MockHdparmData {
    /// hdparm -I output with secure erase support
    pub fn secure_erase_supported() -> String {
        r#"/dev/sda:

ATA device, with non-removable media
	Model Number:       WDC WD10EZEX-08WN4A0
	Serial Number:      WD-WCC6Y0123456
	Firmware Revision:  01.01A01
	Transport:          Serial, SATA 1.0a, SATA II Extensions, SATA Rev 2.5, SATA Rev 2.6, SATA Rev 3.0
Capabilities:
	LBA, IORDY(can be disabled)
	Queue depth: 32
	Standby timer values: spec'd by Standard, with device specific minimum
	R/W multiple sector transfer: Max = 16	Current = 16
	Advanced power management level: 128
	DMA: mdma0 mdma1 mdma2 udma0 udma1 udma2 udma3 udma4 udma5 *udma6
	     Cycle time: min=120ns recommended=120ns
	PIO: pio0 pio1 pio2 pio3 pio4
	     Cycle time: no flow control=120ns  IORDY flow control=120ns
Commands/features:
	Enabled	Supported:
	   *	SMART feature set
	   *	Security Mode feature set
	   *		SECURITY ERASE UNIT
	        supported: enhanced erase
	   *	Power Management feature set
	   *	Write cache
	   *	WRITE_BUFFER command
	   *	READ_BUFFER command"#.to_string()
    }

    /// hdparm -N output with HPA
    pub fn hpa_detected(current_sectors: u64, native_sectors: u64) -> String {
        format!(
            r#"/dev/sda:
 max sectors   = {}/{}(HPA is enabled)
 HPA is enabled"#,
            current_sectors, native_sectors
        )
    }

    /// hdparm -N output without HPA
    pub fn no_hpa(sectors: u64) -> String {
        format!(
            r#"/dev/sda:
 max sectors   = {}/{}, HPA is disabled"#,
            sectors, sectors
        )
    }

    /// hdparm --dco-identify output with DCO
    #[allow(dead_code)]
    pub fn dco_detected(dco_max: u64, real_max: u64) -> String {
        format!(
            r#"/dev/sda:
DCO Revision: 1
Real max sectors: {}
DCO max sectors: {}
DCO is active"#,
            real_max, dco_max
        )
    }

    /// hdparm --dco-identify output without DCO
    #[allow(dead_code)]
    pub fn no_dco() -> String {
        r#"/dev/sda:
DCO feature set not supported"#.to_string()
    }
}

/// Mock blockdev output
pub struct MockBlockdevData;

impl MockBlockdevData {
    /// blockdev --getsize64 output
    pub fn size_bytes(size_bytes: u64) -> String {
        format!("{}\n", size_bytes)
    }

    /// blockdev --getsz output (sectors)
    #[allow(dead_code)]
    pub fn size_sectors(sectors: u64) -> String {
        format!("{}\n", sectors)
    }
}

/// Mock /proc/mounts content
pub struct MockProcData;

impl MockProcData {
    /// /proc/mounts with system drive mounted at /
    pub fn system_drive_mounted(device: &str) -> String {
        format!(
            r#"{} / ext4 rw,relatime,errors=remount-ro 0 0
proc /proc proc rw,nosuid,nodev,noexec,relatime 0 0
sysfs /sys sysfs rw,nosuid,nodev,noexec,relatime 0 0
tmpfs /tmp tmpfs rw,nosuid,nodev,relatime 0 0"#,
            device
        )
    }

    /// /proc/mounts with device mounted at /mnt/data
    #[allow(dead_code)]
    pub fn data_drive_mounted(device: &str) -> String {
        format!(
            r#"/dev/sda1 / ext4 rw,relatime,errors=remount-ro 0 0
{} /mnt/data ext4 rw,relatime 0 0
proc /proc proc rw,nosuid,nodev,noexec,relatime 0 0"#,
            device
        )
    }

    /// /proc/mounts with no specific device
    #[allow(dead_code)]
    pub fn no_mount() -> String {
        r#"/dev/sda1 / ext4 rw,relatime,errors=remount-ro 0 0
proc /proc proc rw,nosuid,nodev,noexec,relatime 0 0
sysfs /sys sysfs rw,nosuid,nodev,noexec,relatime 0 0"#.to_string()
    }

    /// /proc/cmdline with root device
    pub fn cmdline_with_device(device: &str) -> String {
        format!(
            "BOOT_IMAGE=/boot/vmlinuz-5.10.0 root={} ro quiet splash",
            device
        )
    }
}

/// Mock NVMe command output
pub struct MockNvmeData;

impl MockNvmeData {
    /// nvme id-ctrl output with sanitize support
    pub fn sanitize_supported() -> String {
        r#"NVME Identify Controller:
vid       : 0x144d
ssvid     : 0x144d
sn        : S4EWNX0R123456
mn        : Samsung SSD 980 PRO 1TB
fr        : 3B2QGXA7
rab       : 2
ieee      : 002538
cmic      : 0
mdts      : 9
cntlid    : 1
ver       : 10400
rtd3r     : 500000
rtd3e     : 1000000
oaes      : 0x200
ctratt    : 0x2
sanicap   : 0x3
  Crypto Erase Supported: Yes
  Block Erase Supported: Yes
  Overwrite Supported: Yes
  Crypto Scramble Supported: No
sqes      : 0x66
cqes      : 0x44"#.to_string()
    }

    /// nvme id-ctrl output without sanitize support
    pub fn no_sanitize() -> String {
        r#"NVME Identify Controller:
vid       : 0x1987
ssvid     : 0x1987
sn        : AA000000000000000001
mn        : Basic NVMe SSD 512GB
fr        : 1.0.0
sanicap   : 0x0
  Crypto Erase Supported: No
  Block Erase Supported: No
  Overwrite Supported: No"#.to_string()
    }
}

/// Mock mdadm output
pub struct MockMdadmData;

impl MockMdadmData {
    /// mdadm --examine output for RAID member
    pub fn raid_member(raid_level: &str, array_uuid: &str) -> String {
        format!(
            r#"/dev/sdb:
          Magic : a92b4efc
        Version : 1.2
    Feature Map : 0x0
     Array UUID : {}
           Name : server:0
  Creation Time : Thu Oct 30 12:00:00 2025
     Raid Level : {}
   Raid Devices : 4

 Avail Dev Size : 1953382400 (931.26 GiB 1000.07 GB)
     Array Size : 2930043904 (2793.77 GiB 3000.21 GB)
  Used Dev Size : 1953382400 (931.26 GiB 1000.07 GB)
    Data Offset : 262144 sectors
   Super Offset : 8 sectors
   Unused Space : before=262056 sectors, after=0 sectors
          State : clean
    Device UUID : {}

    Update Time : Thu Oct 30 13:00:00 2025
  Bad Block Log : 512 entries available at offset 72 sectors
       Checksum : 12345678 - correct
         Events : 12345


   Device Role : Active device 0
   Array State : AAAA ('A' == active, '.' == missing, 'R' == replacing)"#,
            array_uuid, raid_level, array_uuid
        )
    }

    /// mdadm --examine output for non-RAID device
    pub fn not_raid() -> String {
        "mdadm: No md superblock detected on /dev/sda.".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_registry() {
        let mut registry = MockCommandRegistry::new();

        let output = MockCommandOutput::success("test output");
        registry.register("test_command", output.clone());

        let retrieved = registry.get("test_command").unwrap();
        assert_eq!(retrieved.stdout, b"test output");
        assert!(retrieved.success);
    }

    #[test]
    fn test_smartctl_data_generation() {
        let hdd = MockSmartctlData::hdd_output("/dev/sda", "WDC WD10EZEX", "WD-123456", 1000);
        assert!(hdd.contains("WDC WD10EZEX"));
        assert!(hdd.contains("7200 rpm"));
        assert!(hdd.contains("1000 GB"));

        let ssd = MockSmartctlData::ssd_output("/dev/sdb", "Samsung 870 EVO", "S123456", 500);
        assert!(ssd.contains("Samsung 870 EVO"));
        assert!(ssd.contains("Solid State Device"));
        assert!(ssd.contains("TRIM Command"));

        let nvme = MockSmartctlData::nvme_output("/dev/nvme0n1", "Samsung 980 PRO", "S123456", 1000);
        assert!(nvme.contains("Samsung 980 PRO"));
        assert!(nvme.contains("NVMe Version"));
    }

    #[test]
    fn test_hdparm_data() {
        let secure_erase = MockHdparmData::secure_erase_supported();
        assert!(secure_erase.contains("SECURITY ERASE UNIT"));
        assert!(secure_erase.contains("enhanced erase"));

        let hpa = MockHdparmData::hpa_detected(1000000, 1200000);
        assert!(hpa.contains("HPA is enabled"));
        assert!(hpa.contains("1000000/1200000"));

        let no_hpa = MockHdparmData::no_hpa(1000000);
        assert!(no_hpa.contains("HPA is disabled"));
    }

    #[test]
    fn test_blockdev_data() {
        let size = MockBlockdevData::size_bytes(1000000000);
        assert_eq!(size.trim(), "1000000000");
    }

    #[test]
    fn test_proc_data() {
        let mounts = MockProcData::system_drive_mounted("/dev/sda1");
        assert!(mounts.contains("/dev/sda1 / ext4"));

        let cmdline = MockProcData::cmdline_with_device("/dev/sda1");
        assert!(cmdline.contains("root=/dev/sda1"));
    }

    #[test]
    fn test_nvme_data() {
        let sanitize = MockNvmeData::sanitize_supported();
        assert!(sanitize.contains("Crypto Erase Supported: Yes"));
        assert!(sanitize.contains("Block Erase Supported: Yes"));

        let no_sanitize = MockNvmeData::no_sanitize();
        assert!(no_sanitize.contains("Crypto Erase Supported: No"));
    }

    #[test]
    fn test_mdadm_data() {
        let raid = MockMdadmData::raid_member("raid5", "12345678-1234-1234-1234-123456789abc");
        assert!(raid.contains("Raid Level : raid5"));
        assert!(raid.contains("Array UUID"));

        let not_raid = MockMdadmData::not_raid();
        assert!(not_raid.contains("No md superblock"));
    }
}
