pub mod certificates;
pub mod secure_rng; // Made public for testing

#[cfg(test)]
mod secure_rng_tests;

// Re-export
pub use certificates::{CertificateGenerator, VerificationResult, WipeCertificate, WipeDetails};
pub use secure_rng::secure_random_bytes; // Export for compliance tests
