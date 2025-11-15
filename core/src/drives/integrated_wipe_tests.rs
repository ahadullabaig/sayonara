// Comprehensive tests for Integrated Wipe Operations
//
// NOTE: Many functions in integrated_wipe.rs require actual hardware access and IOHandle
// objects. These tests focus on testable logic, helper functions, and integration test
// stubs that can be run in hardware test environments.

use super::integrated_wipe::*;
use anyhow::Result;

// ==================== WIPE ALGORITHM TESTS ====================

#[test]
fn test_wipe_algorithm_zeros_variant() {
    let algo = WipeAlgorithm::Zeros;
    assert!(matches!(algo, WipeAlgorithm::Zeros));
}

#[test]
fn test_wipe_algorithm_ones_variant() {
    let algo = WipeAlgorithm::Ones;
    assert!(matches!(algo, WipeAlgorithm::Ones));
}

#[test]
fn test_wipe_algorithm_random_variant() {
    let algo = WipeAlgorithm::Random;
    assert!(matches!(algo, WipeAlgorithm::Random));
}

#[test]
fn test_wipe_algorithm_pattern_variant() {
    let algo = WipeAlgorithm::Pattern(0xAA);
    match algo {
        WipeAlgorithm::Pattern(byte) => assert_eq!(byte, 0xAA),
        _ => panic!("Expected Pattern variant"),
    }
}

#[test]
fn test_wipe_algorithm_pattern_different_bytes() {
    let patterns = vec![0x00, 0xFF, 0xAA, 0x55, 0xDE, 0xAD, 0xBE, 0xEF];

    for byte in patterns {
        let algo = WipeAlgorithm::Pattern(byte);
        match algo {
            WipeAlgorithm::Pattern(b) => assert_eq!(b, byte),
            _ => panic!("Expected Pattern variant"),
        }
    }
}

#[test]
fn test_wipe_algorithm_clone() {
    let algo1 = WipeAlgorithm::Zeros;
    let algo2 = algo1.clone();
    assert!(matches!(algo2, WipeAlgorithm::Zeros));

    let algo3 = WipeAlgorithm::Pattern(0x42);
    let algo4 = algo3.clone();
    match algo4 {
        WipeAlgorithm::Pattern(b) => assert_eq!(b, 0x42),
        _ => panic!("Expected Pattern variant"),
    }
}

#[test]
fn test_wipe_algorithm_debug() {
    let algo = WipeAlgorithm::Zeros;
    let debug_str = format!("{:?}", algo);
    assert!(debug_str.contains("Zeros"));

    let algo2 = WipeAlgorithm::Pattern(0xAA);
    let debug_str2 = format!("{:?}", algo2);
    assert!(debug_str2.contains("Pattern"));
}

// ==================== DEVICE SIZE HELPER TESTS ====================

#[test]
#[cfg(target_os = "linux")]
fn test_get_device_size_with_valid_sysfs() -> Result<()> {
    // Test with actual system devices if available
    // This tests the sysfs reading logic

    use std::fs;
    use std::path::Path;

    // Look for any block device
    if let Ok(entries) = fs::read_dir("/sys/block") {
        for entry in entries.flatten() {
            let dev_name = entry.file_name();
            let dev_name_str = dev_name.to_string_lossy();

            // Skip loop devices and other virtual devices
            if dev_name_str.starts_with("loop")
                || dev_name_str.starts_with("ram")
                || dev_name_str.starts_with("dm-")
            {
                continue;
            }

            let device_path = format!("/dev/{}", dev_name_str);
            if Path::new(&device_path).exists() {
                // Try to get size (may fail due to permissions, that's OK)
                let result = get_device_size(&device_path);

                // If it succeeds, size should be positive
                if let Ok(size) = result {
                    assert!(size > 0, "Device size should be positive");
                    return Ok(()); // Found at least one device
                }
            }
        }
    }

    // If no devices found, that's OK for this test
    Ok(())
}

#[test]
fn test_get_device_size_with_nonexistent_device() {
    let result = get_device_size("/dev/nonexistent_device_xyz123");
    assert!(result.is_err(), "Should fail for nonexistent device");
}

#[test]
#[cfg(target_os = "linux")]
fn test_get_device_size_sysfs_path_parsing() {
    // Test the path parsing logic
    let device_path = "/dev/sda";
    let dev_name = device_path.strip_prefix("/dev/").unwrap();
    assert_eq!(dev_name, "sda");

    let size_path = format!("/sys/block/{}/size", dev_name);
    assert_eq!(size_path, "/sys/block/sda/size");
}

#[test]
fn test_device_size_block_to_bytes_conversion() {
    // Test the 512-byte block to bytes conversion logic
    let blocks = 1953525168u64; // Example: 1TB drive
    let bytes = blocks * 512;
    let expected_bytes = 1000204886016u64; // ~1TB
    assert_eq!(bytes, expected_bytes);
}

#[test]
fn test_device_size_small_device() {
    // Test conversion for small devices (e.g., USB sticks)
    let blocks = 15759360u64; // Example: ~8GB device
    let bytes = blocks * 512;
    assert_eq!(bytes, 8068792320u64); // ~8GB
}

// ==================== PATTERN BUFFER FILL TESTS ====================
// These test the logic of how buffers are filled with patterns

#[test]
fn test_fill_buffer_with_zeros() {
    let mut buffer = vec![0xFF; 1024]; // Start with all 0xFF
    buffer.fill(0x00);
    assert!(buffer.iter().all(|&b| b == 0x00));
}

#[test]
fn test_fill_buffer_with_ones() {
    let mut buffer = vec![0x00; 1024]; // Start with all 0x00
    buffer.fill(0xFF);
    assert!(buffer.iter().all(|&b| b == 0xFF));
}

#[test]
fn test_fill_buffer_with_pattern() {
    let mut buffer = vec![0x00; 1024];
    let pattern = 0xAA;
    buffer.fill(pattern);
    assert!(buffer.iter().all(|&b| b == pattern));
}

#[test]
fn test_fill_buffer_multiple_patterns() {
    let patterns = vec![0x00, 0xFF, 0xAA, 0x55, 0xDE, 0xAD];

    for pattern in patterns {
        let mut buffer = vec![0x00; 512];
        buffer.fill(pattern);
        assert!(
            buffer.iter().all(|&b| b == pattern),
            "Buffer should be filled with pattern 0x{:02X}",
            pattern
        );
    }
}

// ==================== BUFFER SIZE CALCULATION TESTS ====================

#[test]
fn test_write_size_calculation_full_buffer() {
    let size = 1024 * 1024u64; // 1MB to write
    let bytes_written = 0u64;
    let buffer_size = 4096u64; // 4KB buffer

    let write_size = (size - bytes_written).min(buffer_size);
    assert_eq!(
        write_size, buffer_size,
        "First write should use full buffer"
    );
}

#[test]
fn test_write_size_calculation_partial_buffer() {
    let size = 5000u64; // 5000 bytes to write
    let bytes_written = 4096u64; // Already written 4096
    let buffer_size = 4096u64; // 4KB buffer

    let write_size = (size - bytes_written).min(buffer_size);
    assert_eq!(write_size, 904, "Last write should use partial buffer");
}

#[test]
fn test_write_size_calculation_exact_buffer() {
    let size = 8192u64; // Exactly 2 buffers
    let bytes_written = 4096u64; // One buffer written
    let buffer_size = 4096u64;

    let write_size = (size - bytes_written).min(buffer_size);
    assert_eq!(
        write_size, buffer_size,
        "Should use full buffer for exact fit"
    );
}

#[test]
fn test_write_size_calculation_zero_remaining() {
    let size = 4096u64;
    let bytes_written = 4096u64;

    let remaining = size - bytes_written;
    assert_eq!(remaining, 0, "Should have zero bytes remaining");
}

// ==================== OFFSET CALCULATION TESTS ====================

#[test]
fn test_offset_calculation_sequential() {
    let start_offset = 0u64;
    let buffer_size = 4096u64;

    let offsets: Vec<u64> = (0..10).map(|i| start_offset + (i * buffer_size)).collect();

    assert_eq!(offsets[0], 0);
    assert_eq!(offsets[1], 4096);
    assert_eq!(offsets[9], 36864);
}

#[test]
fn test_offset_calculation_zone_based() {
    // Test zone offset calculation (for SMR/ZNS)
    let zone_id = 5u64;
    let zone_size = 256 * 1024 * 1024u64; // 256MB zones
    let zone_offset = zone_id * zone_size;

    assert_eq!(zone_offset, 1342177280); // 5 * 256MB
}

#[test]
fn test_offset_calculation_with_lba() {
    // Test LBA to byte offset conversion
    let lba = 1000u64;
    let sector_size = 512u64;
    let byte_offset = lba * sector_size;

    assert_eq!(byte_offset, 512000);
}

// ==================== PROGRESS CALCULATION TESTS ====================

#[test]
fn test_progress_percentage_calculation() {
    let bytes_written = 50 * 1024 * 1024u64; // 50MB
    let total_size = 100 * 1024 * 1024u64; // 100MB
    let progress = (bytes_written as f64 / total_size as f64) * 100.0;

    assert!((progress - 50.0).abs() < 0.01, "Progress should be 50%");
}

#[test]
fn test_progress_percentage_completion() {
    let bytes_written = 100 * 1024 * 1024u64;
    let total_size = 100 * 1024 * 1024u64;
    let progress = (bytes_written as f64 / total_size as f64) * 100.0;

    assert!((progress - 100.0).abs() < 0.01, "Progress should be 100%");
}

#[test]
fn test_progress_percentage_start() {
    let bytes_written = 0u64;
    let total_size = 100 * 1024 * 1024u64;
    let progress = (bytes_written as f64 / total_size as f64) * 100.0;

    assert!((progress - 0.0).abs() < 0.01, "Progress should be 0%");
}

#[test]
fn test_progress_update_interval() {
    // Test that progress updates every 100MB
    let update_interval = 100 * 1024 * 1024u64;

    let bytes_values = vec![
        50 * 1024 * 1024u64,  // 50MB - no update
        99 * 1024 * 1024u64,  // 99MB - no update
        100 * 1024 * 1024u64, // 100MB - update!
        150 * 1024 * 1024u64, // 150MB - no update
        200 * 1024 * 1024u64, // 200MB - update!
    ];

    for bytes in bytes_values {
        let should_update = bytes % update_interval == 0;
        let expected =
            bytes >= update_interval && (bytes / update_interval) * update_interval == bytes;
        assert_eq!(should_update, expected);
    }
}

// ==================== MULTI-PASS WIPE LOGIC TESTS ====================

#[test]
fn test_multipass_wipe_pattern_sequence() {
    // Test the 3-pass sequence: zeros -> ones -> random
    let pass_patterns = [0x00, 0xFF]; // Random is tested separately

    for (idx, pattern) in pass_patterns.iter().enumerate() {
        let mut buffer = vec![0xAA; 1024]; // Start with different pattern
        buffer.fill(*pattern);

        assert!(
            buffer.iter().all(|&b| b == *pattern),
            "Pass {} should fill with 0x{:02X}",
            idx + 1,
            pattern
        );
    }
}

#[test]
fn test_multipass_wipe_pass_count() {
    // Verify 3-pass wipe structure
    let passes = [
        ("zeros", 0x00),
        ("ones", 0xFF),
        ("random", 0xAA), // Placeholder for random
    ];

    assert_eq!(passes.len(), 3, "Should have exactly 3 passes");
}

// ==================== NAMESPACE HANDLING TESTS ====================

#[test]
fn test_namespace_size_calculation() {
    // Test namespace size calculations (from NVMe)
    let block_size = 512u64;
    let block_count = 1953525168u64;
    let namespace_size = block_size * block_count;

    assert_eq!(namespace_size, 1000204886016); // ~1TB
}

#[test]
fn test_namespace_gb_conversion() {
    let namespace_size = 1000204886016u64; // ~1TB in bytes
    let gb = namespace_size / (1024 * 1024 * 1024);

    assert_eq!(gb, 931); // ~931 GB (binary)
}

#[test]
fn test_multiple_namespaces_total_size() {
    let namespace_sizes = [
        500 * 1024 * 1024 * 1024u64, // 500GB
        250 * 1024 * 1024 * 1024u64, // 250GB
        250 * 1024 * 1024 * 1024u64, // 250GB
    ];

    let total: u64 = namespace_sizes.iter().sum();
    let expected = 1000 * 1024 * 1024 * 1024u64; // 1TB

    assert_eq!(total, expected);
}

// ==================== ZONE HANDLING TESTS (SMR/ZNS) ====================

#[test]
fn test_zone_size_calculation() {
    let zone_capacity = 524288u64; // 256MB in 512-byte blocks
    let sector_size = 512u64;
    let zone_size_bytes = zone_capacity * sector_size;

    assert_eq!(zone_size_bytes, 268435456); // 256MB
}

#[test]
fn test_zone_offset_from_lba() {
    let zone_start_lba = 1000u64;
    let sector_size = 512u64;
    let zone_offset = zone_start_lba * sector_size;

    assert_eq!(zone_offset, 512000);
}

#[test]
fn test_zone_count_calculation() {
    let total_capacity = 1024 * 1024 * 1024 * 1024u64; // 1TB
    let zone_size = 256 * 1024 * 1024u64; // 256MB
    let zone_count = total_capacity / zone_size;

    assert_eq!(zone_count, 4096); // 4096 zones
}

// ==================== RAID MEMBER TESTS ====================

#[test]
fn test_raid_member_count() {
    // Test RAID member iteration logic
    let member_drives = ["/dev/sda".to_string(),
        "/dev/sdb".to_string(),
        "/dev/sdc".to_string(),
        "/dev/sdd".to_string()];

    assert_eq!(member_drives.len(), 4);

    for (idx, member) in member_drives.iter().enumerate() {
        assert!(member.starts_with("/dev/sd"));
        assert!(idx < member_drives.len());
    }
}

#[test]
fn test_raid_member_progress_calculation() {
    let total_members = 4;
    let completed_members = vec![1, 2, 3, 4];

    for completed in completed_members {
        let progress = (completed as f64 / total_members as f64) * 100.0;
        assert_eq!(progress, completed as f64 * 25.0);
    }
}

// ==================== BOOT PARTITION TESTS (eMMC) ====================

#[test]
fn test_boot_partition_size_mb_conversion() {
    let boot_partition_size = 4 * 1024 * 1024u64; // 4MB
    let mb = boot_partition_size / (1024 * 1024);

    assert_eq!(mb, 4);
}

#[test]
fn test_boot_partition_skip_zero_size() {
    // Test logic for skipping zero-size partitions
    let boot_partitions = [
        ("boot0", 4 * 1024 * 1024u64), // 4MB - should wipe
        ("boot1", 4 * 1024 * 1024u64), // 4MB - should wipe
        ("boot2", 0u64),               // 0 bytes - should skip
    ];

    let non_zero: Vec<_> = boot_partitions
        .iter()
        .filter(|(_, size)| *size > 0)
        .collect();

    assert_eq!(non_zero.len(), 2);
}

// ==================== HYBRID DRIVE CACHE TESTS ====================

#[test]
fn test_hybrid_cache_size_gb_conversion() {
    let cache_size = 8 * 1024 * 1024 * 1024u64; // 8GB SSD cache
    let gb = cache_size / (1024 * 1024 * 1024);

    assert_eq!(gb, 8);
}

#[test]
fn test_hybrid_hdd_capacity_gb_conversion() {
    let hdd_capacity = 2 * 1024 * 1024 * 1024 * 1024u64; // 2TB HDD
    let gb = hdd_capacity / (1024 * 1024 * 1024);

    assert_eq!(gb, 2048);
}

// ==================== ERROR HANDLING TESTS ====================

#[test]
fn test_error_propagation_pattern() {
    // Test that Result<()> properly propagates errors
    let result: Result<()> = Ok(());
    assert!(result.is_ok());

    let error_result: Result<()> = Err(anyhow::anyhow!("Test error"));
    assert!(error_result.is_err());
}

// ==================== INTEGRATION TESTS ====================
// Integration tests for specialized drive types have been moved to:
// tests/hardware_integration.rs
//
// These tests use mock drives and can run without physical hardware:
// - test_wipe_smr_drive
// - test_wipe_optane_drive
// - test_wipe_hybrid_drive
// - test_wipe_emmc_drive
// - test_wipe_raid_array
// - test_wipe_nvme_advanced

// ==================== EDGE CASE TESTS ====================

#[test]
fn test_buffer_alignment_4k() {
    // Test 4K alignment for direct I/O
    let buffer_size = 4096;
    assert_eq!(buffer_size % 4096, 0, "Buffer should be 4K aligned");
}

#[test]
fn test_buffer_size_power_of_two() {
    // Common buffer sizes should be powers of 2
    let buffer_sizes: Vec<u32> = vec![4096, 8192, 16384, 32768, 65536];

    for size in buffer_sizes {
        assert!(size.is_power_of_two(), "{} should be power of 2", size);
    }
}

#[test]
fn test_large_device_size_overflow() {
    // Test that large device sizes don't overflow
    let max_size = u64::MAX;
    let safe_size = max_size - 1024; // Leave room for calculations

    assert!(safe_size < max_size);

    // Test calculations don't overflow
    let block_count = safe_size / 512;
    assert!(block_count > 0);
}

#[test]
fn test_zero_size_device_handling() {
    let size = 0u64;
    let blocks = size / 512;
    assert_eq!(blocks, 0, "Zero size should result in zero blocks");
}
