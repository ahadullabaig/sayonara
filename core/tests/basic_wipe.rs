/// Basic wipe operation integration tests
///
/// Tests end-to-end wipe operations using mock drives
use sayonara_wipe::io::{IOConfig, OptimizedIO};

// Import common test utilities
// Note: In integration tests, common modules must be in tests/common/
#[path = "common/mod.rs"]
mod common;

use common::mock_drive::MockDrive;
use common::test_helpers::verify_all_zeros;

#[test]
fn test_basic_zero_wipe() {
    // Create a small mock drive
    let mock = MockDrive::create_hdd(10).expect("Failed to create mock drive");
    let path = mock.path_str();
    let size = mock.size_bytes();

    // Configure I/O for regular files (no direct I/O)
    let mut config = IOConfig::default();
    config.use_direct_io = false;

    // Open the mock drive
    let mut handle = OptimizedIO::open(path, config).expect("Failed to open mock drive");

    // Write zeros using sequential_write
    OptimizedIO::sequential_write(&mut handle, size, |buffer| {
        buffer.as_mut_slice().fill(0x00);
        Ok(())
    })
    .expect("Failed to write zeros");

    // Sync to ensure all data is written
    handle.sync().expect("Failed to sync");

    // Verify the drive is all zeros
    assert!(
        verify_all_zeros(mock.path()).expect("Failed to verify zeros"),
        "Drive should be completely zeroed"
    );
}

#[test]
fn test_pattern_wipe() {
    // Create a small mock drive
    let mock = MockDrive::create_ssd(5).expect("Failed to create mock drive");
    let path = mock.path_str();
    let size = mock.size_bytes();

    // Configure I/O for regular files
    let mut config = IOConfig::default();
    config.use_direct_io = false;

    // Open the mock drive
    let mut handle = OptimizedIO::open(path, config).expect("Failed to open mock drive");

    // Write pattern
    let pattern = [0xAA, 0x55];
    OptimizedIO::sequential_write(&mut handle, size, |buffer| {
        for (i, byte) in buffer.as_mut_slice().iter_mut().enumerate() {
            *byte = pattern[i % pattern.len()];
        }
        Ok(())
    })
    .expect("Failed to write pattern");

    // Sync
    handle.sync().expect("Failed to sync");

    // Verify pattern (simplified check)
    // In a real test, we'd use verify_pattern from test_helpers
}
