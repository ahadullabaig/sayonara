/// Degraded mode operations - fallback when full operation isn't possible
///
/// This module defines degraded operation modes that allow the wipe to continue
/// with reduced functionality when errors prevent full operation.
use crate::WipeConfig;
use serde::{Deserialize, Serialize};

/// Degraded operation mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DegradedMode {
    /// Skip verification after write (continue wiping only)
    SkipVerification,

    /// Reduce number of passes (e.g., 35 → 7 → 3)
    ReducedPasses,

    /// Use slower, safer I/O (disable O_DIRECT, reduce queue depth)
    SlowerIO,

    /// Skip hidden areas (HPA/DCO) if problematic
    SkipHiddenAreas,

    /// Skip TRIM/discard operations
    SkipTRIM,

    /// Continue despite bad sectors (within limits)
    TolerateBadSectors,
}

impl DegradedMode {
    /// Get human-readable description
    pub fn description(&self) -> &'static str {
        match self {
            DegradedMode::SkipVerification => "Skip post-write verification",
            DegradedMode::ReducedPasses => "Reduce number of overwrite passes",
            DegradedMode::SlowerIO => "Use slower, safer I/O operations",
            DegradedMode::SkipHiddenAreas => "Skip HPA/DCO areas",
            DegradedMode::SkipTRIM => "Skip TRIM/discard operations",
            DegradedMode::TolerateBadSectors => "Continue despite bad sectors",
        }
    }

    /// Check if this mode is safe for compliance requirements
    pub fn is_compliance_safe(&self) -> bool {
        match self {
            // These modes maintain data destruction integrity
            DegradedMode::SlowerIO => true,
            DegradedMode::SkipTRIM => true,
            DegradedMode::TolerateBadSectors => true,

            // These modes may compromise compliance
            DegradedMode::SkipVerification => false,
            DegradedMode::ReducedPasses => false,
            DegradedMode::SkipHiddenAreas => false,
        }
    }

    /// Get severity level (0-10, where 10 is most severe degradation)
    pub fn severity(&self) -> u8 {
        match self {
            DegradedMode::SkipTRIM => 2,
            DegradedMode::SlowerIO => 3,
            DegradedMode::TolerateBadSectors => 5,
            DegradedMode::SkipVerification => 7,
            DegradedMode::ReducedPasses => 8,
            DegradedMode::SkipHiddenAreas => 9,
        }
    }

    /// Check if user confirmation is required
    pub fn requires_confirmation(&self) -> bool {
        !self.is_compliance_safe()
    }
}

/// Degraded mode manager
pub struct DegradedModeManager {
    /// Active degraded modes
    active_modes: Vec<DegradedMode>,

    /// Whether user confirmation was obtained
    user_confirmed: bool,
}

impl DegradedModeManager {
    /// Create new degraded mode manager
    pub fn new() -> Self {
        Self {
            active_modes: Vec::new(),
            user_confirmed: false,
        }
    }

    /// Enable a degraded mode
    pub fn enable(&mut self, mode: DegradedMode) {
        if !self.active_modes.contains(&mode) {
            tracing::warn!(
                mode = ?mode,
                description = mode.description(),
                severity = mode.severity(),
                compliance_safe = mode.is_compliance_safe(),
                "Enabling degraded mode"
            );
            self.active_modes.push(mode);
        }
    }

    /// Check if mode is active
    pub fn is_active(&self, mode: DegradedMode) -> bool {
        self.active_modes.contains(&mode)
    }

    /// Get all active modes
    pub fn active_modes(&self) -> &[DegradedMode] {
        &self.active_modes
    }

    /// Check if any non-compliance-safe modes are active
    pub fn has_compliance_risk(&self) -> bool {
        self.active_modes.iter().any(|m| !m.is_compliance_safe())
    }

    /// Get maximum severity of active modes
    pub fn max_severity(&self) -> u8 {
        self.active_modes
            .iter()
            .map(|m| m.severity())
            .max()
            .unwrap_or(0)
    }

    /// Set user confirmation status
    pub fn set_user_confirmed(&mut self, confirmed: bool) {
        self.user_confirmed = confirmed;
    }

    /// Check if user confirmation was obtained
    pub fn is_user_confirmed(&self) -> bool {
        self.user_confirmed
    }

    /// Adjust wipe configuration for degraded mode
    pub fn adjust_config(&self, config: &mut WipeConfig) {
        for mode in &self.active_modes {
            match mode {
                DegradedMode::SkipVerification => {
                    config.verify = false;
                }
                DegradedMode::ReducedPasses => {
                    // Reduce passes intelligently based on algorithm
                    if let Some(passes) = config.multiple_passes {
                        config.multiple_passes = Some(passes.min(3));
                    }
                }
                DegradedMode::SlowerIO => {
                    // This would be handled at I/O engine level
                    // Set flag in config for I/O engine to read
                    config.temperature_monitoring = true;
                }
                DegradedMode::SkipHiddenAreas => {
                    config.handle_hpa_dco = crate::HPADCOHandling::Ignore;
                }
                DegradedMode::SkipTRIM => {
                    config.use_trim_after = false;
                }
                DegradedMode::TolerateBadSectors => {
                    // This is handled by BadSectorHandler, not config
                    // Just logging here
                    tracing::info!("Bad sectors will be tolerated and logged");
                }
            }
        }

        if !self.active_modes.is_empty() {
            tracing::warn!(
                active_count = self.active_modes.len(),
                max_severity = self.max_severity(),
                "Wipe configuration adjusted for degraded mode"
            );
        }
    }

    /// Generate degraded mode summary
    pub fn summary(&self) -> String {
        if self.active_modes.is_empty() {
            return "No degraded modes active".to_string();
        }

        let mut summary = format!("Active degraded modes ({}):\n", self.active_modes.len());

        for mode in &self.active_modes {
            summary.push_str(&format!(
                "  - {} (severity: {}, compliance safe: {})\n",
                mode.description(),
                mode.severity(),
                mode.is_compliance_safe()
            ));
        }

        if self.has_compliance_risk() {
            summary.push_str("\n⚠️  WARNING: Compliance requirements may not be met!\n");
        }

        summary
    }
}

impl Default for DegradedModeManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_degraded_mode_properties() {
        assert_eq!(
            DegradedMode::SkipVerification.description(),
            "Skip post-write verification"
        );
        assert!(!DegradedMode::SkipVerification.is_compliance_safe());
        assert!(DegradedMode::SlowerIO.is_compliance_safe());
    }

    #[test]
    fn test_degraded_mode_severity() {
        assert!(DegradedMode::SkipTRIM.severity() < DegradedMode::SkipVerification.severity());
        assert!(DegradedMode::SkipVerification.severity() < DegradedMode::ReducedPasses.severity());
    }

    #[test]
    fn test_degraded_mode_manager_enable() {
        let mut manager = DegradedModeManager::new();
        assert_eq!(manager.active_modes().len(), 0);

        manager.enable(DegradedMode::SkipVerification);
        assert_eq!(manager.active_modes().len(), 1);
        assert!(manager.is_active(DegradedMode::SkipVerification));
        assert!(!manager.is_active(DegradedMode::ReducedPasses));
    }

    #[test]
    fn test_degraded_mode_manager_duplicate() {
        let mut manager = DegradedModeManager::new();

        manager.enable(DegradedMode::SkipVerification);
        manager.enable(DegradedMode::SkipVerification);

        // Should only be added once
        assert_eq!(manager.active_modes().len(), 1);
    }

    #[test]
    fn test_has_compliance_risk() {
        let mut manager = DegradedModeManager::new();
        assert!(!manager.has_compliance_risk());

        manager.enable(DegradedMode::SlowerIO);
        assert!(!manager.has_compliance_risk());

        manager.enable(DegradedMode::SkipVerification);
        assert!(manager.has_compliance_risk());
    }

    #[test]
    fn test_max_severity() {
        let mut manager = DegradedModeManager::new();
        assert_eq!(manager.max_severity(), 0);

        manager.enable(DegradedMode::SkipTRIM); // severity 2
        assert_eq!(manager.max_severity(), 2);

        manager.enable(DegradedMode::ReducedPasses); // severity 8
        assert_eq!(manager.max_severity(), 8);
    }

    #[test]
    fn test_adjust_config_skip_verification() {
        let mut manager = DegradedModeManager::new();
        let mut config = WipeConfig::default();

        config.verify = true;
        manager.enable(DegradedMode::SkipVerification);
        manager.adjust_config(&mut config);

        assert!(!config.verify);
    }

    #[test]
    fn test_adjust_config_reduced_passes() {
        let mut manager = DegradedModeManager::new();
        let mut config = WipeConfig::default();

        config.multiple_passes = Some(35);
        manager.enable(DegradedMode::ReducedPasses);
        manager.adjust_config(&mut config);

        assert!(config.multiple_passes.unwrap() <= 3);
    }

    #[test]
    fn test_adjust_config_skip_hidden_areas() {
        let mut manager = DegradedModeManager::new();
        let mut config = WipeConfig::default();

        config.handle_hpa_dco = crate::HPADCOHandling::TemporaryRemove;
        manager.enable(DegradedMode::SkipHiddenAreas);
        manager.adjust_config(&mut config);

        assert_eq!(config.handle_hpa_dco, crate::HPADCOHandling::Ignore);
    }

    #[test]
    fn test_adjust_config_skip_trim() {
        let mut manager = DegradedModeManager::new();
        let mut config = WipeConfig::default();

        config.use_trim_after = true;
        manager.enable(DegradedMode::SkipTRIM);
        manager.adjust_config(&mut config);

        assert!(!config.use_trim_after);
    }

    #[test]
    fn test_user_confirmation() {
        let mut manager = DegradedModeManager::new();
        assert!(!manager.is_user_confirmed());

        manager.set_user_confirmed(true);
        assert!(manager.is_user_confirmed());
    }

    #[test]
    fn test_requires_confirmation() {
        assert!(!DegradedMode::SlowerIO.requires_confirmation());
        assert!(DegradedMode::SkipVerification.requires_confirmation());
        assert!(DegradedMode::ReducedPasses.requires_confirmation());
    }

    #[test]
    fn test_summary() {
        let mut manager = DegradedModeManager::new();

        let summary = manager.summary();
        assert!(summary.contains("No degraded modes"));

        manager.enable(DegradedMode::SkipVerification);
        manager.enable(DegradedMode::ReducedPasses);

        let summary = manager.summary();
        assert!(summary.contains("Active degraded modes (2)"));
        assert!(summary.contains("Skip post-write verification"));
        assert!(summary.contains("WARNING"));
    }
}
