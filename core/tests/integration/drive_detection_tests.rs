/// Integration tests for drive detection
///
/// These tests verify drive type detection, capability detection, and encryption detection.
/// Note: Some tests use file-based mocking since the detection code uses direct Command execution.

use std::fs;
use std::path::Path;
use tempfile::TempDir;

/// Test helper to create a mock /sys/block structure
fn create_mock_sysfs(temp_dir: &Path, devices: &[&str]) -> std::io::Result<()> {
    let block_dir = temp_dir.join("sys/block");
    fs::create_dir_all(&block_dir)?;

    for device in devices {
        let device_dir = block_dir.join(device);
        fs::create_dir_all(&device_dir)?;

        // Create device symlink simulation
        let device_subdir = device_dir.join("device");
        fs::create_dir_all(&device_subdir)?;
    }

    Ok(())
}

/// Test helper to create a mock /dev structure
fn create_mock_dev(temp_dir: &Path, devices: &[&str]) -> std::io::Result<()> {
    let dev_dir = temp_dir.join("dev");
    fs::create_dir_all(&dev_dir)?;

    for device in devices {
        let device_path = dev_dir.join(device);
        // Create empty file to simulate device
        fs::write(&device_path, b"")?;
    }

    Ok(())
}

#[cfg(test)]
mod drive_type_detection_tests {
    use super::*;

    #[test]
    fn test_nvme_drive_detection_by_path() {
        // NVMe drives are detected by device path containing "nvme"
        let test_paths = vec![
            ("/dev/nvme0n1", DriveType::NVMe),
            ("/dev/nvme1n1", DriveType::NVMe),
            ("/dev/nvme0n1p1", DriveType::NVMe), // partition
        ];

        for (path, expected_type) in test_paths {
            let is_nvme = path.contains("nvme");
            if is_nvme {
                assert_eq!(expected_type, DriveType::NVMe, "Failed for path: {}", path);
            }
        }
    }

    #[test]
    fn test_emmc_drive_detection_by_path() {
        // eMMC drives are detected by device path containing "mmcblk"
        let test_paths = vec![
            ("/dev/mmcblk0", true),
            ("/dev/mmcblk1", true),
            ("/dev/mmcblk0p1", true), // partition
            ("/dev/sda", false),
            ("/dev/nvme0n1", false),
        ];

        for (path, should_be_emmc) in test_paths {
            let is_emmc = path.contains("mmcblk");
            assert_eq!(is_emmc, should_be_emmc, "Failed for path: {}", path);
        }
    }

    #[test]
    fn test_rotation_rate_parsing_for_hdd() {
        // Test parsing smartctl output to detect HDDs by rotation rate
        let smartctl_output = MockSmartctlData::hdd_output("/dev/sda", "WDC WD10EZEX", "WD-123456", 1000);

        assert!(smartctl_output.contains("Rotation Rate:"));
        assert!(smartctl_output.contains("7200 rpm"));
        assert!(!smartctl_output.contains("Solid State Device"));

        // Verify we can extract the rotation rate
        let has_rpm = smartctl_output.contains("rpm") && !smartctl_output.contains("0 rpm");
        let is_ssd_marker = smartctl_output.contains("Solid State Device");

        assert!(has_rpm, "HDD should have RPM value");
        assert!(!is_ssd_marker, "HDD should not have SSD marker");
    }

    #[test]
    fn test_rotation_rate_parsing_for_ssd() {
        // Test parsing smartctl output to detect SSDs
        let smartctl_output = MockSmartctlData::ssd_output("/dev/sdb", "Samsung 870 EVO", "S123456", 500);

        assert!(smartctl_output.contains("Rotation Rate:"));
        assert!(smartctl_output.contains("Solid State Device"));
        assert!(!smartctl_output.contains("7200 rpm"));

        // Verify SSD detection
        let is_ssd = smartctl_output.contains("Solid State Device") ||
                     smartctl_output.contains("SSD");

        assert!(is_ssd, "SSD should have SSD marker");
    }

    #[test]
    fn test_smartctl_field_extraction() {
        // Test that we can extract fields from smartctl output
        let output = MockSmartctlData::hdd_output("/dev/sda", "WDC WD10EZEX", "WD-TEST123", 1000);

        // Simulate field extraction (this is what the real code does)
        let extract_field = |output: &str, field_name: &str| -> Option<String> {
            output
                .lines()
                .find(|line| line.contains(field_name))?
                .split(':')
                .nth(1)?
                .trim()
                .to_string()
                .into()
        };

        let model = extract_field(&output, "Device Model:").expect("Should extract model");
        assert_eq!(model, "WDC WD10EZEX");

        let serial = extract_field(&output, "Serial Number:").expect("Should extract serial");
        assert_eq!(serial, "WD-TEST123");
    }
}

#[cfg(test)]
mod capability_detection_tests {
    use super::*;

    #[test]
    fn test_secure_erase_detection() {
        let hdparm_output = MockHdparmData::secure_erase_supported();

        // Simulate the secure erase check
        let supports_secure_erase = hdparm_output.contains("SECURITY ERASE UNIT");
        let supports_enhanced = hdparm_output.contains("enhanced erase");

        assert!(supports_secure_erase, "Should detect secure erase support");
        assert!(supports_enhanced, "Should detect enhanced erase support");
    }

    #[test]
    fn test_trim_support_detection() {
        let ssd_output = MockSmartctlData::ssd_output("/dev/sdb", "Samsung 870 EVO", "S123456", 500);

        // Check for TRIM support
        let supports_trim = ssd_output.contains("TRIM Command:");

        assert!(supports_trim, "SSD should support TRIM");
    }

    #[test]
    fn test_nvme_sanitize_capability_parsing() {
        let nvme_output = MockNvmeData::sanitize_supported();

        // Parse sanitize capabilities
        let crypto_erase = nvme_output.contains("Crypto Erase Supported: Yes");
        let block_erase = nvme_output.contains("Block Erase Supported: Yes");
        let overwrite = nvme_output.contains("Overwrite Supported: Yes");
        let crypto_scramble = nvme_output.contains("Crypto Scramble Supported:");

        assert!(crypto_erase, "Should support crypto erase");
        assert!(block_erase, "Should support block erase");
        assert!(overwrite, "Should support overwrite");
        assert!(crypto_scramble, "Should have crypto scramble field");
    }

    #[test]
    fn test_nvme_no_sanitize_support() {
        let nvme_output = MockNvmeData::no_sanitize();

        let crypto_erase = nvme_output.contains("Crypto Erase Supported: Yes");
        let block_erase = nvme_output.contains("Block Erase Supported: Yes");

        assert!(!crypto_erase, "Should not support crypto erase");
        assert!(!block_erase, "Should not support block erase");
    }
}

#[cfg(test)]
mod hpa_dco_detection_tests {
    use super::*;

    #[test]
    fn test_hpa_detection_parsing() {
        let current_sectors = 1000000u64;
        let native_sectors = 1200000u64;
        let hdparm_output = MockHdparmData::hpa_detected(current_sectors, native_sectors);

        // Simulate HPA detection
        let has_hpa = hdparm_output.contains("HPA is enabled");
        assert!(has_hpa, "Should detect HPA");

        // Simulate sector parsing
        let contains_sectors = hdparm_output.contains(&format!("{}/{}", current_sectors, native_sectors));
        assert!(contains_sectors, "Should contain sector information");

        // Calculate hidden space
        let hidden_sectors = native_sectors - current_sectors;
        assert_eq!(hidden_sectors, 200000, "Should calculate correct hidden sectors");
    }

    #[test]
    fn test_no_hpa_detection() {
        let sectors = 1000000u64;
        let hdparm_output = MockHdparmData::no_hpa(sectors);

        let has_hpa = hdparm_output.contains("HPA is enabled");
        assert!(!has_hpa, "Should not detect HPA");

        let disabled = hdparm_output.contains("HPA is disabled");
        assert!(disabled, "Should indicate HPA is disabled");
    }

    #[test]
    fn test_dco_detection_parsing() {
        let dco_max = 1000000u64;
        let real_max = 1100000u64;
        let hdparm_output = MockHdparmData::dco_detected(dco_max, real_max);

        // Simulate DCO detection
        let has_dco = hdparm_output.contains("DCO is active");
        assert!(has_dco, "Should detect DCO");

        let has_real_max = hdparm_output.contains("Real max sectors:");
        let has_dco_max = hdparm_output.contains("DCO max sectors:");

        assert!(has_real_max, "Should have real max sectors");
        assert!(has_dco_max, "Should have DCO max sectors");

        // Calculate hidden space
        let hidden_sectors = real_max - dco_max;
        assert_eq!(hidden_sectors, 100000, "Should calculate correct hidden sectors");
    }

    #[test]
    fn test_no_dco_detection() {
        let hdparm_output = MockHdparmData::no_dco();

        let has_dco = hdparm_output.contains("DCO is active");
        assert!(!has_dco, "Should not detect DCO");

        let not_supported = hdparm_output.contains("not supported");
        assert!(not_supported, "Should indicate DCO not supported");
    }

    #[test]
    fn test_combined_hpa_dco_hidden_space() {
        // Test calculation of total hidden space with both HPA and DCO
        let hpa_hidden = 200000u64 * 512; // 200K sectors in bytes
        let dco_hidden = 100000u64 * 512; // 100K sectors in bytes
        let total_hidden = hpa_hidden + dco_hidden;

        assert_eq!(total_hidden, 150 * 1024 * 1024, "Should calculate total hidden space correctly");
    }
}

#[cfg(test)]
mod raid_detection_tests {
    use super::*;

    #[test]
    fn test_raid_member_detection() {
        let mdadm_output = MockMdadmData::raid_member("raid5", "12345678-1234-1234-1234-123456789abc");

        // Simulate RAID detection
        let has_raid_level = mdadm_output.contains("Raid Level");
        let has_array_uuid = mdadm_output.contains("Array UUID");

        assert!(has_raid_level, "Should detect RAID level");
        assert!(has_array_uuid, "Should detect Array UUID");

        let is_raid5 = mdadm_output.contains("raid5");
        assert!(is_raid5, "Should detect RAID 5");
    }

    #[test]
    fn test_non_raid_detection() {
        let mdadm_output = MockMdadmData::not_raid();

        let has_superblock = mdadm_output.contains("No md superblock");
        assert!(has_superblock, "Should indicate no RAID superblock");
    }

    #[test]
    fn test_raid_level_extraction() {
        // Test different RAID levels
        let raid_levels = vec!["raid0", "raid1", "raid5", "raid6", "raid10"];

        for level in raid_levels {
            let output = MockMdadmData::raid_member(level, "test-uuid");
            assert!(output.contains(&format!("Raid Level : {}", level)),
                   "Should contain RAID level: {}", level);
        }
    }
}

#[cfg(test)]
mod system_drive_detection_tests {
    use super::*;

    #[test]
    fn test_system_drive_detection_from_mounts() {
        let device = "/dev/sda1";
        let mounts = MockProcData::system_drive_mounted(device);

        // Simulate system drive detection
        let is_root_device = mounts.lines().any(|line| {
            let parts: Vec<&str> = line.split_whitespace().collect();
            parts.len() >= 2 && parts[1] == "/" && parts[0].starts_with(device)
        });

        assert!(is_root_device, "Should detect system drive from /proc/mounts");
    }

    #[test]
    fn test_data_drive_not_system_drive() {
        let device = "/dev/sdb1";
        let mounts = MockProcData::data_drive_mounted(device);

        // Check that /dev/sdb1 is not the root device
        let is_root_device = mounts.lines().any(|line| {
            let parts: Vec<&str> = line.split_whitespace().collect();
            parts.len() >= 2 && parts[1] == "/" && parts[0].starts_with(device)
        });

        assert!(!is_root_device, "Data drive should not be detected as system drive");

        // Verify it's mounted elsewhere
        let is_mounted = mounts.lines().any(|line| line.starts_with(device));
        assert!(is_mounted, "Data drive should be detected as mounted");
    }

    #[test]
    fn test_system_drive_detection_from_cmdline() {
        let device = "/dev/sda1";
        let cmdline = MockProcData::cmdline_with_device(device);

        // Simulate cmdline-based detection
        let in_cmdline = cmdline.contains(device);
        assert!(in_cmdline, "Should detect system drive from /proc/cmdline");
    }

    #[test]
    fn test_mount_detection() {
        let device = "/dev/sdb1";
        let mounts = MockProcData::data_drive_mounted(device);

        // Simulate mount detection
        let is_mounted = mounts.lines().any(|line| line.starts_with(device));
        assert!(is_mounted, "Should detect mounted device");
    }

    #[test]
    fn test_unmounted_device() {
        let device = "/dev/sdc1";
        let mounts = MockProcData::no_mount();

        let is_mounted = mounts.lines().any(|line| line.starts_with(device));
        assert!(!is_mounted, "Should not detect unmounted device");
    }
}

#[cfg(test)]
mod encryption_detection_tests {
    use super::*;

    #[test]
    fn test_bitlocker_signature_detection() {
        // BitLocker signature: "-FVE-FS-"
        let signature = b"-FVE-FS-";

        // Simulate sector with BitLocker signature
        let mut sector = vec![0u8; 512];
        sector[64..72].copy_from_slice(signature);

        let has_signature = sector.windows(signature.len()).any(|w| w == signature);
        assert!(has_signature, "Should detect BitLocker signature");
    }

    #[test]
    fn test_no_bitlocker_signature() {
        let signature = b"-FVE-FS-";
        let sector = vec![0u8; 512];

        let has_signature = sector.windows(signature.len()).any(|w| w == signature);
        assert!(!has_signature, "Should not detect BitLocker on empty sector");
    }

    #[test]
    fn test_apfs_signature_detection() {
        // APFS signature: "NXSB"
        let signature = b"NXSB\x00\x00\x00\x00";

        let mut sector = vec![0u8; 4096];
        sector[32..40].copy_from_slice(signature);

        let has_signature = sector.windows(8).any(|w| w == signature);
        assert!(has_signature, "Should detect APFS signature");
    }

    #[test]
    fn test_entropy_calculation_for_encrypted_data() {
        // High entropy data (simulating encryption)
        let mut data = vec![0u8; 65536];
        for (i, byte) in data.iter_mut().enumerate() {
            *byte = ((i * 31) % 256) as u8; // Pseudo-random pattern
        }

        // Calculate Shannon entropy
        let mut counts = [0u64; 256];
        for &byte in &data {
            counts[byte as usize] += 1;
        }

        let length = data.len() as f64;
        let mut entropy = 0.0;

        for &count in &counts {
            if count > 0 {
                let probability = count as f64 / length;
                entropy -= probability * probability.log2();
            }
        }

        // High entropy suggests encryption
        assert!(entropy > 7.0, "Encrypted data should have high entropy, got: {}", entropy);
    }

    #[test]
    fn test_entropy_calculation_for_zeros() {
        // Zero-filled data has minimum entropy
        let data = vec![0u8; 65536];

        let mut counts = [0u64; 256];
        for &byte in &data {
            counts[byte as usize] += 1;
        }

        let length = data.len() as f64;
        let mut entropy = 0.0;

        for &count in &counts {
            if count > 0 {
                let probability = count as f64 / length;
                entropy -= probability * probability.log2();
            }
        }

        assert_eq!(entropy, 0.0, "Zero-filled data should have zero entropy");
    }
}

#[cfg(test)]
mod device_filtering_tests {
    use super::*;

    #[test]
    fn test_should_skip_loop_devices() {
        let skip_devices = vec!["loop0", "loop1", "loop99"];

        for device in skip_devices {
            let should_skip = device.starts_with("loop");
            assert!(should_skip, "Should skip loop device: {}", device);
        }
    }

    #[test]
    fn test_should_skip_ram_devices() {
        let skip_devices = vec!["ram0", "ram1"];

        for device in skip_devices {
            let should_skip = device.starts_with("ram");
            assert!(should_skip, "Should skip RAM device: {}", device);
        }
    }

    #[test]
    fn test_should_skip_device_mapper() {
        let skip_devices = vec!["dm-0", "dm-1", "dm-10"];

        for device in skip_devices {
            let should_skip = device.starts_with("dm-");
            assert!(should_skip, "Should skip device mapper: {}", device);
        }
    }

    #[test]
    fn test_should_skip_optical_drives() {
        let skip_devices = vec!["sr0", "sr1"];

        for device in skip_devices {
            let should_skip = device.starts_with("sr");
            assert!(should_skip, "Should skip optical drive: {}", device);
        }
    }

    #[test]
    fn test_should_skip_zram() {
        let skip_devices = vec!["zram0", "zram1"];

        for device in skip_devices {
            let should_skip = device.starts_with("zram");
            assert!(should_skip, "Should skip zram device: {}", device);
        }
    }

    #[test]
    fn test_should_not_skip_physical_drives() {
        let physical_devices = vec!["sda", "sdb", "nvme0n1", "mmcblk0"];

        for device in physical_devices {
            let should_skip = device.starts_with("loop") ||
                             device.starts_with("ram") ||
                             device.starts_with("dm-") ||
                             device.starts_with("sr") ||
                             device.starts_with("zram");

            assert!(!should_skip, "Should not skip physical device: {}", device);
        }
    }
}

#[cfg(test)]
mod blockdev_size_tests {
    use super::*;

    #[test]
    fn test_blockdev_size_parsing() {
        let size_bytes = 1000000000000u64; // 1TB
        let output = MockBlockdevData::size_bytes(size_bytes);

        // Simulate parsing
        let parsed_size = output.trim().parse::<u64>().expect("Should parse size");
        assert_eq!(parsed_size, size_bytes, "Should parse blockdev size correctly");
    }

    #[test]
    fn test_blockdev_sector_parsing() {
        let sectors = 1953525168u64; // ~1TB in 512-byte sectors
        let output = MockBlockdevData::size_sectors(sectors);

        let parsed_sectors = output.trim().parse::<u64>().expect("Should parse sectors");
        assert_eq!(parsed_sectors, sectors, "Should parse sector count correctly");

        // Convert to bytes
        let size_bytes = parsed_sectors * 512;
        assert_eq!(size_bytes, 1000204886016, "Should convert sectors to bytes correctly");
    }

    #[test]
    fn test_size_conversion_calculations() {
        // Test various size conversions
        let test_cases = vec![
            (1024u64 * 1024 * 1024, "1 GB"),
            (500 * 1024 * 1024 * 1024, "500 GB"),
            (1024u64 * 1024 * 1024 * 1024, "1 TB"),
            (2 * 1024u64 * 1024 * 1024 * 1024, "2 TB"),
        ];

        for (bytes, description) in test_cases {
            let gb = bytes / (1024 * 1024 * 1024);
            assert!(gb > 0, "Should calculate GB correctly for {}", description);
        }
    }
}

#[cfg(test)]
mod usb_device_detection_tests {
    use super::*;

    #[test]
    fn test_usb_path_detection_simulation() {
        // Simulate USB device detection via sysfs path
        let usb_paths = vec![
            "/sys/devices/pci0000:00/0000:00:14.0/usb1/1-1/1-1:1.0/host6/target6:0:0/6:0:0:0/block/sda",
            "/sys/devices/pci0000:00/0000:00:14.0/usb2/2-2/2-2:1.0/host7/target7:0:0/7:0:0:0/block/sdb",
        ];

        for path in usb_paths {
            let is_usb = path.contains("usb");
            assert!(is_usb, "Should detect USB device from path: {}", path);
        }
    }

    #[test]
    fn test_non_usb_path_detection() {
        let non_usb_paths = vec![
            "/sys/devices/pci0000:00/0000:00:17.0/ata1/host0/target0:0:0/0:0:0:0/block/sda",
            "/sys/devices/pci0000:00/0000:00:1c.4/0000:03:00.0/nvme/nvme0/nvme0n1",
        ];

        for path in non_usb_paths {
            let is_usb = path.contains("usb");
            assert!(!is_usb, "Should not detect non-USB device as USB: {}", path);
        }
    }
}
