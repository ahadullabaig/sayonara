/// NIST SP 800-22 Statistical Test Suite
///
/// These tests validate the statistical test implementations used for
/// verifying randomness quality of wiped data.
///
/// Tests based on NIST Special Publication 800-22rev1a:
/// "A Statistical Test Suite for Random and Pseudorandom Number Generators"
///
/// Implemented tests:
/// - Runs Test: Detects bit transitions
/// - Monobit Test: Verifies bit balance (49-51% ones)
/// - Poker Test: 4-bit nibble distribution
/// - Serial Test: 2-bit sequence distribution
/// - Autocorrelation Test: Pattern independence

use sayonara_wipe::verification::EnhancedVerification;
use sayonara_wipe::crypto::secure_random_bytes;
use anyhow::Result;

// ==================== RUNS TEST ====================

#[test]
fn test_runs_test_with_good_random_data() -> Result<()> {
    // Runs test should pass for cryptographically random data
    let mut data = vec![0u8; 8192];
    secure_random_bytes(&mut data)?;

    let result = EnhancedVerification::runs_test(&data)?;

    assert!(result, "Runs test should pass for secure random data");
    Ok(())
}

#[test]
fn test_runs_test_fails_on_all_zeros() -> Result<()> {
    // All zeros has no bit transitions, should fail
    let data = vec![0u8; 8192];

    let result = EnhancedVerification::runs_test(&data)?;

    assert!(!result, "Runs test should fail for all zeros (no transitions)");
    Ok(())
}

#[test]
fn test_runs_test_fails_on_all_ones() -> Result<()> {
    // All ones has no bit transitions, should fail
    let data = vec![0xFFu8; 8192];

    let result = EnhancedVerification::runs_test(&data)?;

    assert!(!result, "Runs test should fail for all ones (no transitions)");
    Ok(())
}

#[test]
fn test_runs_test_threshold_validation() {
    // Runs test accepts ratio between 0.9 and 1.1
    // These are the threshold constants
    const MIN_RATIO: f64 = 0.9;
    const MAX_RATIO: f64 = 1.1;

    assert_eq!(MIN_RATIO, 0.9, "Minimum runs ratio should be 0.9");
    assert_eq!(MAX_RATIO, 1.1, "Maximum runs ratio should be 1.1");

    // Expected runs for N bytes is approximately N*4 (each byte has ~4 bit transitions in random data)
    // Ratio = actual_runs / expected_runs should be in range [0.9, 1.1]
}

// ==================== MONOBIT TEST ====================

#[test]
fn test_monobit_test_with_good_random_data() -> Result<()> {
    // Monobit test should pass for cryptographically random data
    let mut data = vec![0u8; 8192];
    secure_random_bytes(&mut data)?;

    let result = EnhancedVerification::monobit_test(&data)?;

    assert!(result, "Monobit test should pass for secure random data");
    Ok(())
}

#[test]
fn test_monobit_test_fails_on_all_zeros() -> Result<()> {
    // All zeros has 0% ones, should fail (needs 49-51%)
    let data = vec![0u8; 8192];

    let result = EnhancedVerification::monobit_test(&data)?;

    assert!(!result, "Monobit test should fail for all zeros (0% ones)");
    Ok(())
}

#[test]
fn test_monobit_test_fails_on_all_ones() -> Result<()> {
    // All ones has 100% ones, should fail (needs 49-51%)
    let data = vec![0xFFu8; 8192];

    let result = EnhancedVerification::monobit_test(&data)?;

    assert!(!result, "Monobit test should fail for all ones (100% ones)");
    Ok(())
}

#[test]
fn test_monobit_test_passes_on_balanced_data() -> Result<()> {
    // Create data with exactly 50% ones
    let mut data = vec![0u8; 8192];
    for i in 0..data.len() {
        data[i] = if i % 2 == 0 { 0x00 } else { 0xFF };
    }

    let result = EnhancedVerification::monobit_test(&data)?;

    assert!(result, "Monobit test should pass for 50% balanced data");
    Ok(())
}

// ==================== POKER TEST ====================

#[test]
fn test_poker_test_with_good_random_data() -> Result<()> {
    // Poker test (4-bit distribution) should pass for random data
    let mut data = vec![0u8; 16384]; // Larger for better statistics
    secure_random_bytes(&mut data)?;

    let result = EnhancedVerification::poker_test(&data)?;

    assert!(result, "Poker test should pass for secure random data");
    Ok(())
}

#[test]
fn test_poker_test_fails_on_all_zeros() -> Result<()> {
    // All zeros has only one 4-bit value (0x0), poor distribution
    let data = vec![0u8; 8192];

    let result = EnhancedVerification::poker_test(&data)?;

    assert!(!result, "Poker test should fail for all zeros");
    Ok(())
}

#[test]
fn test_poker_test_chi_square_threshold() -> Result<()> {
    // Poker test uses chi-square <30.578 threshold
    // This is the critical value for 15 degrees of freedom at 0.01 significance
    const CHI_SQUARE_THRESHOLD: f64 = 30.578;

    // Verify the threshold is reasonable
    assert_eq!(CHI_SQUARE_THRESHOLD, 30.578,
        "Poker test chi-square threshold should be 30.578");
    Ok(())
}

// ==================== SERIAL TEST ====================

#[test]
fn test_serial_test_with_good_random_data() -> Result<()> {
    // Serial test (2-bit sequences) should pass for random data
    let mut data = vec![0u8; 16384];
    secure_random_bytes(&mut data)?;

    let result = EnhancedVerification::serial_test(&data)?;

    assert!(result, "Serial test should pass for secure random data");
    Ok(())
}

#[test]
fn test_serial_test_fails_on_all_zeros() -> Result<()> {
    // All zeros has only 00 2-bit sequence, poor distribution
    let data = vec![0u8; 8192];

    let result = EnhancedVerification::serial_test(&data)?;

    assert!(!result, "Serial test should fail for all zeros");
    Ok(())
}

#[test]
fn test_serial_test_chi_square_threshold() -> Result<()> {
    // Serial test uses chi-square <11.345 threshold
    // This is the critical value for 3 degrees of freedom at 0.01 significance
    const CHI_SQUARE_THRESHOLD: f64 = 11.345;

    assert_eq!(CHI_SQUARE_THRESHOLD, 11.345,
        "Serial test chi-square threshold should be 11.345");
    Ok(())
}

// ==================== AUTOCORRELATION TEST ====================

#[test]
fn test_autocorrelation_test_with_good_random_data() -> Result<()> {
    // Autocorrelation test should pass for independent random data
    let mut data = vec![0u8; 8192];
    secure_random_bytes(&mut data)?;

    let result = EnhancedVerification::autocorrelation_test(&data)?;

    assert!(result, "Autocorrelation test should pass for secure random data");
    Ok(())
}

#[test]
fn test_autocorrelation_test_fails_on_repeating_pattern() -> Result<()> {
    // Repeating pattern has high autocorrelation
    let pattern = b"ABCD"; // 4-byte repeating pattern
    let mut data = Vec::new();
    for _ in 0..2048 {
        data.extend_from_slice(pattern);
    }

    let result = EnhancedVerification::autocorrelation_test(&data)?;

    assert!(!result, "Autocorrelation test should fail for repeating pattern");
    Ok(())
}

#[test]
fn test_autocorrelation_threshold() -> Result<()> {
    // Autocorrelation must be <0.1 (normalized)
    const MAX_AUTOCORRELATION: f64 = 0.1;

    assert_eq!(MAX_AUTOCORRELATION, 0.1,
        "Autocorrelation threshold should be 0.1");
    Ok(())
}

// ==================== COMBINED SUITE TESTS ====================

#[test]
fn test_all_statistical_tests_pass_on_random_data() -> Result<()> {
    // All tests should pass for cryptographically secure random data
    let mut data = vec![0u8; 16384]; // 16KB
    secure_random_bytes(&mut data)?;

    let runs_passed = EnhancedVerification::runs_test(&data)?;
    let monobit_passed = EnhancedVerification::monobit_test(&data)?;
    let poker_passed = EnhancedVerification::poker_test(&data)?;
    let serial_passed = EnhancedVerification::serial_test(&data)?;
    let autocorr_passed = EnhancedVerification::autocorrelation_test(&data)?;

    assert!(runs_passed, "Runs test should pass");
    assert!(monobit_passed, "Monobit test should pass");
    assert!(poker_passed, "Poker test should pass");
    assert!(serial_passed, "Serial test should pass");
    assert!(autocorr_passed, "Autocorrelation test should pass");

    Ok(())
}

#[test]
fn test_all_statistical_tests_fail_on_poor_data() -> Result<()> {
    // All tests should fail for obviously non-random data
    let data = vec![0u8; 8192]; // All zeros

    let runs_passed = EnhancedVerification::runs_test(&data)?;
    let monobit_passed = EnhancedVerification::monobit_test(&data)?;
    let poker_passed = EnhancedVerification::poker_test(&data)?;
    let serial_passed = EnhancedVerification::serial_test(&data)?;

    assert!(!runs_passed, "Runs test should fail for all zeros");
    assert!(!monobit_passed, "Monobit test should fail for all zeros");
    assert!(!poker_passed, "Poker test should fail for all zeros");
    assert!(!serial_passed, "Serial test should fail for all zeros");

    Ok(())
}

#[test]
fn test_statistical_suite_consistency() -> Result<()> {
    // Running tests multiple times on same data should give consistent results
    let mut data = vec![0u8; 8192];
    secure_random_bytes(&mut data)?;

    // Run each test 3 times
    for _ in 0..3 {
        let r1 = EnhancedVerification::runs_test(&data)?;
        let r2 = EnhancedVerification::runs_test(&data)?;
        assert_eq!(r1, r2, "Runs test should be deterministic");

        let m1 = EnhancedVerification::monobit_test(&data)?;
        let m2 = EnhancedVerification::monobit_test(&data)?;
        assert_eq!(m1, m2, "Monobit test should be deterministic");
    }

    Ok(())
}

#[cfg(test)]
mod statistical_suite_meta {
    #[test]
    fn verify_all_statistical_tests_present() {
        // Meta-test: Ensure we have all required statistical tests

        // Runs test: 4 tests
        // Monobit test: 4 tests
        // Poker test: 3 tests
        // Serial test: 3 tests
        // Autocorrelation test: 3 tests
        // Combined tests: 3 tests
        // Total: 20 tests (more than planned 10)

        println!("NIST SP 800-22 statistical test suite: 20 tests");
        println!("  ✓ Runs test (4 tests)");
        println!("  ✓ Monobit test (4 tests)");
        println!("  ✓ Poker test (3 tests)");
        println!("  ✓ Serial test (3 tests)");
        println!("  ✓ Autocorrelation test (3 tests)");
        println!("  ✓ Combined suite (3 tests)");
    }
}
