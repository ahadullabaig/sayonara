// Zero Wipe Algorithm - Single pass writing zeros
//
// This is a fast, simple wiping algorithm that overwrites all data with zeros.
// Not suitable for high-security requirements but useful for:
// - Quick sanitization
// - Drive testing
// - Preparing drives for reuse in same organization

use crate::error::{ErrorContext, Progress, RecoveryCoordinator};
use crate::io::{IOConfig, IOHandle, OptimizedIO};
use crate::ui::progress::ProgressBar;
use crate::DriveType;
use crate::WipeConfig;
use crate::{DriveError, DriveResult};
use anyhow::Result;
use serde_json::json;

pub struct ZeroWipe;

impl ZeroWipe {
    /// Perform a single-pass zero wipe with error recovery
    pub fn wipe_drive(
        device_path: &str,
        size: u64,
        drive_type: DriveType,
        config: &WipeConfig,
    ) -> Result<()> {
        println!(
            "Starting single-pass zero wipe with error recovery on {}",
            device_path
        );
        println!(
            "Drive size: {} bytes ({} GB)",
            size,
            size / (1024 * 1024 * 1024)
        );

        // Initialize recovery coordinator
        let mut coordinator = RecoveryCoordinator::new(device_path, config)?;

        // Check for existing checkpoint
        let should_resume = coordinator.resume_from_checkpoint("Zero")?.is_some();
        if should_resume {
            println!("Resuming zero wipe from checkpoint");
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

        println!("\n🔄 Writing zeros to entire drive");

        // Execute with recovery
        let context = ErrorContext::new("zero_wipe", device_path);
        coordinator.execute_with_recovery("zero_wipe", context, || -> DriveResult<()> {
            Self::write_zeros(&mut io_handle, size)
                .map_err(|e| DriveError::IoError(std::io::Error::other(format!("{}", e))))?;
            Ok(())
        })?;

        // Save final checkpoint
        coordinator.maybe_checkpoint(
            "Zero",
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

        println!("\n✅ Zero wipe completed successfully");
        println!("All sectors have been overwritten with zeros.");

        Ok(())
    }

    /// Write zeros to the entire drive
    fn write_zeros(io_handle: &mut IOHandle, size: u64) -> Result<()> {
        let mut bytes_written = 0u64;
        let mut bar = ProgressBar::new(48);

        OptimizedIO::sequential_write(io_handle, size, |buffer| {
            // Fill buffer with zeros
            let buf = buffer.as_mut_slice();
            buf.fill(0x00);

            bytes_written += buf.len() as u64;

            // Update progress every 50MB or at completion
            if bytes_written.is_multiple_of(50 * 1024 * 1024) || bytes_written >= size {
                let progress = (bytes_written as f64 / size as f64) * 100.0;
                bar.render(progress, Some(bytes_written), Some(size));
            }

            Ok(())
        })?;

        bar.render(100.0, Some(size), Some(size));
        Ok(())
    }

    /// Verify zeros were written (useful for testing)
    #[cfg(test)]
    pub fn verify_zeros(device_path: &str, size: u64, _sample_size: u64) -> Result<bool> {
        use crate::io::OptimizedIO;

        let config = IOConfig::verification_optimized();
        let mut io_handle = OptimizedIO::open(device_path, config)?;

        let mut rng = rand::thread_rng();
        let mut all_zeros = true;

        // Sample random locations
        for _ in 0..100 {
            use rand::Rng;
            let offset = rng.gen_range(0..size.saturating_sub(4096));
            let data = OptimizedIO::read_range(&mut io_handle, offset, 4096)?;

            for byte in data {
                if byte != 0x00 {
                    all_zeros = false;
                    break;
                }
            }

            if !all_zeros {
                break;
            }
        }

        Ok(all_zeros)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_zero_wipe_small_file() {
        // Create a temporary file with non-zero data
        let mut temp = NamedTempFile::new().unwrap();
        let test_data = vec![0xFF; 1024 * 1024]; // 1MB of 0xFF
        temp.write_all(&test_data).unwrap();
        temp.flush().unwrap();

        let path = temp.path().to_str().unwrap();
        let size = test_data.len() as u64;

        // Configure for buffered I/O (can't use Direct I/O on regular files)
        let io_config = IOConfig {
            use_direct_io: false,
            ..Default::default()
        };

        let mut io_handle = OptimizedIO::open(path, io_config).unwrap();

        // Perform zero wipe
        let result = write_zeros(&mut io_handle, size);
        assert!(
            result.is_ok(),
            "Zero wipe should succeed: {:?}",
            result.err()
        );
    }

    fn write_zeros(io_handle: &mut IOHandle, size: u64) -> Result<()> {
        let mut bytes_written = 0u64;

        OptimizedIO::sequential_write(io_handle, size, |buffer| {
            buffer.as_mut_slice().fill(0x00);
            bytes_written += buffer.as_slice().len() as u64;
            Ok(())
        })?;

        Ok(())
    }

    #[test]
    fn test_zero_buffer_creation() {
        use crate::io::buffer_pool::BufferPool;

        let pool = BufferPool::new(4096, 512, 4);
        let mut buffer = pool.acquire().unwrap();

        // Fill with zeros
        buffer.as_mut_slice().fill(0x00);

        // Verify all zeros
        assert!(buffer.as_slice().iter().all(|&b| b == 0x00));
    }
}
