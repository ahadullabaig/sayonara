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
    BootPartition,
    EMMCDevice,
    HDDInfo,
    // Basic types
    HDDWipe,
    HybridDrive,
    MetadataLocation,
    MetadataRegion,
    // Advanced NVMe
    NVMeAdvanced,
    NVMeNamespace,
    NVMeWipe,

    NamespaceType,
    OptaneDrive,
    OptaneMode,
    OptaneNamespace,
    PinnedRegion,
    RAIDArray,
    RAIDController,
    RAIDType,
    RPMBPartition,
    // Advanced drive types (Phase 1, Step 6)
    SMRDrive,
    SSDCacheInfo,
    SSDWipe,
    UFSDevice,
    UFSLogicalUnit,
    UserDataArea,
    ZNSZone,
    ZNSZoneState,

    Zone,
    ZoneCondition,
    ZoneModel,
    ZoneType,
};

// Operations
pub use operations::{HPADCOManager, SEDManager, SMARTMonitor, TrimOperations};

// Freeze mitigation
pub use freeze::{
    get_mitigation,
    // Advanced freeze mitigation
    AdvancedFreezeMitigation,
    FreezeDetector,

    FreezeInfo,

    // Basic freeze mitigation
    FreezeMitigation,

    FreezeMitigationConfig,
    // Helper functions
    FreezeMitigationStrategy,
    // Freeze detection
    FreezeReason,
    StrategyResult,

    UnfreezeResult,
    // Strategies (optional, for direct access)
    UnfreezeStrategy,
};

// Integrated wipe operations
pub use integrated_wipe::{
    wipe_emmc_drive_integrated, wipe_hybrid_drive_integrated, wipe_nvme_advanced_integrated,
    wipe_optane_drive_integrated, wipe_raid_array_integrated, wipe_smr_drive_integrated,
    WipeAlgorithm,
};
