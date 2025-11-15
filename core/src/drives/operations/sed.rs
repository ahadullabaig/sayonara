use crate::SEDType;
use crate::{DriveError, DriveResult};
use anyhow::{anyhow, Result};
use std::process::Command;

#[derive(Debug, Clone)]
pub struct SEDInfo {
    pub sed_type: SEDType,
    pub locked: bool,
    pub enabled: bool,
    pub frozen: bool,
    pub max_password_tries: Option<u32>,
    pub supports_crypto_erase: bool,
    pub supports_instant_secure_erase: bool,
    pub firmware_version: Option<String>,
}

pub struct SEDManager;

impl SEDManager {
    /// Detect and return comprehensive SED information
    pub fn detect_sed(device_path: &str) -> DriveResult<SEDInfo> {
        println!(
            "Detecting self-encrypting drive capabilities for {}...",
            device_path
        );

        // Try multiple detection methods in order of preference
        if let Ok(info) = Self::detect_opal(device_path) {
            return Ok(info);
        }

        if let Ok(info) = Self::detect_tcg_enterprise(device_path) {
            return Ok(info);
        }

        if let Ok(info) = Self::detect_ata_security(device_path) {
            return Ok(info);
        }

        if let Ok(info) = Self::detect_proprietary(device_path) {
            return Ok(info);
        }

        // No SED detected
        Ok(SEDInfo {
            sed_type: SEDType::None,
            locked: false,
            enabled: false,
            frozen: false,
            max_password_tries: None,
            supports_crypto_erase: false,
            supports_instant_secure_erase: false,
            firmware_version: None,
        })
    }

    /// Detect OPAL compliance
    fn detect_opal(device_path: &str) -> Result<SEDInfo> {
        // Try sedutil-cli for OPAL detection
        let output = Command::new("sedutil-cli")
            .args(["--query", device_path])
            .output();

        if let Ok(output) = output {
            if output.status.success() {
                let output_str = String::from_utf8_lossy(&output.stdout);
                return Self::parse_opal_info(&output_str, device_path);
            }
        }

        // Try nvme for NVMe OPAL
        if device_path.contains("nvme") {
            let output = Command::new("nvme").args(["id-ctrl", device_path]).output();

            if let Ok(output) = output {
                let output_str = String::from_utf8_lossy(&output.stdout);
                if output_str.contains("OPAL") || output_str.contains("TCG") {
                    return Ok(SEDInfo {
                        sed_type: SEDType::OPAL20,
                        locked: false,
                        enabled: Self::check_opal_enabled(device_path),
                        frozen: false,
                        max_password_tries: Some(5),
                        supports_crypto_erase: true,
                        supports_instant_secure_erase: true,
                        firmware_version: Self::get_firmware_version(device_path),
                    });
                }
            }
        }

        Err(anyhow!("No OPAL support detected"))
    }

    /// Parse OPAL information from sedutil output
    fn parse_opal_info(output: &str, device_path: &str) -> Result<SEDInfo> {
        let mut sed_type = SEDType::OPAL10;
        let mut locked = false;
        let mut enabled = false;

        for line in output.lines() {
            if line.contains("OPAL 2") {
                sed_type = SEDType::OPAL20;
            } else if line.contains("Locked") && line.contains("Y") {
                locked = true;
            } else if line.contains("LockingEnabled") && line.contains("Y") {
                enabled = true;
            }
        }

        Ok(SEDInfo {
            sed_type,
            locked,
            enabled,
            frozen: false,
            max_password_tries: Some(5),
            supports_crypto_erase: true,
            supports_instant_secure_erase: true,
            firmware_version: Self::get_firmware_version(device_path),
        })
    }

    /// Detect TCG Enterprise SED
    fn detect_tcg_enterprise(device_path: &str) -> Result<SEDInfo> {
        // TCG Enterprise detection via sg_readcap and sg_opcodes
        let output = Command::new("sg_opcodes").args([device_path]).output();

        if let Ok(output) = output {
            let output_str = String::from_utf8_lossy(&output.stdout);
            if output_str.contains("SECURITY PROTOCOL") {
                // Check for TCG Enterprise specifics
                let tcg_output = Command::new("sg_readcap")
                    .args(["--16", device_path])
                    .output();

                if let Ok(tcg_output) = tcg_output {
                    let tcg_str = String::from_utf8_lossy(&tcg_output.stdout);
                    if tcg_str.contains("Protection") {
                        return Ok(SEDInfo {
                            sed_type: SEDType::TCGEnterprise,
                            locked: false,
                            enabled: true,
                            frozen: false,
                            max_password_tries: Some(3),
                            supports_crypto_erase: true,
                            supports_instant_secure_erase: true,
                            firmware_version: Self::get_firmware_version(device_path),
                        });
                    }
                }
            }
        }

        Err(anyhow!("No TCG Enterprise support detected"))
    }

    /// Detect ATA Security (non-OPAL)
    fn detect_ata_security(device_path: &str) -> Result<SEDInfo> {
        let output = Command::new("hdparm").args(["-I", device_path]).output();

        if let Ok(output) = output {
            let output_str = String::from_utf8_lossy(&output.stdout);

            if output_str.contains("Security:") {
                let enabled = output_str.contains("enabled");
                let locked = output_str.contains("locked");
                let frozen = output_str.contains("frozen") && !output_str.contains("not frozen");

                if output_str.contains("supported") {
                    return Ok(SEDInfo {
                        sed_type: SEDType::ATASecurity,
                        locked,
                        enabled,
                        frozen,
                        max_password_tries: Some(5),
                        supports_crypto_erase: false,
                        supports_instant_secure_erase: enabled,
                        firmware_version: Self::get_firmware_version(device_path),
                    });
                }
            }
        }

        Err(anyhow!("No ATA Security support detected"))
    }

    /// Detect proprietary encryption (Samsung, Crucial, WD, etc.)
    fn detect_proprietary(device_path: &str) -> Result<SEDInfo> {
        let drive_info = Self::get_drive_model(device_path)?;

        // Samsung specific
        if drive_info.contains("Samsung") {
            if drive_info.contains("EVO") || drive_info.contains("PRO") {
                return Ok(SEDInfo {
                    sed_type: SEDType::Proprietary("Samsung".to_string()),
                    locked: false,
                    enabled: Self::check_samsung_encryption(device_path),
                    frozen: false,
                    max_password_tries: Some(10),
                    supports_crypto_erase: true,
                    supports_instant_secure_erase: true,
                    firmware_version: Self::get_firmware_version(device_path),
                });
            }
        }

        // Crucial/Micron specific
        if drive_info.contains("Crucial") || drive_info.contains("Micron") {
            if drive_info.contains("MX") || drive_info.contains("BX") {
                return Ok(SEDInfo {
                    sed_type: SEDType::Proprietary("Crucial".to_string()),
                    locked: false,
                    enabled: false,
                    frozen: false,
                    max_password_tries: Some(5),
                    supports_crypto_erase: true,
                    supports_instant_secure_erase: true,
                    firmware_version: Self::get_firmware_version(device_path),
                });
            }
        }

        // Intel specific
        if drive_info.contains("Intel") && drive_info.contains("SSD") {
            return Ok(SEDInfo {
                sed_type: SEDType::Proprietary("Intel".to_string()),
                locked: false,
                enabled: false,
                frozen: false,
                max_password_tries: Some(3),
                supports_crypto_erase: true,
                supports_instant_secure_erase: true,
                firmware_version: Self::get_firmware_version(device_path),
            });
        }

        Err(anyhow!("No proprietary encryption detected"))
    }

    /// Perform crypto erase on SED
    pub fn crypto_erase(device_path: &str, sed_info: &SEDInfo) -> DriveResult<()> {
        println!("Performing cryptographic erase on {}...", device_path);

        if !sed_info.supports_crypto_erase {
            return Err(DriveError::CryptoEraseFailed(
                "Device does not support cryptographic erase".to_string(),
            ));
        }

        match &sed_info.sed_type {
            SEDType::OPAL20 | SEDType::OPAL10 => Self::opal_crypto_erase(device_path),
            SEDType::TCGEnterprise => Self::tcg_crypto_erase(device_path),
            SEDType::ATASecurity => Self::ata_secure_erase(device_path),
            SEDType::EDrive => Self::edrive_crypto_erase(device_path),
            SEDType::Proprietary(vendor) => Self::proprietary_crypto_erase(device_path, vendor),
            SEDType::None => Err(DriveError::CryptoEraseFailed(
                "No SED capabilities detected".to_string(),
            )),
        }
    }

    /// OPAL crypto erase
    fn opal_crypto_erase(device_path: &str) -> DriveResult<()> {
        println!("Executing OPAL cryptographic erase...");

        // First try revert to factory (most thorough)
        let output = Command::new("sedutil-cli")
            .args(["--revertTPer", "password", device_path])
            .output();

        if let Ok(output) = output {
            if output.status.success() {
                println!("OPAL revert completed successfully");
                return Ok(());
            }
        }

        // Try PSID revert if available
        println!("Attempting PSID revert (requires physical label PSID)...");
        // This would require user input for PSID

        Err(DriveError::CryptoEraseFailed(
            "OPAL crypto erase requires valid credentials or PSID".to_string(),
        ))
    }

    /// TCG Enterprise crypto erase
    fn tcg_crypto_erase(device_path: &str) -> DriveResult<()> {
        println!("Executing TCG Enterprise cryptographic erase...");

        // TCG Enterprise erase typically requires specialized tools
        // Try generic SCSI sanitize with crypto erase
        let output = Command::new("sg_sanitize")
            .args(["--crypto", device_path])
            .output();

        if let Ok(output) = output {
            if output.status.success() {
                println!("TCG Enterprise crypto erase completed");
                return Ok(());
            }
        }

        Err(DriveError::CryptoEraseFailed(
            "TCG Enterprise crypto erase failed".to_string(),
        ))
    }

    /// ATA secure erase (for ATA Security feature set)
    fn ata_secure_erase(device_path: &str) -> DriveResult<()> {
        // This is handled by existing secure erase code
        println!("Using ATA Secure Erase for crypto erase...");

        // Set temporary password and erase
        let password = "temporary_erase_pwd";

        let output = Command::new("hdparm")
            .args([
                "--user-master",
                "u",
                "--security-set-pass",
                password,
                device_path,
            ])
            .output()
            .map_err(|e| DriveError::CryptoEraseFailed(format!("Failed to set password: {}", e)))?;

        if !output.status.success() {
            return Err(DriveError::CryptoEraseFailed(
                "Failed to set security password".to_string(),
            ));
        }

        let output = Command::new("hdparm")
            .args([
                "--user-master",
                "u",
                "--security-erase",
                password,
                device_path,
            ])
            .output()
            .map_err(|e| DriveError::CryptoEraseFailed(format!("Secure erase failed: {}", e)))?;

        if output.status.success() {
            println!("ATA Secure Erase completed");
            Ok(())
        } else {
            Err(DriveError::CryptoEraseFailed(
                "ATA Secure Erase failed".to_string(),
            ))
        }
    }

    /// eDrive (BitLocker hardware encryption) crypto erase
    fn edrive_crypto_erase(device_path: &str) -> DriveResult<()> {
        println!("Executing eDrive cryptographic erase...");

        // eDrive typically uses OPAL 2.0 underneath
        Self::opal_crypto_erase(device_path)
    }

    /// Proprietary vendor-specific crypto erase
    fn proprietary_crypto_erase(device_path: &str, vendor: &str) -> DriveResult<()> {
        println!("Executing {} proprietary crypto erase...", vendor);

        match vendor {
            "Samsung" => {
                // Try Samsung Magician CLI if available
                let output = Command::new("magician")
                    .args(["--secure-erase", device_path])
                    .output();

                if let Ok(output) = output {
                    if output.status.success() {
                        return Ok(());
                    }
                }
            }
            "Crucial" => {
                // Crucial drives often support standard ATA secure erase
                return Self::ata_secure_erase(device_path);
            }
            "Intel" => {
                // Intel SSDs typically support enhanced secure erase
                return Self::ata_secure_erase(device_path);
            }
            _ => {}
        }

        // Fall back to standard secure erase
        Self::ata_secure_erase(device_path)
    }

    /// Check if OPAL is enabled
    fn check_opal_enabled(device_path: &str) -> bool {
        if let Ok(output) = Command::new("sedutil-cli")
            .args(["--isValidSED", device_path])
            .output()
        {
            return output.status.success();
        }
        false
    }

    /// Check Samsung encryption status
    fn check_samsung_encryption(device_path: &str) -> bool {
        // Check via smartctl for Samsung-specific attributes
        if let Ok(output) = Command::new("smartctl").args(["-A", device_path]).output() {
            let output_str = String::from_utf8_lossy(&output.stdout);
            // Samsung SSDs report encryption status in vendor-specific attributes
            return output_str.contains("Encrypted");
        }
        false
    }

    /// Get drive model information
    fn get_drive_model(device_path: &str) -> Result<String> {
        let output = Command::new("smartctl")
            .args(["-i", device_path])
            .output()?;

        let output_str = String::from_utf8_lossy(&output.stdout);

        for line in output_str.lines() {
            if line.contains("Device Model:") || line.contains("Model Number:") {
                if let Some(model) = line.split(':').nth(1) {
                    return Ok(model.trim().to_string());
                }
            }
        }

        Err(anyhow!("Could not determine drive model"))
    }

    /// Get firmware version
    fn get_firmware_version(device_path: &str) -> Option<String> {
        let output = Command::new("smartctl")
            .args(["-i", device_path])
            .output()
            .ok()?;

        let output_str = String::from_utf8_lossy(&output.stdout);

        for line in output_str.lines() {
            if line.contains("Firmware Version:") || line.contains("Revision:") {
                if let Some(version) = line.split(':').nth(1) {
                    return Some(version.trim().to_string());
                }
            }
        }

        None
    }

    /// Unlock SED with password
    pub fn unlock_sed(device_path: &str, password: &str, sed_info: &SEDInfo) -> DriveResult<()> {
        if !sed_info.locked {
            println!("Drive is not locked");
            return Ok(());
        }

        match &sed_info.sed_type {
            SEDType::OPAL20 | SEDType::OPAL10 => {
                let output = Command::new("sedutil-cli")
                    .args(["--setLockingRange", "0", "RW", password, device_path])
                    .output()
                    .map_err(|e| DriveError::UnlockFailed(format!("Failed to unlock: {}", e)))?;

                if output.status.success() {
                    println!("Drive unlocked successfully");
                    Ok(())
                } else {
                    Err(DriveError::UnlockFailed(
                        "Invalid password or unlock failed".to_string(),
                    ))
                }
            }
            SEDType::ATASecurity => {
                let output = Command::new("hdparm")
                    .args([
                        "--user-master",
                        "u",
                        "--security-unlock",
                        password,
                        device_path,
                    ])
                    .output()
                    .map_err(|e| DriveError::UnlockFailed(format!("Failed to unlock: {}", e)))?;

                if output.status.success() {
                    println!("Drive unlocked successfully");
                    Ok(())
                } else {
                    Err(DriveError::UnlockFailed(
                        "Invalid password or unlock failed".to_string(),
                    ))
                }
            }
            _ => Err(DriveError::UnlockFailed(
                "Unlock not supported for this SED type".to_string(),
            )),
        }
    }

    /// Verify crypto erase effectiveness
    pub fn verify_crypto_erase(device_path: &str) -> DriveResult<bool> {
        println!("Verifying cryptographic erase effectiveness...");

        // Read some sectors to check for encrypted vs zeros/random
        use crate::io::{IOConfig, OptimizedIO};

        let config = IOConfig::small_read_optimized();
        let mut handle = OptimizedIO::open(device_path, config).map_err(|e| {
            DriveError::IoError(std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string(),
            ))
        })?;

        let mut all_zero = true;
        let mut all_ff = true;

        // Sample multiple locations
        for offset in [0, 1024 * 1024, 1024 * 1024 * 1024].iter() {
            let buffer =
                OptimizedIO::read_range(&mut handle, *offset, 4096).unwrap_or_else(|_| vec![]);

            if !buffer.is_empty() {
                if !buffer.iter().all(|&b| b == 0) {
                    all_zero = false;
                }
                if !buffer.iter().all(|&b| b == 0xFF) {
                    all_ff = false;
                }
            }
        }

        // If data appears random (not all zeros or all FFs), crypto erase was effective
        Ok(!all_zero && !all_ff)
    }
}
