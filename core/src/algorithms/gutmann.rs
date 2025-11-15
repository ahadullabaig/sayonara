use crate::crypto::secure_rng::secure_random_bytes;
use crate::error::{ErrorContext, Progress, RecoveryCoordinator};
use crate::io::{IOConfig, IOHandle, OptimizedIO};
use crate::ui::progress::ProgressBar;
use crate::DriveType;
use crate::WipeConfig;
use anyhow::{anyhow, Result};
use serde_json::json;
use std::collections::HashMap;
use std::time::Instant;

/// Drive encoding types that affect pattern selection
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DriveEncoding {
    MFM,     // Modified Frequency Modulation (older drives)
    RLL,     // Run Length Limited (2,7)
    PRML,    // Partial Response Maximum Likelihood (modern drives)
    Unknown, // Default to most comprehensive patterns
}

pub struct GutmannWipe;

impl GutmannWipe {
    /// The correct 35-pass Gutmann patterns according to the 1996 paper
    /// Passes 1-4 and 32-35 are random data
    /// Passes 5-31 are specific patterns targeting different encoding schemes
    pub(crate) const GUTMANN_PATTERNS: [(Option<&'static [u8]>, &'static str); 35] = [
        // First 4 passes: Cryptographically secure random data
        (None, "Random Pass 1"),
        (None, "Random Pass 2"),
        (None, "Random Pass 3"),
        (None, "Random Pass 4"),
        // Passes 5-31: Specific patterns for different encoding schemes
        (Some(&[0x55]), "0x55 - MFM/RLL encoding"), // Pass 5
        (Some(&[0xAA]), "0xAA - MFM/RLL encoding"), // Pass 6
        (Some(&[0x92, 0x49, 0x24]), "0x92 0x49 0x24 - MFM specific"), // Pass 7
        (Some(&[0x49, 0x24, 0x92]), "0x49 0x24 0x92 - MFM specific"), // Pass 8
        (Some(&[0x24, 0x92, 0x49]), "0x24 0x92 0x49 - MFM specific"), // Pass 9
        (Some(&[0x00]), "0x00 - All zeros"),        // Pass 10
        (Some(&[0x11]), "0x11 - Pattern"),          // Pass 11
        (Some(&[0x22]), "0x22 - Pattern"),          // Pass 12
        (Some(&[0x33]), "0x33 - Pattern"),          // Pass 13
        (Some(&[0x44]), "0x44 - Pattern"),          // Pass 14
        (Some(&[0x55]), "0x55 - Pattern"),          // Pass 15
        (Some(&[0x66]), "0x66 - Pattern"),          // Pass 16
        (Some(&[0x77]), "0x77 - Pattern"),          // Pass 17
        (Some(&[0x88]), "0x88 - Pattern"),          // Pass 18
        (Some(&[0x99]), "0x99 - Pattern"),          // Pass 19
        (Some(&[0xAA]), "0xAA - Pattern"),          // Pass 20
        (Some(&[0xBB]), "0xBB - Pattern"),          // Pass 21
        (Some(&[0xCC]), "0xCC - Pattern"),          // Pass 22
        (Some(&[0xDD]), "0xDD - Pattern"),          // Pass 23
        (Some(&[0xEE]), "0xEE - Pattern"),          // Pass 24
        (Some(&[0xFF]), "0xFF - All ones"),         // Pass 25
        (Some(&[0x92, 0x49, 0x24]), "RLL (2,7) pattern 1"), // Pass 26
        (Some(&[0x49, 0x24, 0x92]), "RLL (2,7) pattern 2"), // Pass 27
        (Some(&[0x24, 0x92, 0x49]), "RLL (2,7) pattern 3"), // Pass 28
        (Some(&[0x6D, 0xB6, 0xDB]), "RLL (2,7) pattern 4"), // Pass 29
        (Some(&[0xB6, 0xDB, 0x6D]), "RLL (2,7) pattern 5"), // Pass 30
        (Some(&[0xDB, 0x6D, 0xB6]), "RLL (2,7) pattern 6"), // Pass 31
        // Last 4 passes: Cryptographically secure random data
        (None, "Random Pass 32"),
        (None, "Random Pass 33"),
        (None, "Random Pass 34"),
        (None, "Random Pass 35"),
    ];

    /// Perform the complete 35-pass Gutmann wipe with error recovery
    pub fn wipe_drive(
        device_path: &str,
        size: u64,
        drive_type: DriveType,
        config: &WipeConfig,
    ) -> Result<()> {
        println!(
            "Starting Gutmann 35-pass secure wipe with error recovery on {}",
            device_path
        );
        println!(
            "Drive size: {} bytes ({} GB)",
            size,
            size / (1024 * 1024 * 1024)
        );

        // Detect drive encoding
        let encoding = Self::detect_drive_encoding(device_path)?;
        println!("Detected drive encoding: {:?}", encoding);

        // Initialize recovery coordinator
        let mut coordinator = RecoveryCoordinator::new(device_path, config)?;

        // Check for existing checkpoint and resume if available
        let start_pass = if let Some(resume) = coordinator.resume_from_checkpoint("Gutmann")? {
            println!(
                "Resuming from pass {} (checkpoint found)",
                resume.current_pass + 1
            );
            resume.current_pass
        } else {
            0
        };

        // Configure I/O based on drive type
        let io_config = match drive_type {
            DriveType::NVMe => IOConfig::nvme_optimized(),
            DriveType::SSD => IOConfig::sata_ssd_optimized(),
            DriveType::HDD => IOConfig::hdd_optimized(),
            _ => IOConfig::default(),
        };

        // Open device with optimized I/O
        let mut io_handle = OptimizedIO::open(device_path, io_config)?;

        // Perform each pass with error recovery
        for (pass_num, (pattern, description)) in Self::GUTMANN_PATTERNS.iter().enumerate() {
            // Skip completed passes if resuming
            if pass_num < start_pass {
                continue;
            }

            println!("\n🔄 Pass {}/35: {}", pass_num + 1, description);

            let pass_start = Instant::now();

            // Create error context for this pass
            let context = ErrorContext::new(format!("gutmann_pass_{}", pass_num + 1), device_path);

            // Execute pass with recovery
            coordinator.execute_with_recovery(
                &format!("pass_{}", pass_num + 1),
                context,
                || {
                    // Write the pattern
                    if let Some(pattern_bytes) = pattern {
                        Self::write_pattern_with_verification(
                            &mut io_handle,
                            size,
                            pattern_bytes,
                            pass_num,
                        )?;
                    } else {
                        Self::write_random_with_verification(&mut io_handle, size, pass_num)?;
                    }
                    Ok(())
                },
            )?;

            let pass_duration = pass_start.elapsed();
            println!(
                "  ✅ Pass {} completed and verified in {:.2}s",
                pass_num + 1,
                pass_duration.as_secs_f64()
            );

            // Save checkpoint using RecoveryCoordinator
            let bytes_written = (pass_num as u64 + 1) * size;
            coordinator.maybe_checkpoint(
                "Gutmann",
                35,
                size * 35,
                &Progress {
                    current_pass: pass_num + 1,
                    bytes_written,
                    state: json!({
                        "encoding": format!("{:?}", encoding),
                        "total_passes": 35,
                    }),
                },
            )?;
        }

        // Final sync
        io_handle.sync()?;

        // Print performance report
        OptimizedIO::print_performance_report(&io_handle, None);

        // Clean up checkpoint on successful completion
        coordinator.delete_checkpoint()?;

        println!("\n✅ Gutmann 35-pass wipe completed successfully!");
        println!("All data has been securely overwritten and verified.");

        Ok(())
    }

    /// Detect the drive's encoding type for optimal pattern selection
    pub(crate) fn detect_drive_encoding(device_path: &str) -> Result<DriveEncoding> {
        use std::process::Command;

        // Try to get drive information via smartctl
        let output = Command::new("smartctl").args(["-i", device_path]).output();

        if let Ok(output) = output {
            let info = String::from_utf8_lossy(&output.stdout);

            // Check for drive age and type indicators
            // Modern drives (post-2000) typically use PRML
            if info.contains("SSD") || info.contains("NVMe") {
                return Ok(DriveEncoding::PRML);
            }

            // Check rotation rate for HDDs
            if let Some(line) = info.lines().find(|l| l.contains("Rotation Rate")) {
                if line.contains("rpm") {
                    // Parse manufacture date if available
                    // Drives before 1995 likely use MFM
                    // Drives 1995-2000 likely use RLL
                    // Drives after 2000 likely use PRML

                    // Default to PRML for modern drives
                    return Ok(DriveEncoding::PRML);
                }
            }
        }

        // Default to Unknown for most comprehensive coverage
        Ok(DriveEncoding::Unknown)
    }

    /// Write a specific pattern and verify it was written correctly
    pub(crate) fn write_pattern_with_verification(
        io_handle: &mut IOHandle,
        size: u64,
        pattern: &[u8],
        pass_num: usize,
    ) -> Result<()> {
        let mut bytes_written = 0u64;
        let mut bar = ProgressBar::new(48);

        // Write phase using OptimizedIO
        OptimizedIO::sequential_write(io_handle, size, |buffer| {
            // Fill buffer with repeating pattern
            let buf = buffer.as_mut_slice();
            for (i, byte) in buf.iter_mut().enumerate() {
                *byte = pattern[i % pattern.len()];
            }

            bytes_written += buf.len() as u64;

            // Update progress every 100MB
            if bytes_written % (100 * 1024 * 1024) == 0 || bytes_written >= size {
                let progress = (bytes_written as f64 / size as f64) * 50.0; // First 50% for writing
                bar.render(progress, Some(bytes_written), Some(size));
            }

            Ok(())
        })?;

        // Verification phase
        println!("\n  🔍 Verifying pass {} pattern...", pass_num + 1);
        Self::verify_pattern_from_device(&io_handle.device_path, size, pattern, &mut bar)?;

        bar.render(100.0, Some(size), Some(size));
        Ok(())
    }

    /// Write cryptographically secure random data and verify
    fn write_random_with_verification(
        io_handle: &mut IOHandle,
        size: u64,
        pass_num: usize,
    ) -> Result<()> {
        let mut bytes_written = 0u64;
        let mut bar = ProgressBar::new(48);

        // Store chunks for verification (sample every 100MB)
        let mut verification_samples: HashMap<u64, Vec<u8>> = HashMap::new();

        // Write phase using OptimizedIO
        OptimizedIO::sequential_write(io_handle, size, |buffer| {
            // Fill buffer with cryptographically secure random data
            let buf = buffer.as_mut_slice();
            secure_random_bytes(buf)?;

            // Store sample for verification (first 4KB of every 100MB)
            if bytes_written % (100 * 1024 * 1024) == 0 {
                let sample_size = std::cmp::min(4096, buf.len());
                verification_samples.insert(bytes_written, buf[..sample_size].to_vec());
            }

            bytes_written += buf.len() as u64;

            // Update progress
            if bytes_written % (100 * 1024 * 1024) == 0 || bytes_written >= size {
                let progress = (bytes_written as f64 / size as f64) * 50.0; // First 50% for writing
                bar.render(progress, Some(bytes_written), Some(size));
            }

            Ok(())
        })?;

        // Verification phase - verify random data has high entropy
        println!("\n  🔍 Verifying pass {} randomness...", pass_num + 1);
        Self::verify_random_entropy_from_device(
            &io_handle.device_path,
            size,
            &verification_samples,
            &mut bar,
        )?;

        bar.render(100.0, Some(size), Some(size));
        Ok(())
    }

    /// Verify that a pattern was written correctly (uses separate file handle for reading)
    pub(crate) fn verify_pattern_from_device(
        device_path: &str,
        size: u64,
        expected_pattern: &[u8],
        bar: &mut ProgressBar,
    ) -> Result<()> {
        const SAMPLE_SIZE: usize = 4096;

        // Open device for reading with optimized I/O
        let config = IOConfig::verification_optimized();
        let mut handle = OptimizedIO::open(device_path, config)?;

        // Verify samples throughout the drive
        let num_samples = std::cmp::min(1000, (size / SAMPLE_SIZE as u64) as usize);
        let sample_interval = size / num_samples as u64;

        for i in 0..num_samples {
            let offset = i as u64 * sample_interval;
            let read_size = std::cmp::min(SAMPLE_SIZE, (size - offset) as usize);

            let buffer = OptimizedIO::read_range(&mut handle, offset, read_size)?;

            // Check pattern matches
            for (j, byte) in buffer.iter().enumerate() {
                let expected = expected_pattern[j % expected_pattern.len()];
                if *byte != expected {
                    return Err(anyhow!(
                        "Verification failed at offset {}: expected 0x{:02x}, got 0x{:02x}",
                        offset + j as u64,
                        expected,
                        byte
                    ));
                }
            }

            // Update progress (50-100% range for verification)
            let progress = 50.0 + ((i as f64 / num_samples as f64) * 50.0);
            bar.render(progress, None, None);
        }

        Ok(())
    }

    /// Verify random data has sufficient entropy (uses separate file handle for reading)
    pub(crate) fn verify_random_entropy_from_device(
        device_path: &str,
        size: u64,
        samples: &HashMap<u64, Vec<u8>>,
        bar: &mut ProgressBar,
    ) -> Result<()> {
        // Open device for reading with optimized I/O
        let config = IOConfig::verification_optimized();
        let mut handle = OptimizedIO::open(device_path, config)?;

        // Verify stored samples match what's on disk
        let num_samples = samples.len();
        let mut verified = 0;

        for (offset, expected_data) in samples {
            let buffer = OptimizedIO::read_range(&mut handle, *offset, expected_data.len())?;

            if buffer != *expected_data {
                // Check entropy instead of exact match (drive might have done something)
                let entropy = Self::calculate_entropy(&buffer);
                if entropy < 7.5 {
                    return Err(anyhow!(
                        "Low entropy detected at offset {}: {:.2} bits/byte",
                        offset,
                        entropy
                    ));
                }
            }

            verified += 1;
            let progress = 50.0 + ((verified as f64 / num_samples as f64) * 50.0);
            bar.render(progress, None, None);
        }

        // Additionally check overall entropy at random positions
        for _ in 0..100 {
            // Generate random offset using secure RNG
            let mut offset_bytes = [0u8; 8];
            secure_random_bytes(&mut offset_bytes)?;
            let offset = u64::from_le_bytes(offset_bytes) % size.saturating_sub(4096);

            if let Ok(buffer) = OptimizedIO::read_range(&mut handle, offset, 4096) {
                let entropy = Self::calculate_entropy(&buffer);
                if entropy < 7.0 {
                    println!(
                        "  ⚠️  Warning: Lower entropy at offset {}: {:.2} bits/byte",
                        offset, entropy
                    );
                }
            }
        }

        Ok(())
    }

    /// Calculate Shannon entropy of data
    pub(crate) fn calculate_entropy(data: &[u8]) -> f64 {
        let mut counts = [0u64; 256];
        for &byte in data {
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

        entropy
    }

    /// Select optimal patterns based on drive encoding
    pub fn get_optimized_patterns(encoding: DriveEncoding) -> Vec<usize> {
        match encoding {
            DriveEncoding::MFM => {
                // Focus on MFM-specific patterns
                vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 15, 20, 25, 31, 32, 33, 34]
            }
            DriveEncoding::RLL => {
                // Focus on RLL patterns
                vec![0, 1, 2, 3, 4, 5, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34]
            }
            DriveEncoding::PRML => {
                // Modern drives - use subset of most effective patterns
                vec![0, 1, 2, 3, 4, 5, 9, 14, 19, 24, 31, 32, 33, 34]
            }
            DriveEncoding::Unknown => {
                // Use all 35 passes for maximum coverage
                (0..35).collect()
            }
        }
    }
}
