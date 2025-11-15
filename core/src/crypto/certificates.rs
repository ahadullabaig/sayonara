use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WipeCertificate {
    pub certificate_id: String,
    pub device_info: DeviceCertInfo,
    pub wipe_details: WipeDetails,
    pub verification: VerificationResult,
    pub timestamp: DateTime<Utc>,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceCertInfo {
    pub device_path: String,
    pub model: String,
    pub serial: String,
    pub size: u64,
    pub device_hash: String, // Hash of device identifying information
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WipeDetails {
    pub algorithm_used: String,
    pub passes_completed: u32,
    pub duration_seconds: u64,
    pub operator_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    pub verified: bool,
    pub entropy_score: f64,
    pub recovery_test_passed: bool,
    pub verification_timestamp: DateTime<Utc>,
}

pub struct CertificateGenerator {
    private_key: String, // In practice, use proper key management
}

impl Default for CertificateGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl CertificateGenerator {
    pub fn new() -> Self {
        // In production, load this from secure key store
        Self {
            private_key: "your-private-signing-key".to_string(),
        }
    }

    pub fn generate_certificate(
        &self,
        device_info: &crate::DriveInfo,
        wipe_details: WipeDetails,
        verification: VerificationResult,
    ) -> Result<WipeCertificate> {
        let certificate_id = Uuid::new_v4().to_string();

        let device_cert_info = DeviceCertInfo {
            device_path: device_info.device_path.clone(),
            model: device_info.model.clone(),
            serial: device_info.serial.clone(),
            size: device_info.size,
            device_hash: self.calculate_device_hash(device_info)?,
        };

        let mut certificate = WipeCertificate {
            certificate_id,
            device_info: device_cert_info,
            wipe_details,
            verification,
            timestamp: Utc::now(),
            signature: String::new(), // Will be filled by signing
        };

        certificate.signature = self.sign_certificate(&certificate)?;

        Ok(certificate)
    }

    fn calculate_device_hash(&self, device_info: &crate::DriveInfo) -> Result<String> {
        let mut hasher = Sha256::new();
        hasher.update(device_info.model.as_bytes());
        hasher.update(device_info.serial.as_bytes());
        hasher.update(device_info.size.to_le_bytes());

        Ok(format!("{:x}", hasher.finalize()))
    }

    fn sign_certificate(&self, certificate: &WipeCertificate) -> Result<String> {
        // Create a signing payload (excluding the signature field)
        let mut signing_data = certificate.clone();
        signing_data.signature = String::new();

        let json_data = serde_json::to_string(&signing_data)?;

        let mut hasher = Sha256::new();
        hasher.update(json_data.as_bytes());
        hasher.update(self.private_key.as_bytes());

        Ok(format!("{:x}", hasher.finalize()))
    }

    pub fn verify_certificate(&self, certificate: &WipeCertificate) -> Result<bool> {
        let expected_signature = self.sign_certificate(certificate)?;
        Ok(expected_signature == certificate.signature)
    }

    pub fn save_certificate(&self, certificate: &WipeCertificate, path: &str) -> Result<()> {
        let json_data = serde_json::to_string_pretty(certificate)?;
        std::fs::write(path, json_data)?;
        Ok(())
    }
}
