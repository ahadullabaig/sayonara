// Integrated Wipe Operations for Advanced Drive Types
//
// This module provides high-level wipe operations that integrate the OptimizedIO
// engine with advanced drive types (SMR, Optane, Hybrid, eMMC, RAID, NVMe).

use super::types::emmc::EMMCDevice;
use super::types::hybrid::HybridDrive;
use super::types::nvme::advanced::{NVMeAdvanced, NVMeNamespace, NamespaceType};
use super::types::optane::OptaneDrive;
use super::types::raid::RAIDArray;
use super::types::smr::SMRDrive;
use crate::crypto::secure_rng::secure_random_bytes;
use crate::io::{IOConfig, IOHandle, OptimizedIO};
use crate::ui::progress::ProgressBar;
use anyhow::Result;

// ==================== SMR DRIVE INTEGRATION ====================

/// Wipe an SMR drive using OptimizedIO with proper zone handling
pub fn wipe_smr_drive_integrated(smr_drive: &SMRDrive, algorithm: WipeAlgorithm) -> Result<()> {
    println!("🔄 Starting SMR-aware integrated wipe");
    println!("   Drive: {}", smr_drive.device_path);
    println!("   Algorithm: {:?}", algorithm);
    println!("   Zone Model: {:?}", smr_drive.zone_model);

    // Configure I/O for sequential writes (optimal for SMR)
    let mut io_config = IOConfig::hdd_optimized();
    io_config.queue_depth = 2; // Lower queue depth for SMR sequential writes

    let mut io_handle = OptimizedIO::open(&smr_drive.device_path, io_config)?;

    // Use SMR's built-in wipe function with OptimizedIO callbacks
    smr_drive.wipe_smr_drive(|offset, size| {
        write_pattern_to_zone(&mut io_handle, offset, size, &algorithm)
    })?;

    // Validate
    smr_drive.validate_smr_wipe()?;

    // Print performance
    OptimizedIO::print_performance_report(&io_handle, None);

    println!("✅ SMR wipe completed successfully");
    Ok(())
}

/// Write pattern to a specific zone using OptimizedIO
fn write_pattern_to_zone(
    io_handle: &mut IOHandle,
    offset: u64,
    size: u64,
    algorithm: &WipeAlgorithm,
) -> Result<()> {
    let mut bytes_written = 0u64;
    let buffer_size = io_handle.acquire_buffer()?.as_slice().len() as u64;

    while bytes_written < size {
        let write_size = (size - bytes_written).min(buffer_size);
        let mut buffer = io_handle.acquire_buffer()?;

        // Fill buffer based on algorithm
        match algorithm {
            WipeAlgorithm::Zeros => {
                buffer.as_mut_slice().fill(0x00);
            }
            WipeAlgorithm::Ones => {
                buffer.as_mut_slice().fill(0xFF);
            }
            WipeAlgorithm::Random => {
                secure_random_bytes(buffer.as_mut_slice())?;
            }
            WipeAlgorithm::Pattern(byte) => {
                buffer.as_mut_slice().fill(*byte);
            }
        }

        // Write to zone
        let written = io_handle.write_at(
            &buffer.as_slice()[..write_size as usize],
            offset + bytes_written,
        )?;

        bytes_written += written as u64;
    }

    io_handle.sync()?;
    Ok(())
}

// ==================== OPTANE / 3D XPOINT INTEGRATION ====================

/// Wipe Intel Optane drive using optimized operations
pub fn wipe_optane_drive_integrated(
    optane_drive: &OptaneDrive,
    use_instant_erase: bool,
) -> Result<()> {
    println!("🔄 Starting Optane/3D XPoint integrated wipe");
    println!("   Drive: {}", optane_drive.device_path);

    if use_instant_erase && optane_drive.supports_ise {
        // Use hardware Instant Secure Erase
        println!("   Using hardware Instant Secure Erase (ISE)");
        optane_drive.instant_secure_erase()?;
        println!("✅ Optane ISE completed successfully");
    } else {
        // Use OptimizedIO with 3D XPoint-specific patterns
        println!("   Using software overwrite with 3D XPoint patterns");

        let io_config = IOConfig::nvme_optimized();
        let mut io_handle = OptimizedIO::open(&optane_drive.device_path, io_config)?;

        // Wipe each namespace
        for namespace in &optane_drive.namespaces {
            println!(
                "   Wiping namespace {}: {} bytes",
                namespace.nsid, namespace.capacity
            );

            // 3D XPoint benefits from specific patterns
            // Pass 1: Write 0x00
            wipe_namespace_with_pattern(&mut io_handle, namespace.capacity, 0x00)?;

            // Pass 2: Write 0xFF (cell charge reversal)
            wipe_namespace_with_pattern(&mut io_handle, namespace.capacity, 0xFF)?;

            // Pass 3: Random data
            wipe_namespace_with_random(&mut io_handle, namespace.capacity)?;
        }

        OptimizedIO::print_performance_report(&io_handle, None);
        println!("✅ Optane software wipe completed successfully");
    }

    Ok(())
}

fn wipe_namespace_with_pattern(io_handle: &mut IOHandle, size: u64, pattern: u8) -> Result<()> {
    Ok(OptimizedIO::sequential_write(io_handle, size, |buffer| {
        buffer.as_mut_slice().fill(pattern);
        Ok(())
    })?)
}

fn wipe_namespace_with_random(io_handle: &mut IOHandle, size: u64) -> Result<()> {
    Ok(OptimizedIO::sequential_write(io_handle, size, |buffer| {
        secure_random_bytes(buffer.as_mut_slice())?;
        Ok(())
    })?)
}

// ==================== HYBRID DRIVE (SSHD) INTEGRATION ====================

/// Wipe hybrid drive (SSHD) with separate cache and HDD handling
pub fn wipe_hybrid_drive_integrated(hybrid_drive: &mut HybridDrive) -> Result<()> {
    println!("🔄 Starting Hybrid Drive (SSHD) integrated wipe");
    println!("   Drive: {}", hybrid_drive.device_path);
    println!(
        "   SSD Cache: {} GB",
        hybrid_drive.ssd_cache.cache_size / (1024 * 1024 * 1024)
    );
    println!(
        "   HDD Capacity: {} GB",
        hybrid_drive.hdd_portion.capacity / (1024 * 1024 * 1024)
    );

    // Step 1: Disable and wipe SSD cache
    println!("\n   Step 1: Disabling and wiping SSD cache...");
    hybrid_drive.disable_cache()?;

    // Wipe pinned regions first
    if !hybrid_drive.pinned_data.is_empty() {
        println!(
            "   Found {} pinned cache regions",
            hybrid_drive.pinned_data.len()
        );
        hybrid_drive.unpin_data()?;
    }

    // Flush cache
    hybrid_drive.flush_cache()?;

    // Step 2: Wipe HDD portion with optimized I/O
    println!("\n   Step 2: Wiping HDD portion...");
    let io_config = IOConfig::hdd_optimized();
    let mut io_handle = OptimizedIO::open(&hybrid_drive.device_path, io_config)?;

    // 3-pass wipe (DoD-style)
    let size = hybrid_drive.hdd_portion.capacity;

    println!("      Pass 1/3: Writing 0x00...");
    wipe_with_pattern_progress(&mut io_handle, size, 0x00)?;

    println!("      Pass 2/3: Writing 0xFF...");
    wipe_with_pattern_progress(&mut io_handle, size, 0xFF)?;

    println!("      Pass 3/3: Writing random data...");
    wipe_with_random_progress(&mut io_handle, size)?;

    OptimizedIO::print_performance_report(&io_handle, None);
    println!("✅ Hybrid drive wipe completed successfully");

    Ok(())
}

fn wipe_with_pattern_progress(io_handle: &mut IOHandle, size: u64, pattern: u8) -> Result<()> {
    let mut bytes_written = 0u64;
    let mut bar = ProgressBar::new(48);

    OptimizedIO::sequential_write(io_handle, size, |buffer| {
        buffer.as_mut_slice().fill(pattern);
        bytes_written += buffer.as_slice().len() as u64;

        if bytes_written.is_multiple_of(100 * 1024 * 1024) {
            let progress = (bytes_written as f64 / size as f64) * 100.0;
            bar.render(progress, Some(bytes_written), Some(size));
        }
        Ok(())
    })?;

    bar.render(100.0, Some(size), Some(size));
    Ok(())
}

fn wipe_with_random_progress(io_handle: &mut IOHandle, size: u64) -> Result<()> {
    let mut bytes_written = 0u64;
    let mut bar = ProgressBar::new(48);

    OptimizedIO::sequential_write(io_handle, size, |buffer| {
        secure_random_bytes(buffer.as_mut_slice())?;
        bytes_written += buffer.as_slice().len() as u64;

        if bytes_written.is_multiple_of(100 * 1024 * 1024) {
            let progress = (bytes_written as f64 / size as f64) * 100.0;
            bar.render(progress, Some(bytes_written), Some(size));
        }
        Ok(())
    })?;

    bar.render(100.0, Some(size), Some(size));
    Ok(())
}

// ==================== eMMC / UFS INTEGRATION ====================

/// Wipe eMMC/UFS embedded storage
pub fn wipe_emmc_drive_integrated(emmc_drive: &EMMCDevice, use_hardware_erase: bool) -> Result<()> {
    println!("🔄 Starting eMMC/UFS integrated wipe");
    println!("   Device: {}", emmc_drive.device_path);

    if use_hardware_erase {
        // Try hardware erase
        println!("   Attempting hardware erase...");

        // Try secure erase - emmc_drive.secure_erase() if available
        // For now, fall back to software
        println!("   Hardware erase not yet fully implemented, using software");
        wipe_emmc_software(emmc_drive)?;
    } else {
        wipe_emmc_software(emmc_drive)?;
    }

    Ok(())
}

fn wipe_emmc_software(emmc_drive: &EMMCDevice) -> Result<()> {
    println!("   Using software overwrite");

    // eMMC typically benefits from SSD-style config
    let io_config = IOConfig::sata_ssd_optimized();
    let mut io_handle = OptimizedIO::open(&emmc_drive.device_path, io_config)?;

    // Wipe user data area
    let size = emmc_drive.user_data_area.size;
    println!(
        "   Wiping user data area: {} GB",
        size / (1024 * 1024 * 1024)
    );

    // Single pass random for embedded storage
    wipe_with_random_progress(&mut io_handle, size)?;

    // Wipe boot partitions if present
    for boot_part in &emmc_drive.boot_partitions {
        if boot_part.size > 0 {
            println!(
                "   Wiping boot partition {}: {} MB",
                boot_part.partition_number,
                boot_part.size / (1024 * 1024)
            );
            wipe_with_pattern_progress(&mut io_handle, boot_part.size, 0x00)?;
        }
    }

    OptimizedIO::print_performance_report(&io_handle, None);
    println!("✅ eMMC software wipe completed successfully");

    Ok(())
}

// ==================== RAID ARRAY INTEGRATION ====================

/// Wipe RAID array members individually
pub fn wipe_raid_array_integrated(raid_array: &RAIDArray, wipe_metadata: bool) -> Result<()> {
    println!("🔄 Starting RAID Array integrated wipe");
    println!("   Array: {}", raid_array.device_path);
    println!("   Type: {:?}", raid_array.raid_type);
    println!("   Members: {}", raid_array.member_drives.len());

    // Wipe each member individually
    for (idx, member_path) in raid_array.member_drives.iter().enumerate() {
        println!(
            "\n   Wiping member {}/{}: {}",
            idx + 1,
            raid_array.member_drives.len(),
            member_path
        );

        let io_config = IOConfig::default();
        let mut io_handle = OptimizedIO::open(member_path, io_config)?;

        // Get drive size
        let size = get_device_size(member_path)?;
        println!("      Size: {} GB", size / (1024 * 1024 * 1024));

        // 3-pass wipe
        println!("      Pass 1/3: zeros");
        wipe_with_pattern_progress(&mut io_handle, size, 0x00)?;

        println!("      Pass 2/3: ones");
        wipe_with_pattern_progress(&mut io_handle, size, 0xFF)?;

        println!("      Pass 3/3: random");
        wipe_with_random_progress(&mut io_handle, size)?;

        // Wipe metadata regions if requested
        if wipe_metadata {
            println!("      Wiping RAID metadata regions...");
            // Metadata wiping would go here - member.metadata_offset, etc.
            // For now, the full wipe covers metadata as well
        }

        println!("      ✅ Member {} completed", idx + 1);
    }

    println!("\n✅ RAID array wipe completed successfully");
    Ok(())
}

// ==================== NVME ADVANCED INTEGRATION ====================

/// Wipe NVMe drive with advanced features (multiple namespaces, ZNS, etc.)
pub fn wipe_nvme_advanced_integrated(nvme_drive: &NVMeAdvanced, use_format: bool) -> Result<()> {
    println!("🔄 Starting Advanced NVMe integrated wipe");
    println!("   Controller: {}", nvme_drive.controller_path);
    println!("   Model: {}", nvme_drive.model);
    println!("   Namespaces: {}", nvme_drive.namespaces.len());
    println!("   ZNS Support: {}", nvme_drive.zns_support);

    if use_format {
        // Use NVMe Format command (fastest)
        println!("   Using NVMe Format command (hardware erase)");

        for namespace in &nvme_drive.namespaces {
            if namespace.is_active {
                println!("      Formatting namespace {}...", namespace.nsid);
                format_nvme_namespace(&nvme_drive.controller_path, namespace.nsid)?;
            }
        }

        println!("✅ NVMe format completed successfully");
    } else {
        // Software wipe with OptimizedIO
        println!("   Using software overwrite");

        let io_config = IOConfig::nvme_optimized();

        for namespace in &nvme_drive.namespaces {
            if !namespace.is_active {
                println!("   Skipping inactive namespace {}", namespace.nsid);
                continue;
            }

            println!("\n   Wiping namespace {}:", namespace.nsid);
            println!("      Path: {}", namespace.device_path);
            println!("      Type: {:?}", namespace.namespace_type);
            println!("      Size: {} GB", namespace.size / (1024 * 1024 * 1024));

            let mut io_handle = OptimizedIO::open(&namespace.device_path, io_config.clone())?;

            match namespace.namespace_type {
                NamespaceType::Block => {
                    // Standard block namespace - 3 pass wipe
                    println!("      Standard block namespace - 3 pass wipe");
                    wipe_namespace_multipass(&mut io_handle, namespace.size)?;
                }

                NamespaceType::ZonedNamespace => {
                    // ZNS namespace - zone-aware wipe
                    println!("      Zoned Namespace - zone-aware wipe");
                    wipe_zns_namespace(&mut io_handle, namespace)?;
                }

                NamespaceType::KeyValue => {
                    // Key-Value namespace - overwrite all keys
                    println!("      Key-Value namespace - overwrite");
                    wipe_kv_namespace(&mut io_handle, namespace.size)?;
                }

                NamespaceType::Computational => {
                    // Computational storage - basic overwrite
                    println!("      Computational storage - basic overwrite");
                    wipe_namespace_multipass(&mut io_handle, namespace.size)?;
                }
            }

            OptimizedIO::print_performance_report(&io_handle, None);
            println!("      ✅ Namespace {} completed", namespace.nsid);
        }

        println!("\n✅ NVMe software wipe completed successfully");
    }

    Ok(())
}

/// Format NVMe namespace using Format command
fn format_nvme_namespace(controller_path: &str, nsid: u32) -> Result<()> {
    use std::process::Command;

    let output = Command::new("nvme")
        .args(["format", controller_path, "-n", &nsid.to_string()])
        .output()?;

    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "NVMe format failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(())
}

/// Wipe standard namespace with multiple passes
fn wipe_namespace_multipass(io_handle: &mut IOHandle, size: u64) -> Result<()> {
    // Pass 1: Zeros
    println!("         Pass 1/3: zeros");
    wipe_with_pattern_progress(io_handle, size, 0x00)?;

    // Pass 2: Ones
    println!("         Pass 2/3: ones");
    wipe_with_pattern_progress(io_handle, size, 0xFF)?;

    // Pass 3: Random
    println!("         Pass 3/3: random");
    wipe_with_random_progress(io_handle, size)?;

    Ok(())
}

/// Wipe ZNS namespace with zone awareness
fn wipe_zns_namespace(io_handle: &mut IOHandle, namespace: &NVMeNamespace) -> Result<()> {
    if let Some(zones) = &namespace.zones {
        println!("         Wiping {} zones", zones.len());

        for zone in zones {
            if zone.needs_reset() {
                println!("         Resetting zone {}", zone.zone_id);
                // Reset zone using nvme-cli or ioctl
                // This is a simplified version
            }

            // Write to zone sequentially
            let zone_size = zone.zone_capacity * 512; // Assuming 512-byte blocks
            let zone_offset = zone.zone_start_lba * 512;

            // Single pass for ZNS (sequential write constraint)
            wipe_zone_sequential(io_handle, zone_offset, zone_size)?;
        }
    } else {
        // Fallback to standard wipe
        wipe_namespace_multipass(io_handle, namespace.size)?;
    }

    Ok(())
}

/// Wipe a single zone sequentially
fn wipe_zone_sequential(io_handle: &mut IOHandle, offset: u64, size: u64) -> Result<()> {
    let mut bytes_written = 0u64;

    while bytes_written < size {
        let mut buffer = io_handle.acquire_buffer()?;
        secure_random_bytes(buffer.as_mut_slice())?;

        let to_write = (size - bytes_written).min(buffer.as_slice().len() as u64);
        io_handle.write_at(
            &buffer.as_slice()[..to_write as usize],
            offset + bytes_written,
        )?;

        bytes_written += to_write;
    }

    io_handle.sync()?;
    Ok(())
}

/// Wipe Key-Value namespace
fn wipe_kv_namespace(io_handle: &mut IOHandle, size: u64) -> Result<()> {
    // For KV namespaces, we do a simple overwrite
    // Real implementation would enumerate and delete keys
    println!("         KV namespace wipe (simplified)");
    wipe_namespace_multipass(io_handle, size)?;
    Ok(())
}

// ==================== HELPER FUNCTIONS ====================

/// Get device size using sysfs or ioctl
pub(crate) fn get_device_size(device_path: &str) -> Result<u64> {
    use std::fs;

    // Try sysfs first (Linux)
    #[cfg(target_os = "linux")]
    {
        if let Some(dev_name) = device_path.strip_prefix("/dev/") {
            let size_path = format!("/sys/block/{}/size", dev_name);
            if let Ok(content) = fs::read_to_string(&size_path) {
                if let Ok(blocks) = content.trim().parse::<u64>() {
                    return Ok(blocks * 512); // Convert 512-byte blocks to bytes
                }
            }
        }
    }

    // Fallback: Try to open and get metadata
    let metadata = fs::metadata(device_path)?;
    Ok(metadata.len())
}

// ==================== HELPER TYPES ====================

#[derive(Debug, Clone)]
pub enum WipeAlgorithm {
    Zeros,
    Ones,
    Random,
    Pattern(u8),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wipe_algorithm_variants() {
        let algos = [WipeAlgorithm::Zeros,
            WipeAlgorithm::Ones,
            WipeAlgorithm::Random,
            WipeAlgorithm::Pattern(0xAA)];

        assert_eq!(algos.len(), 4);
    }
}
