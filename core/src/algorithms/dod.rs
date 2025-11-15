use crate::crypto::secure_rng::secure_random_bytes;
use crate::error::{ErrorContext, Progress, RecoveryCoordinator};
use crate::io::{IOConfig, IOHandle, OptimizedIO};
use crate::ui::progress::ProgressBar;
use crate::DriveType;
use crate::WipeConfig;
use crate::{DriveError, DriveResult};
use anyhow::Result;
use serde_json::json;

pub struct DoDWipe;

impl DoDWipe {
    /// DoD 5220.22-M standard pass 1 pattern (all zeros)
    pub const PASS_1_PATTERN: u8 = 0x00;

    /// DoD 5220.22-M standard pass 2 pattern (all ones)
    pub const PASS_2_PATTERN: u8 = 0xFF;

    /// DoD 5220.22-M requires exactly 3 passes
    pub const PASS_COUNT: usize = 3;

    pub fn wipe_drive(
        device_path: &str,
        size: u64,
        drive_type: DriveType,
        config: &WipeConfig,
    ) -> Result<()> {
        println!(
            "Starting DoD 5220.22-M 3-pass wipe with error recovery on {}",
            device_path
        );

        // Initialize recovery coordinator
        let mut coordinator = RecoveryCoordinator::new(device_path, config)?;

        // Check for existing checkpoint
        let start_pass = if let Some(resume) = coordinator.resume_from_checkpoint("DoD")? {
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

        // Pass 1: Write 0x00
        if start_pass == 0 {
            println!("\n🔄 Pass 1/3: Writing 0x00");
            let context = ErrorContext::new("dod_pass_1", device_path);
            coordinator.execute_with_recovery("pass_1", context, || -> DriveResult<()> {
                Self::write_pattern(&mut io_handle, size, Self::PASS_1_PATTERN).map_err(|e| {
                    DriveError::IoError(std::io::Error::other(
                        format!("{}", e),
                    ))
                })?;
                Ok(())
            })?;
            coordinator.maybe_checkpoint(
                "DoD",
                Self::PASS_COUNT,
                size * Self::PASS_COUNT as u64,
                &Progress {
                    current_pass: 1,
                    bytes_written: size,
                    state: json!({"pass": 1}),
                },
            )?;
        }

        // Pass 2: Write 0xFF
        if start_pass <= 1 {
            println!("\n🔄 Pass 2/3: Writing 0xFF");
            let context = ErrorContext::new("dod_pass_2", device_path);
            coordinator.execute_with_recovery("pass_2", context, || -> DriveResult<()> {
                Self::write_pattern(&mut io_handle, size, Self::PASS_2_PATTERN).map_err(|e| {
                    DriveError::IoError(std::io::Error::other(
                        format!("{}", e),
                    ))
                })?;
                Ok(())
            })?;
            coordinator.maybe_checkpoint(
                "DoD",
                Self::PASS_COUNT,
                size * Self::PASS_COUNT as u64,
                &Progress {
                    current_pass: 2,
                    bytes_written: size * 2,
                    state: json!({"pass": 2}),
                },
            )?;
        }

        // Pass 3: Write random data
        if start_pass <= 2 {
            println!("\n🔄 Pass 3/3: Writing random data");
            let context = ErrorContext::new("dod_pass_3", device_path);
            coordinator.execute_with_recovery("pass_3", context, || -> DriveResult<()> {
                Self::write_random(&mut io_handle, size).map_err(|e| {
                    DriveError::IoError(std::io::Error::other(
                        format!("{}", e),
                    ))
                })?;
                Ok(())
            })?;
            coordinator.maybe_checkpoint(
                "DoD",
                Self::PASS_COUNT,
                size * Self::PASS_COUNT as u64,
                &Progress {
                    current_pass: 3,
                    bytes_written: size * Self::PASS_COUNT as u64,
                    state: json!({"pass": 3}),
                },
            )?;
        }

        // Final sync
        io_handle.sync()?;

        // Print performance report
        OptimizedIO::print_performance_report(&io_handle, None);

        // Clean up checkpoint on success
        coordinator.delete_checkpoint()?;

        println!("\n✅ DoD wipe completed successfully");
        Ok(())
    }

    fn write_pattern(io_handle: &mut IOHandle, size: u64, pattern_byte: u8) -> Result<()> {
        let mut bytes_written = 0u64;
        let mut bar = ProgressBar::new(48);

        OptimizedIO::sequential_write(io_handle, size, |buffer| {
            // Fill buffer with pattern
            let buf = buffer.as_mut_slice();
            buf.fill(pattern_byte);

            bytes_written += buf.len() as u64;

            if bytes_written.is_multiple_of(50 * 1024 * 1024) || bytes_written >= size {
                let progress = (bytes_written as f64 / size as f64) * 100.0;
                bar.render(progress, Some(bytes_written), Some(size));
            }

            Ok(())
        })?;

        bar.render(100.0, Some(size), Some(size));
        Ok(())
    }

    fn write_random(io_handle: &mut IOHandle, size: u64) -> Result<()> {
        let mut bytes_written = 0u64;
        let mut bar = ProgressBar::new(48);

        OptimizedIO::sequential_write(io_handle, size, |buffer| {
            // Fill buffer with cryptographically secure random data
            let buf = buffer.as_mut_slice();
            secure_random_bytes(buf)?;

            bytes_written += buf.len() as u64;

            if bytes_written.is_multiple_of(50 * 1024 * 1024) || bytes_written >= size {
                let progress = (bytes_written as f64 / size as f64) * 100.0;
                bar.render(progress, Some(bytes_written), Some(size));
            }

            Ok(())
        })?;

        bar.render(100.0, Some(size), Some(size));
        Ok(())
    }
}
