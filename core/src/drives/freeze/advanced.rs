// Advanced drive freeze mitigation with vendor-specific support
// and multiple unfreeze strategies

use crate::{DriveError, DriveResult, FreezeStatus};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

// Use relative imports from sibling modules
use super::detection::{FreezeDetector, FreezeReason};
use super::strategies::UnfreezeStrategy;

// Import strategies directly
use super::strategies::{
    AcpiSleep, IpmiPower, KernelModule, PcieHotReset, SataLinkReset, UsbSuspend, VendorSpecific,
};

/// Configuration for freeze mitigation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FreezeMitigationConfig {
    /// Maximum time to spend trying to unfreeze (seconds)
    pub max_attempts_duration: u64,
    /// Enable kernel module loading if available
    pub allow_kernel_module: bool,
    /// Enable IPMI commands (server environments)
    pub allow_ipmi: bool,
    /// Enable ACPI sleep (may affect other system operations)
    pub allow_acpi_sleep: bool,
    /// Enable vendor-specific commands
    pub allow_vendor_specific: bool,
    /// Retry delay between attempts (milliseconds)
    pub retry_delay_ms: u64,
}

impl Default for FreezeMitigationConfig {
    fn default() -> Self {
        Self {
            max_attempts_duration: 300, // 5 minutes
            allow_kernel_module: true,
            allow_ipmi: false,       // Conservative default
            allow_acpi_sleep: false, // May affect system
            allow_vendor_specific: true,
            retry_delay_ms: 2000,
        }
    }
}

/// Result of an unfreeze attempt
#[derive(Debug, Clone)]
pub struct UnfreezeResult {
    pub success: bool,
    pub method_used: String,
    pub attempts_made: u32,
    pub time_taken: Duration,
    pub freeze_reason: Option<FreezeReason>,
    pub warnings: Vec<String>,
}

/// Advanced freeze mitigation system
pub struct AdvancedFreezeMitigation {
    config: FreezeMitigationConfig,
    pub(crate) strategies: Vec<Box<dyn UnfreezeStrategy>>,
    success_history: SuccessHistory,
}

/// Track successful methods for optimization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessHistory {
    successful_methods: std::collections::HashMap<String, u32>,
    last_updated: chrono::DateTime<chrono::Utc>,
}

impl SuccessHistory {
    pub(crate) fn new() -> Self {
        Self {
            successful_methods: std::collections::HashMap::new(),
            last_updated: chrono::Utc::now(),
        }
    }

    pub(crate) fn record_success(&mut self, method: &str) {
        *self
            .successful_methods
            .entry(method.to_string())
            .or_insert(0) += 1;
        self.last_updated = chrono::Utc::now();
        let _ = self.save();
    }

    pub(crate) fn get_priority(&self, method: &str) -> u32 {
        *self.successful_methods.get(method).unwrap_or(&0)
    }

    pub(crate) fn save(&self) -> Result<()> {
        let path = "/var/lib/sayonara-wipe/freeze_history.json";
        if let Some(parent) = Path::new(path).parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)?;
        Ok(())
    }

    fn load() -> Self {
        let path = "/var/lib/sayonara-wipe/freeze_history.json";
        match fs::read_to_string(path) {
            Ok(json) => serde_json::from_str(&json).unwrap_or_else(|_| Self::new()),
            Err(_) => Self::new(),
        }
    }
}

impl AdvancedFreezeMitigation {
    /// Create new advanced freeze mitigation system
    pub fn new(config: FreezeMitigationConfig) -> Self {
        let mut strategies: Vec<Box<dyn UnfreezeStrategy>> = Vec::new();

        // Build strategy list based on configuration
        // Order by historical success rate
        let history = SuccessHistory::load();

        // Add all available strategies
        strategies.push(Box::new(SataLinkReset::new()));
        strategies.push(Box::new(PcieHotReset::new()));

        if config.allow_acpi_sleep {
            strategies.push(Box::new(AcpiSleep::new()));
        }

        strategies.push(Box::new(UsbSuspend::new()));

        if config.allow_ipmi {
            strategies.push(Box::new(IpmiPower::new()));
        }

        if config.allow_vendor_specific {
            strategies.push(Box::new(VendorSpecific::new()));
        }

        if config.allow_kernel_module {
            strategies.push(Box::new(KernelModule::new()));
        }

        // Sort strategies by historical success rate
        strategies.sort_by_key(|s| std::cmp::Reverse(history.get_priority(s.name())));

        Self {
            config,
            strategies,
            success_history: history,
        }
    }

    /// Attempt to unfreeze a drive using all available methods
    pub fn unfreeze_drive(&mut self, device_path: &str) -> DriveResult<UnfreezeResult> {
        println!("🔓 Starting advanced freeze mitigation for {}", device_path);

        let start_time = Instant::now();
        let mut warnings = Vec::new();
        let mut attempts = 0u32;

        // Step 1: Detect current freeze status
        let initial_status = self.get_freeze_status(device_path)?;
        println!("  📊 Initial freeze status: {:?}", initial_status);

        if initial_status == FreezeStatus::NotFrozen {
            return Ok(UnfreezeResult {
                success: true,
                method_used: "None (not frozen)".to_string(),
                attempts_made: 0,
                time_taken: Duration::from_secs(0),
                freeze_reason: None,
                warnings: vec![],
            });
        }

        // Step 2: Detect freeze reason
        let freeze_reason = FreezeDetector::detect_reason(device_path)?;
        println!("  🔍 Detected freeze reason: {:?}", freeze_reason);

        // Step 3: Try strategies in order
        for strategy in &self.strategies {
            if start_time.elapsed() > Duration::from_secs(self.config.max_attempts_duration) {
                warnings.push(format!(
                    "Timeout reached after {} seconds",
                    self.config.max_attempts_duration
                ));
                break;
            }

            // Check if strategy is compatible with freeze reason
            if !strategy.is_compatible_with(&freeze_reason) {
                println!(
                    "  ⏭️  Skipping {} (incompatible with freeze reason)",
                    strategy.name()
                );
                continue;
            }

            println!("  🔧 Attempting method: {}", strategy.name());
            println!("     Description: {}", strategy.description());

            attempts += 1;

            match strategy.execute(device_path, &freeze_reason) {
                Ok(result) => {
                    if let Some(warn) = result.warning {
                        warnings.push(warn);
                    }

                    // Wait for device to stabilize
                    thread::sleep(Duration::from_millis(self.config.retry_delay_ms));

                    // Verify unfreeze was successful
                    let new_status = self.get_freeze_status(device_path)?;

                    if new_status == FreezeStatus::NotFrozen {
                        println!("  ✅ Successfully unfrozen using {}", strategy.name());

                        // Record success for future optimization
                        self.success_history.record_success(strategy.name());

                        return Ok(UnfreezeResult {
                            success: true,
                            method_used: strategy.name().to_string(),
                            attempts_made: attempts,
                            time_taken: start_time.elapsed(),
                            freeze_reason: Some(freeze_reason),
                            warnings,
                        });
                    } else {
                        println!("  ❌ Method failed to unfreeze drive");
                    }
                }
                Err(e) => {
                    println!("  ⚠️  Method failed: {}", e);
                    warnings.push(format!("{} failed: {}", strategy.name(), e));
                }
            }
        }

        // All methods failed
        Err(DriveError::DriveFrozen(format!(
            "Failed to unfreeze drive after {} attempts using {} methods. \
                    Freeze reason: {:?}. Consider: 1) Cold boot the system, \
                    2) Try different controller mode in BIOS, 3) Use IPMI power cycle",
            attempts,
            self.strategies.len(),
            freeze_reason
        )))
    }

    /// Get current freeze status
    fn get_freeze_status(&self, device_path: &str) -> DriveResult<FreezeStatus> {
        let output = Command::new("hdparm")
            .args(["-I", device_path])
            .output()
            .map_err(|e| DriveError::HardwareCommandFailed(format!("hdparm failed: {}", e)))?;

        let output_str = String::from_utf8_lossy(&output.stdout);

        if output_str.contains("frozen") && output_str.contains("not") {
            Ok(FreezeStatus::NotFrozen)
        } else if output_str.contains("frozen") {
            if output_str.contains("BIOS") {
                Ok(FreezeStatus::FrozenByBIOS)
            } else {
                Ok(FreezeStatus::Frozen)
            }
        } else if output_str.contains("locked") {
            Ok(FreezeStatus::SecurityLocked)
        } else {
            Ok(FreezeStatus::Unknown)
        }
    }

    /// Check if secure erase is blocked
    pub fn is_secure_erase_blocked(&self, device_path: &str) -> DriveResult<bool> {
        let status = self.get_freeze_status(device_path)?;
        Ok(matches!(
            status,
            FreezeStatus::Frozen | FreezeStatus::FrozenByBIOS
        ))
    }

    /// Get detailed freeze information
    pub fn get_freeze_info(&self, device_path: &str) -> DriveResult<FreezeInfo> {
        let status = self.get_freeze_status(device_path)?;
        let reason = if status != FreezeStatus::NotFrozen {
            Some(FreezeDetector::detect_reason(device_path)?)
        } else {
            None
        };

        let compatible_strategies: Vec<String> = if let Some(ref r) = reason {
            self.strategies
                .iter()
                .filter(|s| s.is_compatible_with(r))
                .map(|s| s.name().to_string())
                .collect()
        } else {
            Vec::new()
        };

        Ok(FreezeInfo {
            status,
            reason: reason.clone(),
            compatible_strategies,
            estimated_success_rate: self.calculate_success_probability(&reason),
        })
    }

    /// Calculate probability of successful unfreeze
    pub(crate) fn calculate_success_probability(&self, reason: &Option<FreezeReason>) -> f64 {
        let Some(ref r) = reason else {
            return 1.0; // Not frozen
        };

        let compatible_count = self
            .strategies
            .iter()
            .filter(|s| s.is_compatible_with(r))
            .count();

        if compatible_count == 0 {
            return 0.1; // Low but not zero (kernel module might work)
        }

        // Calculate based on historical success rate
        let total_successes: u32 = self
            .strategies
            .iter()
            .filter(|s| s.is_compatible_with(r))
            .map(|s| self.success_history.get_priority(s.name()))
            .sum();

        if total_successes == 0 {
            // No history, use baseline estimates
            match r {
                FreezeReason::BiosSetFrozen => 0.85,
                FreezeReason::ControllerPolicy => 0.90,
                FreezeReason::OsSecurity => 0.95,
                FreezeReason::RaidController => 0.70,
                FreezeReason::Unknown => 0.60,
            }
        } else {
            // Use historical success rate
            0.5 + (total_successes as f64 / 100.0).min(0.45)
        }
    }
}

/// Detailed freeze information
#[derive(Debug, Clone)]
pub struct FreezeInfo {
    pub status: FreezeStatus,
    pub reason: Option<FreezeReason>,
    pub compatible_strategies: Vec<String>,
    pub estimated_success_rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_success_history() {
        let mut history = SuccessHistory::new();

        history.record_success("SATA Link Reset");
        history.record_success("SATA Link Reset");
        history.record_success("PCIe Hot Reset");

        assert_eq!(history.get_priority("SATA Link Reset"), 2);
        assert_eq!(history.get_priority("PCIe Hot Reset"), 1);
        assert_eq!(history.get_priority("Unknown Method"), 0);
    }

    #[test]
    fn test_config_defaults() {
        let config = FreezeMitigationConfig::default();

        assert_eq!(config.max_attempts_duration, 300);
        assert!(config.allow_kernel_module);
        assert!(!config.allow_ipmi); // Conservative default
        assert!(config.allow_vendor_specific);
    }

    #[test]
    fn test_strategy_prioritization() {
        let mut history = SuccessHistory::new();
        history.record_success("Method B");
        history.record_success("Method B");
        history.record_success("Method A");

        // Method B should have higher priority
        assert!(history.get_priority("Method B") > history.get_priority("Method A"));
    }
}
