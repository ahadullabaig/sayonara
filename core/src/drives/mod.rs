// Drive detection and operations module
//
// Organized structure:
// - detection.rs: Core drive detection logic
// - types/: Drive-type specific implementations (HDD, SSD, NVMe, SMR, etc.)
// - operations/: Drive operations (SMART, TRIM, HPA/DCO, SED)
// - freeze/: Freeze detection and mitigation
// - integrated_wipe.rs: OptimizedIO-integrated wipe operations for advanced drives

// Core functionality
pub mod detection;

// Tests
#[cfg(test)]
mod detection_tests;

#[cfg(test)]
mod integrated_wipe_tests;

// Drive types (organized by category)
pub mod types;

// Drive operations
pub mod operations;

// Freeze mitigation (already well-organized)
pub mod freeze;

// Integrated wipe operations (Phase 1, Step 5 - I/O Engine Integration)
pub mod integrated_wipe;

// Re-exports for backward compatibility and convenience
pub use detection::DriveDetector;

// Drive types
pub use types::{
    // Basic types
    HDDWipe,
    SSDWipe,
    NVMeWipe,

    // Advanced NVMe
    NVMeAdvanced,
    NVMeNamespace,
    NamespaceType,
    ZNSZone,
    ZNSZoneState,

    // Advanced drive types (Phase 1, Step 6)
    SMRDrive,
    Zone,
    ZoneType,
    ZoneCondition,
    ZoneModel,
    OptaneDrive,
    OptaneMode,
    OptaneNamespace,
    HybridDrive,
    HDDInfo,
    SSDCacheInfo,
    PinnedRegion,
    EMMCDevice,
    BootPartition,
    RPMBPartition,
    UserDataArea,
    UFSDevice,
    UFSLogicalUnit,
    RAIDArray,
    RAIDType,
    RAIDController,
    MetadataRegion,
    MetadataLocation,
};

// Operations
pub use operations::{
    HPADCOManager,
    SEDManager,
    TrimOperations,
    SMARTMonitor,
};

// Freeze mitigation
pub use freeze::{
    // Basic freeze mitigation
    FreezeMitigation,

    // Advanced freeze mitigation
    AdvancedFreezeMitigation,
    FreezeMitigationConfig,
    UnfreezeResult,
    FreezeInfo,

    // Freeze detection
    FreezeReason,
    FreezeDetector,

    // Strategies (optional, for direct access)
    UnfreezeStrategy,
    StrategyResult,

    // Helper functions
    FreezeMitigationStrategy,
    get_mitigation,
};

// Integrated wipe operations
pub use integrated_wipe::{
    wipe_smr_drive_integrated,
    wipe_optane_drive_integrated,
    wipe_hybrid_drive_integrated,
    wipe_emmc_drive_integrated,
    wipe_raid_array_integrated,
    wipe_nvme_advanced_integrated,
    WipeAlgorithm,
};
