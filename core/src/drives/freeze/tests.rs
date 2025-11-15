// Comprehensive tests for advanced freeze mitigation

#[cfg(test)]
mod tests {
    use tempfile::TempDir;
    use crate::drives::{AdvancedFreezeMitigation, FreezeDetector, FreezeInfo,
                        FreezeMitigationConfig, FreezeReason, UnfreezeResult};
    use crate::drives::freeze::advanced::SuccessHistory;

    #[test]
    fn test_config_default_values() {
        let config = FreezeMitigationConfig::default();

        assert_eq!(config.max_attempts_duration, 300);
        assert!(config.allow_kernel_module);
        assert!(config.allow_vendor_specific);
        assert!(!config.allow_ipmi); // Conservative default
        assert!(!config.allow_acpi_sleep);
        assert_eq!(config.retry_delay_ms, 2000);
    }

    #[test]
    fn test_config_custom_values() {
        let config = FreezeMitigationConfig {
            max_attempts_duration: 600,
            allow_kernel_module: false,
            allow_ipmi: true,
            allow_acpi_sleep: true,
            allow_vendor_specific: false,
            retry_delay_ms: 5000,
        };

        assert_eq!(config.max_attempts_duration, 600);
        assert!(!config.allow_kernel_module);
        assert!(config.allow_ipmi);
    }

    #[test]
    fn test_freeze_reason_descriptions() {
        let reasons = vec![
            FreezeReason::BiosSetFrozen,
            FreezeReason::RaidController,
            FreezeReason::OsSecurity,
            FreezeReason::ControllerPolicy,
            FreezeReason::Unknown,
        ];

        for reason in reasons {
            let desc = FreezeDetector::describe_reason(&reason);
            assert!(!desc.is_empty());
            assert!(desc.len() > 50); // Should be detailed
        }
    }

    #[test]
    fn test_success_history_recording() {
        let mut history = SuccessHistory::new();

        // Record multiple successes
        history.record_success("SATA Link Reset");
        history.record_success("SATA Link Reset");
        history.record_success("SATA Link Reset");
        history.record_success("PCIe Hot Reset");

        // Verify counts
        assert_eq!(history.get_priority("SATA Link Reset"), 3);
        assert_eq!(history.get_priority("PCIe Hot Reset"), 1);
        assert_eq!(history.get_priority("Unknown Method"), 0);
    }

    #[test]
    fn test_success_history_persistence() {
        let tmp_dir = TempDir::new().unwrap();

        // Create a mock that saves to temp dir instead
        let test_path = tmp_dir.path().join("freeze_history.json");

        let mut history = SuccessHistory::new();
        history.record_success("Test Method");

        // Manually save to test path instead of using the built-in save
        let json = serde_json::to_string_pretty(&history).unwrap();
        std::fs::write(&test_path, json).unwrap();

        // Verify file was created
        assert!(test_path.exists());
    }

    #[test]
    fn test_strategy_prioritization() {
        let config = FreezeMitigationConfig::default();
        let mitigation = AdvancedFreezeMitigation::new(config);

        // Strategies should be ordered by historical success
        let strategy_names: Vec<String> = mitigation.strategies
            .iter()
            .map(|s| s.name().to_string())
            .collect();

        // Should have multiple strategies
        assert!(strategy_names.len() >= 3);

        // Should include key strategies
        let has_sata = strategy_names.iter().any(|n| n.contains("SATA"));
        let has_vendor = strategy_names.iter().any(|n| n.contains("Vendor"));

        assert!(has_sata || has_vendor);
    }

    #[test]
    fn test_freeze_info_structure() {
        let info = FreezeInfo {
            status: crate::FreezeStatus::Frozen,
            reason: Some(FreezeReason::BiosSetFrozen),
            compatible_strategies: vec![
                "SATA Link Reset".to_string(),
                "ACPI Sleep".to_string(),
            ],
            estimated_success_rate: 0.85,
        };

        assert_eq!(info.status, crate::FreezeStatus::Frozen);
        assert!(info.reason.is_some());
        assert_eq!(info.compatible_strategies.len(), 2);
        assert!(info.estimated_success_rate > 0.0);
        assert!(info.estimated_success_rate <= 1.0);
    }

    #[test]
    fn test_unfreeze_result_structure() {
        let result = UnfreezeResult {
            success: true,
            method_used: "SATA Link Reset".to_string(),
            attempts_made: 2,
            time_taken: std::time::Duration::from_secs(15),
            freeze_reason: Some(FreezeReason::BiosSetFrozen),
            warnings: vec!["Minor warning".to_string()],
        };

        assert!(result.success);
        assert_eq!(result.attempts_made, 2);
        assert_eq!(result.warnings.len(), 1);
        assert!(result.time_taken.as_secs() > 0);
    }

    #[test]
    fn test_compatibility_matrix() {
        use crate::drives::freeze::strategies::*;

        let sata = SataLinkReset::new();
        let vendor = VendorSpecific::new();
        let kernel = KernelModule::new();

        // SATA compatible with BIOS and controller freeze
        assert!(sata.is_compatible_with(&FreezeReason::BiosSetFrozen));
        assert!(sata.is_compatible_with(&FreezeReason::ControllerPolicy));
        assert!(!sata.is_compatible_with(&FreezeReason::RaidController));

        // Vendor specific compatible with RAID
        assert!(vendor.is_compatible_with(&FreezeReason::RaidController));
        assert!(vendor.is_compatible_with(&FreezeReason::ControllerPolicy));

        // Kernel module compatible with everything (nuclear option)
        assert!(kernel.is_compatible_with(&FreezeReason::BiosSetFrozen));
        assert!(kernel.is_compatible_with(&FreezeReason::RaidController));
        assert!(kernel.is_compatible_with(&FreezeReason::Unknown));
    }

    #[test]
    fn test_risk_levels() {
        use crate::drives::freeze::strategies::*;

        // Low risk strategies
        assert!(SataLinkReset::new().risk_level() <= 3);
        assert!(UsbSuspend::new().risk_level() <= 3);

        // Medium risk strategies
        assert!(VendorSpecific::new().risk_level() >= 5);
        assert!(VendorSpecific::new().risk_level() <= 7);

        // High risk strategies
        assert!(KernelModule::new().risk_level() >= 7);
        assert!(AcpiSleep::new().risk_level() >= 8);

        // Maximum risk
        assert_eq!(IpmiPower::new().risk_level(), 10);
    }

    #[test]
    fn test_estimated_durations() {
        use crate::drives::freeze::strategies::*;

        // Quick strategies
        assert!(SataLinkReset::new().estimated_duration() <= 10);

        // Medium duration
        assert!(VendorSpecific::new().estimated_duration() <= 20);
        assert!(KernelModule::new().estimated_duration() <= 30);

        // Long duration (system reboot)
        assert!(IpmiPower::new().estimated_duration() >= 60);
    }

    #[test]
    fn test_strategy_descriptions() {
        use crate::drives::freeze::strategies::*;

        let strategies: Vec<Box<dyn UnfreezeStrategy>> = vec![
            Box::new(SataLinkReset::new()),
            Box::new(PcieHotReset::new()),
            Box::new(AcpiSleep::new()),
            Box::new(UsbSuspend::new()),
            Box::new(IpmiPower::new()),
            Box::new(VendorSpecific::new()),
            Box::new(KernelModule::new()),
        ];

        for strategy in strategies {
            let name = strategy.name();
            let desc = strategy.description();

            assert!(!name.is_empty());
            assert!(!desc.is_empty());
            assert!(desc.len() > 20); // Should be descriptive
        }
    }

    #[test]
    fn test_success_probability_calculation() {
        let config = FreezeMitigationConfig::default();
        let mitigation = AdvancedFreezeMitigation::new(config);

        // Test with different freeze reasons
        let bios_prob = mitigation.calculate_success_probability(
            &Some(FreezeReason::BiosSetFrozen)
        );
        let raid_prob = mitigation.calculate_success_probability(
            &Some(FreezeReason::RaidController)
        );
        let unknown_prob = mitigation.calculate_success_probability(
            &Some(FreezeReason::Unknown)
        );

        // All should be valid probabilities
        assert!(bios_prob >= 0.0 && bios_prob <= 1.0);
        assert!(raid_prob >= 0.0 && raid_prob <= 1.0);
        assert!(unknown_prob >= 0.0 && unknown_prob <= 1.0);

        // Not frozen should have 100% success
        let not_frozen = mitigation.calculate_success_probability(&None);
        assert_eq!(not_frozen, 1.0);
    }

    #[test]
    fn test_strategy_result_helpers() {
        use crate::drives::freeze::strategies::StrategyResult;

        let success = StrategyResult::success("Test passed");
        assert!(success.success);
        assert_eq!(success.message, "Test passed");
        assert!(success.warning.is_none());

        let warning = StrategyResult::success_with_warning(
            "Completed",
            "Minor issue"
        );
        assert!(warning.success);
        assert_eq!(warning.warning.unwrap(), "Minor issue");

        let failure = StrategyResult::failure("Test failed");
        assert!(!failure.success);
        assert_eq!(failure.message, "Test failed");
    }
}

// ==================== INTEGRATION TESTS ====================
// Freeze mitigation integration test has been moved to:
// tests/hardware_integration.rs::test_freeze_mitigation_disabled
// This test uses mock drives and can run without physical hardware or root
//
// Note: Full freeze mitigation testing with frozen drives requires hardware
// The mock-based test validates that freeze mitigation can be disabled/enabled
