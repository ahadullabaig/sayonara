pub mod enhanced;
mod enhanced_tests;
pub mod recovery_test;

// Re-export all verification types
pub use enhanced::{
    // Bad sector tracking
    BadSectorTracker,

    // Main verification system
    EnhancedVerification,

    // Heat map
    EntropyHeatMap,

    FileSignatureMatch,

    FilesystemMetadataResults,
    // Hidden area verification
    HiddenAreaVerification,

    // Live USB
    LiveUSBVerification,
    MFMResults,
    // Pattern analysis
    PatternAnalysis,
    PhotoRecResults,
    PostWipeAnalysis,

    PreWipeTestResults,
    RecoveryRisk,
    // Recovery simulation
    RecoverySimulationResults,
    SectorSamplingResult,

    StatisticalTests,
    TestDiskResults,
    // Verification levels
    VerificationLevel,

    // Report structures
    VerificationReport,
};
pub use recovery_test::RecoveryTest;
