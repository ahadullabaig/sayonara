/// Compliance Test Suite Entry Point
///
/// This file serves as the entry point for all compliance tests.
/// It includes all compliance test modules.
mod common; // Common test utilities

// Include all compliance test modules
mod compliance {
    pub mod certificate_validation;
    pub mod dod_5220_22m;
    pub mod nist_800_88;
    pub mod statistical_suite;
}
