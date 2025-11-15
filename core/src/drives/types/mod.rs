// Drive type specific implementations
//
// This module organizes different drive types and their wiping implementations

// Basic drive types
pub mod hdd;
pub mod nvme;
pub mod ssd;

// Advanced drive types (Phase 1, Step 6)
pub mod emmc; // eMMC/UFS embedded storage
pub mod hybrid; // Hybrid SSHD drives
pub mod optane; // Intel Optane / 3D XPoint
pub mod raid;
pub mod smr; // Shingled Magnetic Recording // RAID array handling

// Re-exports for convenience
pub use emmc::{BootPartition, EMMCDevice, RPMBPartition, UFSDevice, UFSLogicalUnit, UserDataArea};
pub use hdd::HDDWipe;
pub use hybrid::{HDDInfo, HybridDrive, PinnedRegion, SSDCacheInfo};
pub use nvme::{NVMeAdvanced, NVMeNamespace, NVMeWipe, NamespaceType, ZNSZone, ZNSZoneState};
pub use optane::{OptaneDrive, OptaneMode, OptaneNamespace};
pub use raid::{MetadataLocation, MetadataRegion, RAIDArray, RAIDController, RAIDType};
pub use smr::{SMRDrive, Zone, ZoneCondition, ZoneModel, ZoneType};
pub use ssd::SSDWipe;
