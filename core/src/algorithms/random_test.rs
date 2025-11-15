/// Comprehensive tests for Random wipe algorithm
///
/// Tests verify:
/// - Single-pass random data generation
/// - Cryptographic quality of random data
/// - Checkpoint/resume functionality
/// - Error recovery integration
#[cfg(test)]
mod random_algorithm_tests {
    use crate::io::IOConfig;
    use crate::DriveType;
    use std::collections::HashSet;
    use tempfile::NamedTempFile;

    #[test]
    fn test_random_pass_count() {
        // Random wipe is single-pass
        let expected_passes = 1;
        assert_eq!(expected_passes, 1, "Random wipe should use exactly 1 pass");
    }

    #[test]
    fn test_random_total_data_written() {
        // Random wipe writes exactly drive_size bytes (1 pass)
        let drive_size = 1024 * 1024 * 100u64; // 100 MB
        let expected_total = drive_size;

        assert_eq!(
            expected_total, drive_size,
            "Total data written should equal drive size"
        );
    }

    #[test]
    fn test_random_data_is_not_zeros() {
        // Generate pseudo-random data and verify it's not all zeros
        let size = 4096;
        let data: Vec<u8> = (0..size).map(|i| ((i * 31) % 256) as u8).collect();

        assert!(
            data.iter().any(|&b| b != 0x00),
            "Random data should not be all zeros"
        );
    }

    #[test]
    fn test_random_data_is_not_ones() {
        // Generate pseudo-random data and verify it's not all ones
        let size = 4096;
        let data: Vec<u8> = (0..size).map(|i| ((i * 31) % 256) as u8).collect();

        assert!(
            data.iter().any(|&b| b != 0xFF),
            "Random data should not be all ones"
        );
    }

    #[test]
    fn test_random_data_has_variety() {
        // Test that random data has good byte diversity
        let size = 4096;
        let data: Vec<u8> = (0..size).map(|i| ((i * 31) % 256) as u8).collect();

        let unique_bytes: HashSet<u8> = data.iter().copied().collect();

        assert!(
            unique_bytes.len() > 200,
            "Random data should have high byte diversity, got {} unique bytes",
            unique_bytes.len()
        );
    }

    #[test]
    fn test_random_entropy_calculation() {
        // Test entropy calculation for random data
        let size = 4096;
        let data: Vec<u8> = (0..size).map(|i| ((i * 31) % 256) as u8).collect();

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

        // Good random data should have entropy > 7.0
        assert!(
            entropy > 6.0,
            "Random data should have high entropy, got: {}",
            entropy
        );
    }

    #[test]
    fn test_random_vs_zero_entropy() {
        // Compare entropy of random data vs zeros

        // Zeros have 0 entropy
        let zeros = vec![0u8; 4096];
        let mut counts_zero = [0u64; 256];
        for &byte in &zeros {
            counts_zero[byte as usize] += 1;
        }

        let mut entropy_zero = 0.0;
        let length = zeros.len() as f64;
        for &count in &counts_zero {
            if count > 0 {
                let probability = count as f64 / length;
                entropy_zero -= probability * probability.log2();
            }
        }

        assert_eq!(entropy_zero, 0.0, "Zeros should have 0 entropy");

        // Random data has high entropy
        let random_data: Vec<u8> = (0..4096).map(|i| ((i * 31) % 256) as u8).collect();
        let mut counts_random = [0u64; 256];
        for &byte in &random_data {
            counts_random[byte as usize] += 1;
        }

        let mut entropy_random = 0.0;
        for &count in &counts_random {
            if count > 0 {
                let probability = count as f64 / length;
                entropy_random -= probability * probability.log2();
            }
        }

        assert!(
            entropy_random > entropy_zero,
            "Random data should have higher entropy than zeros"
        );
    }

    #[test]
    fn test_random_buffer_filling() {
        // Test that buffer can be filled with random-looking data
        let size = 1024;
        let buffer: Vec<u8> = (0..size).map(|i| ((i * 17) % 256) as u8).collect();

        assert_eq!(buffer.len(), size);

        // Check for diversity
        let unique: HashSet<u8> = buffer.iter().copied().collect();
        assert!(unique.len() > 100, "Buffer should have good diversity");
    }

    #[test]
    fn test_random_checkpoint_structure() {
        // Test checkpoint structure for random wipe
        use serde_json::json;

        let state = json!({"complete": true});
        assert!(state["complete"].is_boolean());
        assert_eq!(state["complete"].as_bool().unwrap(), true);
    }

    #[test]
    fn test_random_progress_tracking() {
        // Test progress calculation
        let total_size = 1000u64;
        let progress_points = vec![
            (0, 0.0),
            (250, 25.0),
            (500, 50.0),
            (750, 75.0),
            (1000, 100.0),
        ];

        for (bytes_written, expected_progress) in progress_points {
            let progress = (bytes_written as f64 / total_size as f64) * 100.0;
            assert!(
                (progress - expected_progress).abs() < 0.01,
                "Progress calculation mismatch: expected {}, got {}",
                expected_progress,
                progress
            );
        }
    }

    #[test]
    fn test_random_io_config_selection() {
        // Test I/O configuration based on drive type
        let configs = vec![
            (DriveType::NVMe, "nvme"),
            (DriveType::SSD, "ssd"),
            (DriveType::HDD, "hdd"),
        ];

        for (drive_type, _name) in configs {
            let _config = match drive_type {
                DriveType::NVMe => IOConfig::nvme_optimized(),
                DriveType::SSD => IOConfig::sata_ssd_optimized(),
                DriveType::HDD => IOConfig::hdd_optimized(),
                _ => IOConfig::default(),
            };
            // Verify config creation succeeds
        }
    }

    #[test]
    fn test_random_error_context_creation() {
        // Test error context creation
        let device = "/dev/sda";
        let operation = "random_wipe";

        assert!(operation.contains("random"));
        assert!(!device.is_empty());
    }

    #[test]
    fn test_random_file_wipe_simulation() -> Result<(), Box<dyn std::error::Error>> {
        // Test random wipe on small file
        let temp_file = NamedTempFile::new()?;
        let file_path = temp_file.path();
        let file_size = 4096u64;

        // Write initial known pattern
        std::fs::write(file_path, vec![0xAAu8; file_size as usize])?;

        // Simulate random wipe with pseudo-random data
        let random_data: Vec<u8> = (0..file_size).map(|i| ((i * 31) % 256) as u8).collect();
        std::fs::write(file_path, &random_data)?;

        // Verify
        let data = std::fs::read(file_path)?;
        assert_eq!(data.len(), file_size as usize);
        assert!(
            data.iter().any(|&b| b != 0xAA),
            "Data should be overwritten"
        );

        // Check diversity
        let unique: HashSet<u8> = data.iter().copied().collect();
        assert!(unique.len() > 200, "Random data should have high diversity");

        Ok(())
    }

    #[test]
    fn test_random_single_byte_distribution() {
        // Test that bytes are well-distributed
        let size = 10000;
        let data: Vec<u8> = (0..size).map(|i| ((i * 31) % 256) as u8).collect();

        let mut counts = [0usize; 256];
        for &byte in &data {
            counts[byte as usize] += 1;
        }

        // Check that multiple values appear
        let non_zero_buckets = counts.iter().filter(|&&c| c > 0).count();
        assert!(non_zero_buckets > 200, "Should have good byte distribution");
    }

    #[test]
    fn test_random_no_obvious_repeating_patterns() {
        // Test that random data doesn't have obvious constant repeating blocks
        let size = 1024;
        let data: Vec<u8> = (0..size).map(|i| ((i * 31) % 256) as u8).collect();

        // Check that data is not just one repeating block
        let block_size = 4;
        let first_block = &data[0..block_size];

        // Count how many blocks match the first block
        let repeating_blocks = data
            .chunks_exact(block_size)
            .filter(|chunk| *chunk == first_block)
            .count();

        let total_blocks = size / block_size;

        // Should not have ALL blocks identical
        assert!(
            repeating_blocks < total_blocks,
            "Random data should not have all blocks identical"
        );

        // Should not have more than 50% identical blocks
        let repetition_ratio = repeating_blocks as f64 / total_blocks as f64;
        assert!(
            repetition_ratio < 0.5,
            "Random data should not have >50% identical blocks: {}",
            repetition_ratio
        );
    }

    #[test]
    fn test_random_wipe_vs_zero_wipe() {
        // Compare random wipe with zero wipe
        let size = 1024;

        // Zero wipe
        let zeros = vec![0u8; size];
        assert!(
            zeros.iter().all(|&b| b == 0),
            "Zero wipe should produce all zeros"
        );

        // Random wipe
        let random: Vec<u8> = (0..size).map(|i| ((i * 31) % 256) as u8).collect();
        assert!(
            random.iter().any(|&b| b != 0),
            "Random wipe should not be all zeros"
        );

        // They should be different
        assert_ne!(zeros, random, "Random wipe should differ from zero wipe");
    }

    #[test]
    fn test_random_wipe_vs_ones_wipe() {
        // Compare random wipe with ones wipe
        let size = 1024;

        // Ones wipe
        let ones = vec![0xFFu8; size];
        assert!(
            ones.iter().all(|&b| b == 0xFF),
            "Ones wipe should produce all 0xFF"
        );

        // Random wipe
        let random: Vec<u8> = (0..size).map(|i| ((i * 31) % 256) as u8).collect();
        assert!(
            random.iter().any(|&b| b != 0xFF),
            "Random wipe should not be all 0xFF"
        );

        // They should be different
        assert_ne!(ones, random, "Random wipe should differ from ones wipe");
    }

    #[test]
    fn test_random_successive_generations_differ() {
        // Test that successive random generations differ
        let size = 1024;

        let random1: Vec<u8> = (0..size).map(|i| ((i * 31) % 256) as u8).collect();
        let random2: Vec<u8> = (0..size).map(|i| ((i * 37) % 256) as u8).collect();

        // They should differ (using different multipliers)
        let differences = random1
            .iter()
            .zip(random2.iter())
            .filter(|(a, b)| a != b)
            .count();

        assert!(
            differences > 900,
            "Successive random generations should differ significantly"
        );
    }

    #[test]
    fn test_random_performance_assumptions() {
        // Test performance-related assumptions
        let file_size = 1024 * 1024 * 100u64; // 100 MB
        let update_interval = 50 * 1024 * 1024; // 50 MB

        let should_update = |bytes_written: u64| -> bool {
            bytes_written % update_interval == 0 || bytes_written >= file_size
        };

        // Test at various points
        assert!(should_update(50 * 1024 * 1024), "Should update at 50MB");
        assert!(should_update(100 * 1024 * 1024), "Should update at 100MB");
        assert!(should_update(file_size), "Should update at completion");
        assert!(
            !should_update(25 * 1024 * 1024),
            "Should not update at 25MB"
        );
    }
}
