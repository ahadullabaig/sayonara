// Consolidated freeze mitigation module

// Submodules
pub mod advanced; // Advanced freeze mitigation with strategies
pub mod basic; // Basic freeze mitigation (original implementation)
pub mod detection; // Freeze reason detection
pub mod strategies; // Unfreeze strategies

#[cfg(test)]
mod tests;

// Re-exports for convenience
pub use basic::FreezeMitigation;

pub use advanced::{AdvancedFreezeMitigation, FreezeInfo, FreezeMitigationConfig, UnfreezeResult};

pub use detection::{FreezeDetector, FreezeReason};

pub use strategies::{
    AcpiSleep,
    IpmiPower,
    KernelModule,
    PcieHotReset,
    // Individual strategies
    SataLinkReset,
    StrategyResult,
    UnfreezeStrategy,
    UsbSuspend,
    VendorSpecific,
};

/// Trait for freeze mitigation strategies (basic and advanced)
pub trait FreezeMitigationStrategy {
    fn unfreeze(&mut self, device_path: &str) -> crate::DriveResult<()>;
    fn is_frozen(&self, device_path: &str) -> crate::DriveResult<bool>;
}

// Implement for basic freeze mitigation
impl FreezeMitigationStrategy for basic::FreezeMitigation {
    fn unfreeze(&mut self, device_path: &str) -> crate::DriveResult<()> {
        basic::FreezeMitigation::unfreeze_drive(device_path)
    }

    fn is_frozen(&self, device_path: &str) -> crate::DriveResult<bool> {
        let status = basic::FreezeMitigation::get_freeze_status(device_path)?;
        Ok(matches!(
            status,
            crate::FreezeStatus::Frozen | crate::FreezeStatus::FrozenByBIOS
        ))
    }
}

// Implement for advanced freeze mitigation
impl FreezeMitigationStrategy for advanced::AdvancedFreezeMitigation {
    fn unfreeze(&mut self, device_path: &str) -> crate::DriveResult<()> {
        match self.unfreeze_drive(device_path) {
            Ok(result) => {
                if result.success {
                    Ok(())
                } else {
                    Err(crate::DriveError::DriveFrozen(format!(
                        "Failed to unfreeze: {}",
                        result.method_used
                    )))
                }
            }
            Err(e) => Err(crate::DriveError::DriveFrozen(e.to_string())),
        }
    }

    fn is_frozen(&self, device_path: &str) -> crate::DriveResult<bool> {
        Ok(self.is_secure_erase_blocked(device_path)?)
    }
}

/// Helper function to choose between basic and advanced freeze mitigation
pub fn get_mitigation(use_advanced: bool) -> Box<dyn FreezeMitigationStrategy> {
    if use_advanced {
        Box::new(advanced::AdvancedFreezeMitigation::new(
            FreezeMitigationConfig::default(),
        ))
    } else {
        Box::new(basic::FreezeMitigation)
    }
}
