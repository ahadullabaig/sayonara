// Strategy trait and implementations for drive unfreeze methods

use super::detection::FreezeReason;
use anyhow::Result; // UPDATED: relative import from parent

mod kernel_module;
mod remaining_impl;
mod sata_link_reset;
mod vendor_specific;

pub use kernel_module::KernelModule;
pub use remaining_impl::{AcpiSleep, IpmiPower, PcieHotReset, UsbSuspend};
pub use sata_link_reset::SataLinkReset;
pub use vendor_specific::VendorSpecific;

/// Result of a strategy execution
#[derive(Debug, Clone)]
pub struct StrategyResult {
    pub success: bool,
    pub message: String,
    pub warning: Option<String>,
}

impl StrategyResult {
    pub fn success(message: impl Into<String>) -> Self {
        Self {
            success: true,
            message: message.into(),
            warning: None,
        }
    }

    pub fn success_with_warning(message: impl Into<String>, warning: impl Into<String>) -> Self {
        Self {
            success: true,
            message: message.into(),
            warning: Some(warning.into()),
        }
    }

    pub fn failure(message: impl Into<String>) -> Self {
        Self {
            success: false,
            message: message.into(),
            warning: None,
        }
    }
}

/// Trait for unfreeze strategies
pub trait UnfreezeStrategy: Send + Sync {
    /// Strategy name
    fn name(&self) -> &str;

    /// Human-readable description
    fn description(&self) -> &str;

    /// Check if strategy is compatible with the freeze reason
    fn is_compatible_with(&self, reason: &FreezeReason) -> bool;

    /// Check if strategy is available on this system
    fn is_available(&self) -> bool;

    /// Execute the unfreeze strategy
    fn execute(&self, device_path: &str, reason: &FreezeReason) -> Result<StrategyResult>;

    /// Estimated time to execute (seconds)
    fn estimated_duration(&self) -> u64 {
        10 // Default 10 seconds
    }

    /// Risk level (0-10, where 0 is safest)
    fn risk_level(&self) -> u8 {
        5 // Default medium risk
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strategy_result() {
        let success = StrategyResult::success("Test passed");
        assert!(success.success);
        assert!(success.warning.is_none());

        let warning = StrategyResult::success_with_warning("Passed", "Minor issue");
        assert!(warning.success);
        assert!(warning.warning.is_some());

        let failure = StrategyResult::failure("Test failed");
        assert!(!failure.success);
    }
}
