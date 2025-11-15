use crate::crypto::secure_rng::get_secure_rng;
use crate::error::{ErrorContext, Progress, RecoveryCoordinator};
use crate::io::{IOConfig, IOHandle, OptimizedIO};
use crate::ui::progress::ProgressBar;
use crate::DriveType;
use crate::WipeConfig;
use crate::{DriveError, DriveResult};
use anyhow::Result;
use serde_json::json;

pub struct RandomWipe;

impl RandomWipe {
    pub fn wipe_drive(
        device_path: &str,
        size: u64,
        drive_type: DriveType,
        config: &WipeConfig,
    ) -> Result<()> {
        println!(
            "Starting single-pass random wipe with error recovery on {}",
            device_path
        );

        // Initialize recovery coordinator
        let mut coordinator = RecoveryCoordinator::new(device_path, config)?;

        // Check for existing checkpoint
        let should_resume = coordinator.resume_from_checkpoint("Random")?.is_some();
        if should_resume {
            println!("Resuming random wipe from checkpoint");
        }

        // Configure I/O based on drive type
        let io_config = match drive_type {
            DriveType::NVMe => IOConfig::nvme_optimized(),
            DriveType::SSD => IOConfig::sata_ssd_optimized(),
            DriveType::HDD => IOConfig::hdd_optimized(),
            _ => IOConfig::default(),
        };

        // Open device with optimized I/O
        let mut io_handle = OptimizedIO::open(device_path, io_config)?;

        // Execute with recovery
        let context = ErrorContext::new("random_wipe", device_path);
        coordinator.execute_with_recovery("random_wipe", context, || -> DriveResult<()> {
            Self::write_random(&mut io_handle, size)
                .map_err(|e| DriveError::IoError(std::io::Error::other(format!("{}", e))))?;
            Ok(())
        })?;

        // Save final checkpoint
        coordinator.maybe_checkpoint(
            "Random",
            1,
            size,
            &Progress {
                current_pass: 1,
                bytes_written: size,
                state: json!({"complete": true}),
            },
        )?;

        // Final sync
        io_handle.sync()?;

        // Print performance report
        OptimizedIO::print_performance_report(&io_handle, None);

        // Clean up checkpoint
        coordinator.delete_checkpoint()?;

        println!("\n✅ Random wipe completed successfully");
        Ok(())
    }

    fn write_random(io_handle: &mut IOHandle, size: u64) -> Result<()> {
        let rng = get_secure_rng();
        let mut bytes_written = 0u64;
        let mut bar = ProgressBar::new(48);

        OptimizedIO::sequential_write(io_handle, size, |buffer| {
            // Fill buffer with cryptographically secure random data
            let buf = buffer.as_mut_slice();
            rng.fill_bytes(buf)?;

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
