/// Compliance Test Suite
///
/// This module contains tests validating compliance with various data sanitization standards:
/// - DoD 5220.22-M (Department of Defense wiping standard)
/// - NIST 800-88 Rev. 1 (NIST guidelines for media sanitization)
/// - NIST SP 800-22 (Statistical test suite for random and pseudorandom number generators)
/// - Certificate validation (cryptographic authenticity and tamper detection)

pub mod dod_5220_22m;
pub mod nist_800_88;
pub mod statistical_suite;
pub mod certificate_validation;
