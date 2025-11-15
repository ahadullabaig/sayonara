#![allow(dead_code)]
/// Builder patterns for creating mock drives
///
/// Provides fluent APIs for constructing mock drives of different types
/// with sensible defaults and easy customization.
use super::mock_drive_v2::*;
use sayonara_wipe::{DriveType, SanitizeOption};

// ============ SMR DRIVE BUILDER ============

/// Builder for SMR (Shingled Magnetic Recording) mock drives
pub struct MockSMRDriveBuilder {
    config: MockDriveConfig,
}

impl MockSMRDriveBuilder {
    pub fn new() -> Self {
        let config = MockDriveConfig {
            drive_type: DriveType::SMR,
            model: "Mock SMR Drive 100MB".to_string(),
            size: 100 * 1024 * 1024, // 100MB for fast tests
            smr_config: Some(SMRMockConfig {
                zone_model: ZoneModel::HostManaged,
                zone_size: 16 * 1024 * 1024, // 16MB zones
                conventional_zone_count: 2,
                sequential_zone_count: 6,
            }),
            ..Default::default()
        };

        Self { config }
    }

    pub fn size_mb(mut self, size_mb: u64) -> Self {
        self.config.size = size_mb * 1024 * 1024;
        self
    }

    pub fn zone_size_mb(mut self, size_mb: u64) -> Self {
        if let Some(smr) = &mut self.config.smr_config {
            smr.zone_size = size_mb * 1024 * 1024;
        }
        self
    }

    pub fn zone_model(mut self, model: ZoneModel) -> Self {
        if let Some(smr) = &mut self.config.smr_config {
            smr.zone_model = model;
        }
        self
    }

    pub fn build(self) -> std::io::Result<MockDrive> {
        MockDrive::new(self.config)
    }
}

impl Default for MockSMRDriveBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ============ OPTANE DRIVE BUILDER ============

/// Builder for Intel Optane / 3D XPoint mock drives
pub struct MockOptaneDriveBuilder {
    config: MockDriveConfig,
}

impl MockOptaneDriveBuilder {
    pub fn new() -> Self {
        let config = MockDriveConfig {
            drive_type: DriveType::Optane,
            model: "Mock Intel Optane 100MB".to_string(),
            size: 100 * 1024 * 1024, // 100MB for fast tests
            supports_crypto_erase: true,
            max_temperature: 85, // Optane runs hotter
            optane_config: Some(OptaneMockConfig {
                is_pmem: false,
                supports_ise: true,
                generation: "P4800X".to_string(),
                namespace_count: 1,
            }),
            ..Default::default()
        };

        Self { config }
    }

    pub fn size_gb(mut self, size_gb: u64) -> Self {
        self.config.size = size_gb * 1024 * 1024 * 1024;
        self
    }

    pub fn enable_ise(mut self, enable: bool) -> Self {
        if let Some(optane) = &mut self.config.optane_config {
            optane.supports_ise = enable;
        }
        self
    }

    pub fn pmem_mode(mut self, pmem: bool) -> Self {
        if let Some(optane) = &mut self.config.optane_config {
            optane.is_pmem = pmem;
        }
        self
    }

    pub fn build(self) -> std::io::Result<MockDrive> {
        MockDrive::new(self.config)
    }
}

impl Default for MockOptaneDriveBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ============ HYBRID DRIVE BUILDER ============

/// Builder for Hybrid SSHD mock drives
pub struct MockHybridDriveBuilder {
    config: MockDriveConfig,
}

impl MockHybridDriveBuilder {
    pub fn new() -> Self {
        let config = MockDriveConfig {
            drive_type: DriveType::HybridSSHD,
            model: "Mock Hybrid SSHD 110MB".to_string(),
            size: 110 * 1024 * 1024, // 110MB total for fast tests
            hybrid_config: Some(HybridMockConfig {
                hdd_capacity: 100 * 1024 * 1024,  // 100MB HDD
                ssd_cache_size: 10 * 1024 * 1024, // 10MB SSD cache
            }),
            ..Default::default()
        };

        Self { config }
    }

    pub fn hdd_size_gb(mut self, size_gb: u64) -> Self {
        if let Some(hybrid) = &mut self.config.hybrid_config {
            hybrid.hdd_capacity = size_gb * 1024 * 1024 * 1024;
            self.config.size = hybrid.hdd_capacity + hybrid.ssd_cache_size;
        }
        self
    }

    pub fn cache_size_gb(mut self, size_gb: u64) -> Self {
        if let Some(hybrid) = &mut self.config.hybrid_config {
            hybrid.ssd_cache_size = size_gb * 1024 * 1024 * 1024;
            self.config.size = hybrid.hdd_capacity + hybrid.ssd_cache_size;
        }
        self
    }

    pub fn build(self) -> std::io::Result<MockDrive> {
        MockDrive::new(self.config)
    }
}

impl Default for MockHybridDriveBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ============ eMMC DRIVE BUILDER ============

/// Builder for eMMC embedded storage mock drives
pub struct MockEMMCDriveBuilder {
    config: MockDriveConfig,
}

impl MockEMMCDriveBuilder {
    pub fn new() -> Self {
        let config = MockDriveConfig {
            drive_type: DriveType::EMMC,
            model: "Mock eMMC 100MB".to_string(),
            size: 100 * 1024 * 1024, // 100MB for fast tests
            supports_trim: true,
            emmc_config: Some(EMMCMockConfig {
                boot_partition_size: 4 * 1024 * 1024, // 4MB boot partitions
                rpmb_size: 512 * 1024,                // 512KB RPMB
                emmc_version: "5.1".to_string(),
            }),
            ..Default::default()
        };

        Self { config }
    }

    pub fn size_gb(mut self, size_gb: u64) -> Self {
        self.config.size = size_gb * 1024 * 1024 * 1024;
        self
    }

    pub fn boot_partition_mb(mut self, size_mb: u64) -> Self {
        if let Some(emmc) = &mut self.config.emmc_config {
            emmc.boot_partition_size = size_mb * 1024 * 1024;
        }
        self
    }

    pub fn version(mut self, version: String) -> Self {
        if let Some(emmc) = &mut self.config.emmc_config {
            emmc.emmc_version = version;
        }
        self
    }

    pub fn build(self) -> std::io::Result<MockDrive> {
        MockDrive::new(self.config)
    }
}

impl Default for MockEMMCDriveBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ============ NVME DRIVE BUILDER ============

/// Builder for NVMe mock drives (advanced features)
pub struct MockNVMeDriveBuilder {
    config: MockDriveConfig,
}

impl MockNVMeDriveBuilder {
    pub fn new() -> Self {
        let config = MockDriveConfig {
            drive_type: DriveType::NVMe,
            model: "Mock NVMe SSD 100MB".to_string(),
            size: 100 * 1024 * 1024, // 100MB for fast tests
            supports_trim: true,
            nvme_config: Some(NVMeMockConfig {
                namespace_count: 1,
                supports_sanitize: true,
                sanitize_crypto_erase: true,
                sanitize_block_erase: true,
                supports_format_nvm: true,
            }),
            sanitize_options: vec![SanitizeOption::CryptoErase, SanitizeOption::BlockErase],
            ..Default::default()
        };

        Self { config }
    }

    pub fn size_gb(mut self, size_gb: u64) -> Self {
        self.config.size = size_gb * 1024 * 1024 * 1024;
        self
    }

    pub fn namespace_count(mut self, count: u32) -> Self {
        if let Some(nvme) = &mut self.config.nvme_config {
            nvme.namespace_count = count;
        }
        self
    }

    pub fn enable_sanitize(mut self, enable: bool) -> Self {
        if let Some(nvme) = &mut self.config.nvme_config {
            nvme.supports_sanitize = enable;
        }
        self
    }

    pub fn build(self) -> std::io::Result<MockDrive> {
        MockDrive::new(self.config)
    }
}

impl Default for MockNVMeDriveBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ============ RAID ARRAY BUILDER ============

/// Builder for RAID array mock drives
pub struct MockRAIDArrayBuilder {
    config: MockDriveConfig,
}

impl MockRAIDArrayBuilder {
    pub fn new() -> Self {
        let config = MockDriveConfig {
            drive_type: DriveType::RAID,
            model: "Mock RAID5 Array 100MB".to_string(),
            size: 100 * 1024 * 1024, // 100MB for fast tests
            raid_config: Some(RAIDMockConfig {
                raid_level: "raid5".to_string(),
                member_count: 4,
                array_uuid: "12345678-1234-1234-1234-123456789abc".to_string(),
            }),
            ..Default::default()
        };

        Self { config }
    }

    pub fn raid_level(mut self, level: String) -> Self {
        if let Some(raid) = &mut self.config.raid_config {
            raid.raid_level = level;
        }
        self
    }

    pub fn member_count(mut self, count: u32) -> Self {
        if let Some(raid) = &mut self.config.raid_config {
            raid.member_count = count;
        }
        self
    }

    pub fn array_size_gb(mut self, size_gb: u64) -> Self {
        self.config.size = size_gb * 1024 * 1024 * 1024;
        self
    }

    pub fn build(self) -> std::io::Result<MockDrive> {
        MockDrive::new(self.config)
    }
}

impl Default for MockRAIDArrayBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ============ UFS DRIVE BUILDER ============

/// Builder for UFS (Universal Flash Storage) mock drives
pub struct MockUFSDriveBuilder {
    config: MockDriveConfig,
}

impl MockUFSDriveBuilder {
    pub fn new() -> Self {
        let config = MockDriveConfig {
            drive_type: DriveType::UFS,
            model: "Mock UFS 3.1 100MB".to_string(),
            size: 100 * 1024 * 1024, // 100MB for fast tests
            supports_trim: true,
            supports_crypto_erase: true,
            ..Default::default()
        };

        Self { config }
    }

    pub fn size_gb(mut self, size_gb: u64) -> Self {
        self.config.size = size_gb * 1024 * 1024 * 1024;
        self
    }

    pub fn build(self) -> std::io::Result<MockDrive> {
        MockDrive::new(self.config)
    }
}

impl Default for MockUFSDriveBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ============ CONVENIENCE CONSTRUCTORS ============

impl MockDrive {
    /// Create mock SMR drive with default configuration
    pub fn smr() -> std::io::Result<Self> {
        MockSMRDriveBuilder::new().size_mb(100).build() // Small for fast tests
    }

    /// Create mock Optane drive with default configuration
    pub fn optane() -> std::io::Result<Self> {
        MockOptaneDriveBuilder::new().build() // Uses builder default (100MB)
    }

    /// Create mock Hybrid SSHD with default configuration
    pub fn hybrid() -> std::io::Result<Self> {
        // Create a small hybrid drive for testing (100MB HDD + 10MB cache)
        let config = MockDriveConfig {
            drive_type: DriveType::HybridSSHD,
            model: "Mock Hybrid SSHD 110MB".to_string(),
            size: 110 * 1024 * 1024, // 110MB total
            hybrid_config: Some(HybridMockConfig {
                hdd_capacity: 100 * 1024 * 1024,  // 100MB HDD
                ssd_cache_size: 10 * 1024 * 1024, // 10MB cache
            }),
            ..Default::default()
        };
        MockDrive::new(config)
    }

    /// Create mock eMMC drive with default configuration
    pub fn emmc() -> std::io::Result<Self> {
        MockEMMCDriveBuilder::new().build() // Uses builder default (100MB)
    }

    /// Create mock NVMe drive with default configuration
    pub fn nvme() -> std::io::Result<Self> {
        MockNVMeDriveBuilder::new().build() // Uses builder default (100MB)
    }

    /// Create mock RAID array with default configuration
    pub fn raid() -> std::io::Result<Self> {
        MockRAIDArrayBuilder::new().build() // Uses builder default (100MB)
    }

    /// Create mock UFS drive with default configuration
    pub fn ufs() -> std::io::Result<Self> {
        MockUFSDriveBuilder::new().build() // Uses builder default (100MB)
    }

    /// Create basic mock HDD
    pub fn hdd(size_mb: u64) -> std::io::Result<Self> {
        let config = MockDriveConfig {
            drive_type: DriveType::HDD,
            size: size_mb * 1024 * 1024,
            model: format!("Mock HDD {}MB", size_mb),
            ..Default::default()
        };
        MockDrive::new(config)
    }

    /// Create basic mock SSD
    pub fn ssd(size_mb: u64) -> std::io::Result<Self> {
        let config = MockDriveConfig {
            drive_type: DriveType::SSD,
            size: size_mb * 1024 * 1024,
            model: format!("Mock SSD {}MB", size_mb),
            supports_trim: true,
            ..Default::default()
        };
        MockDrive::new(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convenience_constructors() -> std::io::Result<()> {
        // Test that convenience constructors create drives with small sizes
        let hdd = MockDrive::hdd(10)?;
        assert_eq!(hdd.config.drive_type, DriveType::HDD);
        assert_eq!(hdd.config.size, 10 * 1024 * 1024);

        let ssd = MockDrive::ssd(10)?;
        assert_eq!(ssd.config.drive_type, DriveType::SSD);
        assert!(ssd.config.supports_trim);
        assert_eq!(ssd.config.size, 10 * 1024 * 1024);

        Ok(())
    }

    #[test]
    fn test_builder_configuration() {
        // Test builders set correct configuration without actually building
        let smr_builder = MockSMRDriveBuilder::new();
        assert_eq!(smr_builder.config.drive_type, DriveType::SMR);
        assert!(smr_builder.config.smr_config.is_some());

        let optane_builder = MockOptaneDriveBuilder::new();
        assert_eq!(optane_builder.config.drive_type, DriveType::Optane);
        assert!(optane_builder.config.optane_config.is_some());
    }
}
