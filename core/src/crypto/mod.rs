pub mod certificates;
pub mod secure_rng; // Made public for testing

#[cfg(test)]
mod secure_rng_tests;

// Re-export
pub use certificates::{CertificateGenerator, WipeCertificate, WipeDetails, VerificationResult};
pub use secure_rng::secure_random_bytes; // Export for compliance tests
