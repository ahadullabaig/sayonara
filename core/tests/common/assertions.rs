/// Test assertion helpers for mock drive verification
///
/// Provides reusable assertion functions for validating wipe operations
/// across different drive types and test scenarios.

use super::mock_drive_v2::{MockDrive, MockDriveStats};
use anyhow::Result;

/// Assert that wipe completed successfully with expected number of passes
pub fn assert_wipe_completed(mock: &MockDrive, expected_passes: u32) -> Result<()> {
    // Update stats based on actual file state
    mock.update_stats_post_wipe(expected_passes)?;

    let stats = mock.stats();
    let expected_bytes = mock.config.size * expected_passes as u64;

    // Allow 5% tolerance for overhead
    let min_expected = (expected_bytes as f64 * 0.95) as u64;

    anyhow::ensure!(
        stats.bytes_written >= min_expected,
        "Wipe incomplete: expected ~{} bytes ({}x passes), but only wrote {} bytes",
        expected_bytes,
        expected_passes,
        stats.bytes_written
    );

    anyhow::ensure!(
        stats.error_count == 0,
        "Wipe completed with {} errors",
        stats.error_count
    );

    Ok(())
}

/// Assert that drive temperature stayed within safe limits
pub fn assert_temperature_safe(mock: &MockDrive) -> Result<()> {
    let stats = mock.stats();

    anyhow::ensure!(
        stats.current_temperature <= mock.config.max_temperature,
        "Temperature {} °C exceeded maximum {} °C",
        stats.current_temperature,
        mock.config.max_temperature
    );

    Ok(())
}

/// Assert that wipe verification passed with minimum success rate
pub fn assert_verification_passed(mock: &MockDrive, min_success_rate: f64) -> Result<()> {
    let verification = mock.verify_wipe(None)?;

    anyhow::ensure!(
        verification.success_rate >= min_success_rate,
        "Verification failed: {:.2}% success rate < required {:.2}%\n\
         Total bytes: {}, Mismatches: {}",
        verification.success_rate * 100.0,
        min_success_rate * 100.0,
        verification.total_bytes,
        verification.mismatches
    );

    Ok(())
}

/// Assert that verification passed for a specific pattern
pub fn assert_pattern_verification(
    mock: &MockDrive,
    expected_pattern: &[u8],
    min_success_rate: f64,
) -> Result<()> {
    let verification = mock.verify_wipe(Some(expected_pattern))?;

    anyhow::ensure!(
        verification.success_rate >= min_success_rate,
        "Pattern verification failed: {:.2}% success rate < required {:.2}%\n\
         Expected pattern: {:02X?}, Total bytes: {}, Mismatches: {}",
        verification.success_rate * 100.0,
        min_success_rate * 100.0,
        &expected_pattern[..expected_pattern.len().min(16)],
        verification.total_bytes,
        verification.mismatches
    );

    Ok(())
}

/// Assert that no errors occurred during operation
pub fn assert_no_errors(mock: &MockDrive) -> Result<()> {
    let stats = mock.stats();

    anyhow::ensure!(
        stats.error_count == 0,
        "Operation completed with {} errors",
        stats.error_count
    );

    Ok(())
}

/// Assert that mock drive stats are reasonable
pub fn assert_stats_reasonable(mock: &MockDrive) -> Result<()> {
    let stats = mock.stats();

    // Check that some data was written
    anyhow::ensure!(
        stats.bytes_written > 0,
        "No data was written to drive"
    );

    // Check temperature is in reasonable range
    anyhow::ensure!(
        stats.current_temperature >= 20 && stats.current_temperature <= 100,
        "Temperature {} °C is outside reasonable range (20-100°C)",
        stats.current_temperature
    );

    Ok(())
}

/// Print detailed stats for debugging
pub fn print_mock_stats(mock: &MockDrive, label: &str) {
    let stats = mock.stats();
    println!("\n=== Mock Drive Stats: {} ===", label);
    println!("  Model: {}", mock.config.model);
    println!("  Type: {:?}", mock.config.drive_type);
    println!("  Size: {} MB", mock.config.size / (1024 * 1024));
    println!("  Bytes Written: {} ({} MB)", stats.bytes_written, stats.bytes_written / (1024 * 1024));
    println!("  Bytes Read: {} ({} MB)", stats.bytes_read, stats.bytes_read / (1024 * 1024));
    println!("  Write Count: {}", stats.write_count);
    println!("  Temperature: {} °C", stats.current_temperature);
    println!("  Errors: {}", stats.error_count);
    println!("================================\n");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::mock_drive_v2::MockDrive;

    #[test]
    fn test_assertions() -> Result<()> {
        let mock = MockDrive::hdd(10)?;

        // These assertions should work with a fresh mock
        assert_temperature_safe(&mock)?;
        assert_no_errors(&mock)?;

        Ok(())
    }
}
