// Strategy that loads kernel module for direct ATA register access

use super::{StrategyResult, UnfreezeStrategy};
use crate::drives::freeze::detection::FreezeReason;
use anyhow::{anyhow, Result};
use std::path::Path;
use std::process::Command;
use std::thread;
use std::time::Duration;

pub struct KernelModule {
    module_path: String,
}

impl KernelModule {
    pub fn new() -> Self {
        // Default module path (can be configured)
        let module_path = "/usr/local/lib/sayonara-wipe/ata_unfreeze.ko".to_string();
        Self { module_path }
    }

    /// Check if module is already loaded
    fn is_module_loaded(&self) -> bool {
        let output = Command::new("lsmod").output().ok();

        if let Some(output) = output {
            let output_str = String::from_utf8_lossy(&output.stdout);
            return output_str.contains("ata_unfreeze");
        }

        false
    }

    /// Load the kernel module
    fn load_module(&self) -> Result<()> {
        if !Path::new(&self.module_path).exists() {
            return Err(anyhow!("Kernel module not found at {}", self.module_path));
        }

        println!("      📦 Loading kernel module: {}", self.module_path);

        let output = Command::new("insmod").arg(&self.module_path).output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Failed to load module: {}", stderr));
        }

        // Wait for module to complete scan
        thread::sleep(Duration::from_secs(2));

        println!("      ✅ Kernel module loaded successfully");
        Ok(())
    }

    /// Unload the kernel module
    fn unload_module(&self) -> Result<()> {
        println!("      🗑️  Unloading kernel module");

        let output = Command::new("rmmod").arg("ata_unfreeze").output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Don't fail if module wasn't loaded
            if !stderr.contains("not found") {
                eprintln!("      ⚠️  Failed to unload module: {}", stderr);
            }
        }

        Ok(())
    }

    /// Check module output in dmesg
    fn check_module_output(&self) -> Result<bool> {
        let output = Command::new("dmesg").output()?;

        let output_str = String::from_utf8_lossy(&output.stdout);

        // Look for success messages
        let lines: Vec<&str> = output_str
            .lines()
            .filter(|l| l.contains("ata_unfreeze"))
            .collect();

        if lines.is_empty() {
            return Ok(false);
        }

        // Check last few lines for success
        for line in lines.iter().rev().take(5) {
            if line.contains("Successfully unfrozen") || line.contains("Unfroze") {
                println!("      📜 Module log: {}", line);
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Build the kernel module if needed
    fn build_module(&self) -> Result<()> {
        // Try to find module source in the codebase first
        let codebase_src = std::env::current_dir().ok().and_then(|d| {
            let path = d.join("src/drives/freeze/kernel_module");
            if path.exists() {
                Some(path)
            } else {
                None
            }
        });

        let module_src_dir = if let Some(path) = codebase_src {
            path.to_string_lossy().to_string()
        } else {
            // Fall back to system location
            let fallback = "/usr/local/src/sayonara-wipe/kernel_module";
            if !Path::new(fallback).exists() {
                return Err(anyhow!(
                    "Kernel module source not found. Expected at: \
                    src/drives/freeze/kernel_module/ or {}",
                    fallback
                ));
            }
            fallback.to_string()
        };

        println!("      🔨 Building kernel module from: {}", module_src_dir);

        let output = Command::new("make")
            .current_dir(&module_src_dir)
            .args(["-j", &num_cpus::get().to_string()])
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Module build failed: {}", stderr));
        }

        // Copy module to standard location
        std::fs::create_dir_all("/usr/local/lib/sayonara-wipe")?;
        std::fs::copy(
            format!("{}/ata_unfreeze.ko", module_src_dir),
            &self.module_path,
        )?;

        println!(
            "      ✅ Module built and installed to {}",
            self.module_path
        );
        Ok(())
    }
}

impl UnfreezeStrategy for KernelModule {
    fn name(&self) -> &str {
        "Kernel Module"
    }

    fn description(&self) -> &str {
        "Loads a kernel module that directly manipulates ATA registers to unfreeze drives"
    }

    fn is_compatible_with(&self, _reason: &FreezeReason) -> bool {
        // Kernel module works for all freeze reasons as last resort
        true
    }

    fn is_available(&self) -> bool {
        // Check if we can load kernel modules (root required)
        if unsafe { libc::geteuid() } != 0 {
            return false;
        }

        // Check if module exists or can be built
        if Path::new(&self.module_path).exists() {
            return true;
        }

        // Check if we have kernel headers for building
        let kernel_version = Command::new("uname")
            .arg("-r")
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string());

        if let Some(version) = kernel_version {
            let headers_path = format!("/lib/modules/{}/build", version);
            return Path::new(&headers_path).exists();
        }

        false
    }

    fn execute(&self, _device_path: &str, _reason: &FreezeReason) -> Result<StrategyResult> {
        println!("      🔧 Executing kernel module strategy");

        // Check if module already loaded
        let was_loaded = self.is_module_loaded();
        if was_loaded {
            println!("      ℹ️  Module already loaded, unloading first");
            self.unload_module()?;
            thread::sleep(Duration::from_secs(1));
        }

        // Build module if needed
        if !Path::new(&self.module_path).exists() {
            match self.build_module() {
                Ok(_) => println!("      ✅ Module compiled successfully"),
                Err(e) => {
                    return Err(anyhow!(
                        "Failed to build kernel module: {}. \
                        Install kernel headers with: apt install linux-headers-$(uname -r)",
                        e
                    ));
                }
            }
        }

        // Load the module
        self.load_module()?;

        // Check if module successfully unfroze any drives
        let success = self.check_module_output()?;

        // Always unload module after use
        let _ = self.unload_module();

        if success {
            Ok(StrategyResult::success(
                "Kernel module successfully unfroze drive(s)",
            ))
        } else {
            Ok(StrategyResult::success_with_warning(
                "Kernel module loaded but no frozen drives were unfrozen",
                "Check dmesg for details",
            ))
        }
    }

    fn estimated_duration(&self) -> u64 {
        20 // 20 seconds (includes potential build time)
    }

    fn risk_level(&self) -> u8 {
        8 // High risk - direct hardware manipulation
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kernel_module_properties() {
        let strategy = KernelModule::new();

        assert_eq!(strategy.name(), "Kernel Module");
        assert_eq!(strategy.risk_level(), 8);
        assert!(strategy.is_compatible_with(&FreezeReason::Unknown));
        assert!(strategy.is_compatible_with(&FreezeReason::BiosSetFrozen));
    }

    #[test]
    fn test_module_loaded_check() {
        let strategy = KernelModule::new();
        // This test doesn't require root
        let _loaded = strategy.is_module_loaded();
        // Just verify it doesn't crash
    }
}
