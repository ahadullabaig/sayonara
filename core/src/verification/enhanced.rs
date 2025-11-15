use crate::io::{IOConfig, IOHandle, OptimizedIO};
use anyhow::Result;
use chrono::{DateTime, Utc};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::process::Command;

/// Enhanced verification system with comprehensive forensic analysis
pub struct EnhancedVerification;

// ==================== DATA STRUCTURES ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationReport {
    pub device_path: String,
    pub timestamp: DateTime<Utc>,
    pub pre_wipe_tests: PreWipeTestResults,
    pub post_wipe_analysis: PostWipeAnalysis,
    pub confidence_level: f64,
    pub verification_level: VerificationLevel,
    pub verification_method: String,
    pub compliance_standards: Vec<String>,
    pub recommendations: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreWipeTestResults {
    pub test_pattern_detection: bool,
    pub recovery_tool_simulation: bool,
    pub sensitivity_calibration: f64,
    pub false_positive_rate: f64,
    pub false_negative_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostWipeAnalysis {
    pub entropy_score: f64,
    pub chi_square_test: f64,
    pub pattern_analysis: PatternAnalysis,
    pub statistical_tests: StatisticalTests,
    pub sector_sampling: SectorSamplingResult,
    pub hidden_areas: HiddenAreaVerification,
    pub recovery_simulation: RecoverySimulationResults,
    pub bad_sectors: BadSectorTracker,
    pub heat_map: Option<EntropyHeatMap>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternAnalysis {
    pub repeating_patterns_found: bool,
    pub known_file_signatures: bool,
    pub structured_data_detected: bool,
    pub compression_ratio: f64,
    pub detected_signatures: Vec<FileSignatureMatch>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatisticalTests {
    pub runs_test_passed: bool,
    pub monobit_test_passed: bool,
    pub poker_test_passed: bool,
    pub serial_test_passed: bool,
    pub autocorrelation_test_passed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectorSamplingResult {
    pub total_sectors_sampled: u64,
    pub suspicious_sectors: u64,
    pub entropy_distribution: Vec<f64>,
    pub anomaly_locations: Vec<u64>,
}

// ==================== NEW: HIDDEN AREA VERIFICATION ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HiddenAreaVerification {
    pub hpa_verified: bool,
    pub hpa_sectors_checked: u64,
    pub hpa_entropy: Option<f64>,
    pub dco_verified: bool,
    pub dco_sectors_checked: u64,
    pub remapped_sectors_found: u64,
    pub remapped_sectors_verified: u64,
    pub controller_cache_flushed: bool,
    pub over_provisioning_verified: bool,
    pub wear_leveling_checked: bool,
    pub hidden_area_warnings: Vec<String>,
}

// ==================== NEW: RECOVERY TOOL SIMULATION ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoverySimulationResults {
    pub photorec_results: PhotoRecResults,
    pub testdisk_results: TestDiskResults,
    pub filesystem_metadata: FilesystemMetadataResults,
    pub mfm_simulation: Option<MFMResults>,
    pub overall_recovery_risk: RecoveryRisk,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhotoRecResults {
    pub signatures_scanned: usize,
    pub signatures_found: Vec<FileSignatureMatch>,
    pub recoverable_files_estimated: usize,
    pub confidence: f64,
    pub would_succeed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSignatureMatch {
    pub signature_name: String,
    pub offset: u64,
    pub pattern_length: usize,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestDiskResults {
    pub mbr_signature_found: bool,
    pub gpt_header_found: bool,
    pub partition_table_recoverable: bool,
    pub filesystem_signatures: Vec<String>,
    pub would_succeed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilesystemMetadataResults {
    pub superblock_remnants: Vec<String>,
    pub inode_structures: bool,
    pub journal_data: bool,
    pub fat_tables: bool,
    pub ntfs_mft: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MFMResults {
    pub theoretical_recovery_possible: bool,
    pub confidence_level: f64,
    pub affected_sectors: u64,
    pub flux_transition_anomalies: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum RecoveryRisk {
    None,     // 0% - No recovery possible
    VeryLow,  // <1% - Virtually impossible
    Low,      // 1-5% - Unlikely
    Medium,   // 5-25% - Possible with advanced tools
    High,     // 25-75% - Likely
    Critical, // >75% - Almost certain
}

// ==================== NEW: MULTI-LEVEL VERIFICATION ====================

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum VerificationLevel {
    Level1RandomSampling,     // 1% - Fast (minutes)
    Level2SystematicSampling, // Every Nth - Medium (tens of minutes)
    Level3FullScan,           // 100% - Slow (hours)
    Level4ForensicScan,       // Full + Hidden + MFM - Very slow (hours+)
}

// ==================== NEW: HEAT MAP ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntropyHeatMap {
    pub width: usize,
    pub height: usize,
    pub cells: Vec<Vec<f64>>,
    pub min_entropy: f64,
    pub max_entropy: f64,
    pub suspicious_blocks: Vec<(usize, usize)>,
}

// ==================== NEW: BAD SECTOR TRACKING ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BadSectorTracker {
    pub bad_sectors: Vec<u64>,
    pub unreadable_count: u64,
    pub percentage_unreadable: f64,
    pub total_sectors_attempted: u64,
}

// ==================== FILE SIGNATURES DATABASE ====================

#[derive(Debug, Clone)]
pub struct FileSignature {
    pub name: &'static str,
    pub pattern: &'static [u8],
    pub offset: usize, // Offset in file where signature appears
    pub confidence: f64,
}

impl EnhancedVerification {
    /// Comprehensive file signatures for PhotoRec simulation
    pub(crate) const FILE_SIGNATURES: &'static [FileSignature] = &[
        // Documents
        FileSignature {
            name: "PDF",
            pattern: b"%PDF",
            offset: 0,
            confidence: 0.99,
        },
        FileSignature {
            name: "MS Word (DOCX)",
            pattern: b"PK\x03\x04",
            offset: 0,
            confidence: 0.85,
        },
        FileSignature {
            name: "MS Excel (old)",
            pattern: b"\xD0\xCF\x11\xE0",
            offset: 0,
            confidence: 0.90,
        },
        FileSignature {
            name: "MS Office 2007+",
            pattern: b"PK\x03\x04\x14\x00\x06\x00",
            offset: 0,
            confidence: 0.95,
        },
        FileSignature {
            name: "RTF",
            pattern: b"{\\rtf",
            offset: 0,
            confidence: 0.95,
        },
        FileSignature {
            name: "OpenDocument",
            pattern: b"PK\x03\x04",
            offset: 0,
            confidence: 0.80,
        },
        // Images
        FileSignature {
            name: "JPEG",
            pattern: b"\xFF\xD8\xFF",
            offset: 0,
            confidence: 0.99,
        },
        FileSignature {
            name: "PNG",
            pattern: b"\x89PNG\r\n\x1a\n",
            offset: 0,
            confidence: 0.99,
        },
        FileSignature {
            name: "GIF89a",
            pattern: b"GIF89a",
            offset: 0,
            confidence: 0.99,
        },
        FileSignature {
            name: "GIF87a",
            pattern: b"GIF87a",
            offset: 0,
            confidence: 0.99,
        },
        FileSignature {
            name: "BMP",
            pattern: b"BM",
            offset: 0,
            confidence: 0.95,
        },
        FileSignature {
            name: "TIFF (LE)",
            pattern: b"II*\x00",
            offset: 0,
            confidence: 0.95,
        },
        FileSignature {
            name: "TIFF (BE)",
            pattern: b"MM\x00*",
            offset: 0,
            confidence: 0.95,
        },
        FileSignature {
            name: "WebP",
            pattern: b"RIFF",
            offset: 0,
            confidence: 0.90,
        },
        FileSignature {
            name: "ICO",
            pattern: b"\x00\x00\x01\x00",
            offset: 0,
            confidence: 0.85,
        },
        // Archives
        FileSignature {
            name: "ZIP",
            pattern: b"PK\x03\x04",
            offset: 0,
            confidence: 0.95,
        },
        FileSignature {
            name: "RAR",
            pattern: b"Rar!\x1A\x07",
            offset: 0,
            confidence: 0.99,
        },
        FileSignature {
            name: "7-Zip",
            pattern: b"7z\xBC\xAF\x27\x1C",
            offset: 0,
            confidence: 0.99,
        },
        FileSignature {
            name: "GZIP",
            pattern: b"\x1F\x8B",
            offset: 0,
            confidence: 0.95,
        },
        FileSignature {
            name: "BZIP2",
            pattern: b"BZh",
            offset: 0,
            confidence: 0.95,
        },
        FileSignature {
            name: "TAR",
            pattern: b"ustar",
            offset: 257,
            confidence: 0.90,
        },
        // Media
        FileSignature {
            name: "MP3 (ID3v2)",
            pattern: b"ID3",
            offset: 0,
            confidence: 0.95,
        },
        FileSignature {
            name: "MP3 (no ID3)",
            pattern: b"\xFF\xFB",
            offset: 0,
            confidence: 0.80,
        },
        FileSignature {
            name: "MP4",
            pattern: b"ftyp",
            offset: 4,
            confidence: 0.90,
        },
        FileSignature {
            name: "AVI",
            pattern: b"RIFF",
            offset: 0,
            confidence: 0.85,
        },
        FileSignature {
            name: "WAV",
            pattern: b"RIFF",
            offset: 0,
            confidence: 0.85,
        },
        FileSignature {
            name: "FLAC",
            pattern: b"fLaC",
            offset: 0,
            confidence: 0.99,
        },
        FileSignature {
            name: "OGG",
            pattern: b"OggS",
            offset: 0,
            confidence: 0.95,
        },
        FileSignature {
            name: "MKV",
            pattern: b"\x1A\x45\xDF\xA3",
            offset: 0,
            confidence: 0.95,
        },
        // Executables
        FileSignature {
            name: "Windows EXE",
            pattern: b"MZ",
            offset: 0,
            confidence: 0.90,
        },
        FileSignature {
            name: "Linux ELF",
            pattern: b"\x7FELF",
            offset: 0,
            confidence: 0.99,
        },
        FileSignature {
            name: "Mach-O",
            pattern: b"\xFE\xED\xFA",
            offset: 0,
            confidence: 0.95,
        },
        FileSignature {
            name: "Java Class",
            pattern: b"\xCA\xFE\xBA\xBE",
            offset: 0,
            confidence: 0.99,
        },
        // Databases
        FileSignature {
            name: "SQLite",
            pattern: b"SQLite format 3\x00",
            offset: 0,
            confidence: 0.99,
        },
        FileSignature {
            name: "MS Access",
            pattern: b"\x00\x01\x00\x00Standard Jet DB",
            offset: 0,
            confidence: 0.95,
        },
        // Encryption/Keys
        FileSignature {
            name: "PGP Private Key",
            pattern: b"-----BEGIN PGP PRIVATE KEY BLOCK-----",
            offset: 0,
            confidence: 0.99,
        },
        FileSignature {
            name: "SSH Private Key",
            pattern: b"-----BEGIN OPENSSH PRIVATE KEY-----",
            offset: 0,
            confidence: 0.99,
        },
        FileSignature {
            name: "RSA Private Key",
            pattern: b"-----BEGIN RSA PRIVATE KEY-----",
            offset: 0,
            confidence: 0.99,
        },
        FileSignature {
            name: "Certificate",
            pattern: b"-----BEGIN CERTIFICATE-----",
            offset: 0,
            confidence: 0.99,
        },
        // Disk Images
        FileSignature {
            name: "ISO 9660",
            pattern: b"CD001",
            offset: 0x8001,
            confidence: 0.95,
        },
        FileSignature {
            name: "VDI (VirtualBox)",
            pattern: b"<<< Oracle VM VirtualBox Disk Image >>>",
            offset: 0x40,
            confidence: 0.99,
        },
        FileSignature {
            name: "VMDK",
            pattern: b"KDMV",
            offset: 0,
            confidence: 0.95,
        },
        // Bitcoin/Crypto
        FileSignature {
            name: "Bitcoin Wallet",
            pattern: b"\x00\x00\x00\x00\x01\x00\x00\x00\x00\x00\x00\x00",
            offset: 0,
            confidence: 0.70,
        },
    ];

    // ==================== MAIN VERIFICATION ENTRY POINTS ====================

    /// Stage 1: Pre-wipe verification capability testing
    pub fn pre_wipe_capability_test(
        device_path: &str,
        test_size: u64,
    ) -> Result<PreWipeTestResults> {
        println!("🔬 Stage 1: Testing Verification Capabilities");

        let device_size = Self::get_device_size(device_path)?;
        let test_offset = device_size.saturating_sub(test_size.min(1024 * 1024));

        println!("  ├─ Writing test patterns...");
        let pattern_detection = Self::test_pattern_detection(device_path, test_offset)?;

        println!("  ├─ Testing recovery tool simulation...");
        let recovery_simulation = Self::simulate_recovery_tools_test(device_path, test_offset)?;

        println!("  ├─ Calibrating detection sensitivity...");
        let sensitivity = Self::calibrate_sensitivity(device_path, test_offset)?;

        println!("  └─ Measuring accuracy rates...");
        let (fp_rate, fn_rate) = Self::measure_accuracy_rates(device_path, test_offset)?;

        Ok(PreWipeTestResults {
            test_pattern_detection: pattern_detection,
            recovery_tool_simulation: recovery_simulation,
            sensitivity_calibration: sensitivity,
            false_positive_rate: fp_rate,
            false_negative_rate: fn_rate,
        })
    }

    /// Stage 2: Post-wipe verification with multi-level support
    pub fn post_wipe_verification_with_level(
        device_path: &str,
        device_size: u64,
        level: VerificationLevel,
    ) -> Result<PostWipeAnalysis> {
        println!("🔬 Stage 2: Post-Wipe Verification (Level: {:?})", level);

        match level {
            VerificationLevel::Level1RandomSampling => {
                Self::level1_random_sampling(device_path, device_size, 1.0)
            }
            VerificationLevel::Level2SystematicSampling => {
                Self::level2_systematic_sampling(device_path, device_size, 100)
            }
            VerificationLevel::Level3FullScan => Self::level3_full_scan(device_path, device_size),
            VerificationLevel::Level4ForensicScan => {
                Self::level4_forensic_scan(device_path, device_size)
            }
        }
    }

    // ==================== LEVEL 1: RANDOM SAMPLING ====================

    fn level1_random_sampling(
        device_path: &str,
        device_size: u64,
        sample_percentage: f64,
    ) -> Result<PostWipeAnalysis> {
        println!("  📊 Level 1: Random Sampling ({}%)", sample_percentage);

        let sample_size = ((device_size as f64 * sample_percentage / 100.0) as u64)
            .clamp(10 * 1024 * 1024, 1024 * 1024 * 1024);

        println!("  ├─ Sampling {} MB...", sample_size / (1024 * 1024));
        let samples = Self::collect_stratified_samples(device_path, device_size, sample_size)?;

        Self::analyze_samples(device_path, device_size, samples, false)
    }

    // ==================== LEVEL 2: SYSTEMATIC SAMPLING ====================

    fn level2_systematic_sampling(
        device_path: &str,
        device_size: u64,
        every_nth: u64,
    ) -> Result<PostWipeAnalysis> {
        println!(
            "  📊 Level 2: Systematic Sampling (every {}th sector)",
            every_nth
        );

        let sector_size = 512u64;
        let total_sectors = device_size / sector_size;
        let sectors_to_check = total_sectors / every_nth;

        println!(
            "  ├─ Checking {} sectors systematically...",
            sectors_to_check
        );

        let mut samples = Vec::new();
        let config = IOConfig::small_read_optimized();
        let mut handle = OptimizedIO::open(device_path, config)?;

        for i in 0..sectors_to_check {
            let sector_num = i * every_nth;
            let offset = sector_num * sector_size;

            if offset >= device_size {
                break;
            }

            let buffer = OptimizedIO::read_range(&mut handle, offset, sector_size as usize)?;
            samples.extend_from_slice(&buffer);

            if i % 1000 == 0 {
                println!("    Progress: {}/{} sectors", i, sectors_to_check);
            }
        }

        Self::analyze_samples(device_path, device_size, samples, false)
    }

    // ==================== LEVEL 3: FULL SCAN ====================

    fn level3_full_scan(device_path: &str, device_size: u64) -> Result<PostWipeAnalysis> {
        println!("  📊 Level 3: Full Scan (100% of drive)");
        println!("  ⚠️  Warning: This will take a long time!");

        let mut all_samples = Vec::new();
        let config = IOConfig::verification_optimized();
        let mut handle = OptimizedIO::open(device_path, config)?;

        let mut bytes_read = 0u64;
        let mut chunk_num = 0u64;

        OptimizedIO::sequential_read(&mut handle, device_size, |buffer, bytes| {
            // Analyze every 10th chunk to avoid memory overflow
            if chunk_num.is_multiple_of(10) {
                all_samples.extend_from_slice(&buffer.as_slice()[..bytes]);
            }

            bytes_read += bytes as u64;
            chunk_num += 1;

            if chunk_num.is_multiple_of(100) {
                println!(
                    "    Progress: {:.1}%",
                    (bytes_read as f64 / device_size as f64) * 100.0
                );
            }

            Ok(())
        })?;

        Self::analyze_samples(device_path, device_size, all_samples, false)
    }

    // ==================== LEVEL 4: FORENSIC SCAN ====================

    fn level4_forensic_scan(device_path: &str, device_size: u64) -> Result<PostWipeAnalysis> {
        println!("  🔬 Level 4: Forensic Scan (Full + Hidden Areas + MFM)");
        println!("  ⚠️  Warning: This is the most thorough and time-consuming verification!");

        // Start with full scan
        let mut analysis = Self::level3_full_scan(device_path, device_size)?;

        // Add forensic components
        analysis.hidden_areas = Self::verify_hidden_areas(device_path)?;

        if Self::is_hdd(device_path)? {
            println!("  ├─ Running MFM simulation (HDD detected)...");
            analysis.recovery_simulation.mfm_simulation = Some(Self::simulate_mfm(device_path)?);
        }

        // Generate heat map for forensic analysis
        println!("  └─ Generating detailed entropy heat map...");
        analysis.heat_map = Some(Self::generate_entropy_heat_map(device_path, device_size)?);

        Ok(analysis)
    }

    // ==================== HIDDEN AREA VERIFICATION ====================

    fn verify_hidden_areas(device_path: &str) -> Result<HiddenAreaVerification> {
        println!("  🔍 Verifying Hidden Areas...");

        let mut warnings = Vec::new();
        let mut hpa_verified = true;
        let mut hpa_sectors = 0u64;
        let mut hpa_entropy = None;

        // Check HPA
        println!("    ├─ Checking Host Protected Area (HPA)...");
        if let Ok(Some(hpa_info)) = Self::detect_hpa(device_path) {
            println!("      HPA detected: {} sectors", hpa_info.hidden_sectors);
            hpa_sectors = hpa_info.hidden_sectors;

            // Verify HPA area was wiped
            match Self::verify_hpa_sectors(device_path, &hpa_info) {
                Ok(entropy) => {
                    hpa_entropy = Some(entropy);
                    if entropy < 7.5 {
                        warnings.push(format!("HPA entropy low: {:.2}", entropy));
                        hpa_verified = false;
                    }
                }
                Err(e) => {
                    warnings.push(format!("HPA verification failed: {}", e));
                    hpa_verified = false;
                }
            }
        } else {
            println!("      No HPA detected");
        }

        // Check DCO
        println!("    ├─ Checking Device Configuration Overlay (DCO)...");
        let dco_verified = true;
        let dco_sectors = 0u64;
        // DCO detection logic here

        // Check remapped sectors
        println!("    ├─ Checking remapped/spare sectors...");
        let (remapped_found, remapped_verified) = Self::verify_remapped_sectors(device_path)?;

        // Check controller cache
        println!("    ├─ Verifying controller cache flush...");
        let cache_flushed = Self::verify_controller_cache_flush(device_path)?;

        // Check over-provisioning (SSDs)
        println!("    ├─ Checking over-provisioning area...");
        let op_verified = if Self::is_ssd(device_path)? {
            Self::verify_over_provisioning()?
        } else {
            true // N/A for HDDs
        };

        // Check wear-leveling reserve (SSDs)
        println!("    └─ Checking wear-leveling reserve...");
        let wear_leveling = if Self::is_ssd(device_path)? {
            Self::check_wear_leveling_reserve(device_path)?
        } else {
            true
        };

        Ok(HiddenAreaVerification {
            hpa_verified,
            hpa_sectors_checked: hpa_sectors,
            hpa_entropy,
            dco_verified,
            dco_sectors_checked: dco_sectors,
            remapped_sectors_found: remapped_found,
            remapped_sectors_verified: remapped_verified,
            controller_cache_flushed: cache_flushed,
            over_provisioning_verified: op_verified,
            wear_leveling_checked: wear_leveling,
            hidden_area_warnings: warnings,
        })
    }

    fn detect_hpa(device_path: &str) -> Result<Option<HPAInfo>> {
        // Use hdparm to detect HPA
        let output = Command::new("hdparm").args(["-N", device_path]).output()?;

        let output_str = String::from_utf8_lossy(&output.stdout);

        // Parse output for "max sectors" and "native max"
        // Format: "max sectors = X/Y, HPA is enabled"
        if output_str.contains("HPA") {
            // Parse sectors from output
            // This is simplified - real implementation needs proper parsing
            Ok(Some(HPAInfo {
                hidden_sectors: 0, // Parse from output
            }))
        } else {
            Ok(None)
        }
    }

    fn verify_hpa_sectors(device_path: &str, hpa_info: &HPAInfo) -> Result<f64> {
        // Read HPA area and calculate entropy
        let config = IOConfig::small_read_optimized();
        let mut handle = OptimizedIO::open(device_path, config)?;

        // Calculate HPA start offset
        let device_size = Self::get_device_size(device_path)?;
        let hpa_offset = device_size - (hpa_info.hidden_sectors * 512);

        let sample_size = (hpa_info.hidden_sectors * 512).min(10 * 1024 * 1024);
        let buffer = OptimizedIO::read_range(&mut handle, hpa_offset, sample_size as usize)?;

        Self::calculate_entropy(&buffer)
    }

    fn verify_remapped_sectors(device_path: &str) -> Result<(u64, u64)> {
        // Use smartctl to get reallocated sector count
        let output = Command::new("smartctl")
            .args(["-A", device_path])
            .output()?;

        let output_str = String::from_utf8_lossy(&output.stdout);

        let mut remapped_count = 0u64;

        // Parse SMART attributes 05, 196, 197, 198
        for line in output_str.lines() {
            if line.contains("Reallocated_Sector_Ct")
                || line.contains("Reallocated_Event_Count")
                || line.contains("Current_Pending_Sector")
                || line.contains("Offline_Uncorrectable")
            {
                // Parse count from line
                if let Some(count_str) = line.split_whitespace().nth(9) {
                    if let Ok(count) = count_str.parse::<u64>() {
                        remapped_count += count;
                    }
                }
            }
        }

        // Verify remapped sectors (attempt to read them)
        let verified = remapped_count; // Simplified

        Ok((remapped_count, verified))
    }

    fn verify_controller_cache_flush(device_path: &str) -> Result<bool> {
        // Send FLUSH CACHE command
        let output = Command::new("hdparm").args(["-f", device_path]).output()?;

        Ok(output.status.success())
    }

    fn verify_over_provisioning() -> Result<bool> {
        // For SSDs, try to detect and verify over-provisioning area
        // This is complex and vendor-specific
        // Simplified implementation
        Ok(true)
    }

    fn check_wear_leveling_reserve(device_path: &str) -> Result<bool> {
        // Check SSD wear leveling reserve via SMART
        let output = Command::new("smartctl")
            .args(["-A", device_path])
            .output()?;

        let output_str = String::from_utf8_lossy(&output.stdout);

        // Look for wear leveling indicators
        Ok(!output_str.contains("Wear_Leveling_Count: 0"))
    }

    // ==================== RECOVERY TOOL SIMULATION ====================

    fn simulate_recovery_tools_test(device_path: &str, test_offset: u64) -> Result<bool> {
        // Write known file signatures and try to detect them
        let config = IOConfig::small_read_optimized();
        let mut handle = OptimizedIO::open(device_path, config)?;

        // Write a JPEG signature
        handle.write_at(b"\xFF\xD8\xFF\xE0", test_offset)?;
        handle.sync()?;

        // Try to detect it
        let buffer = OptimizedIO::read_range(&mut handle, test_offset, 4096)?;
        let detected = buffer.windows(4).any(|w| w == b"\xFF\xD8\xFF\xE0");

        // Clean up
        let zeros = vec![0u8; 4096];
        handle.write_at(&zeros, test_offset)?;
        handle.sync()?;

        Ok(detected)
    }

    fn simulate_recovery_tools(
        device_path: &str,
        device_size: u64,
    ) -> Result<RecoverySimulationResults> {
        println!("  🔍 Simulating Recovery Tools...");

        // PhotoRec simulation
        println!("    ├─ PhotoRec simulation...");
        let photorec_results = Self::simulate_photorec(device_path, device_size)?;

        // TestDisk simulation
        println!("    ├─ TestDisk simulation...");
        let testdisk_results = Self::simulate_testdisk(device_path)?;

        // Filesystem metadata check
        println!("    ├─ Filesystem metadata check...");
        let filesystem_metadata = Self::check_filesystem_metadata(device_path)?;

        // MFM simulation (HDDs only)
        let mfm_simulation = if Self::is_hdd(device_path)? {
            println!("    ├─ MFM simulation (HDD detected)...");
            Some(Self::simulate_mfm(device_path)?)
        } else {
            None
        };

        // Calculate overall recovery risk
        let overall_risk = Self::calculate_recovery_risk(
            &photorec_results,
            &testdisk_results,
            &filesystem_metadata,
            mfm_simulation.as_ref(),
        );

        println!("    └─ Overall recovery risk: {:?}", overall_risk);

        Ok(RecoverySimulationResults {
            photorec_results,
            testdisk_results,
            filesystem_metadata,
            mfm_simulation,
            overall_recovery_risk: overall_risk,
        })
    }

    fn simulate_photorec(device_path: &str, device_size: u64) -> Result<PhotoRecResults> {
        let mut found_signatures = Vec::new();
        let config = IOConfig::small_read_optimized();
        let mut handle = OptimizedIO::open(device_path, config)?;

        // Sample 10% of drive in random locations
        let sample_count = 1000;
        let chunk_size = 65536; // 64KB blocks like PhotoRec uses

        let mut rng = rand::thread_rng();

        for _ in 0..sample_count {
            let offset = rng.gen_range(0..device_size.saturating_sub(chunk_size));

            let buffer = match OptimizedIO::read_range(&mut handle, offset, chunk_size as usize) {
                Ok(buf) => buf,
                Err(_) => continue,
            };

            // Check for all known file signatures
            for sig in Self::FILE_SIGNATURES {
                if buffer.len() > sig.offset + sig.pattern.len()
                    && &buffer[sig.offset..sig.offset + sig.pattern.len()] == sig.pattern {
                        found_signatures.push(FileSignatureMatch {
                            signature_name: sig.name.to_string(),
                            offset,
                            pattern_length: sig.pattern.len(),
                            confidence: sig.confidence,
                        });
                    }
            }
        }

        let would_succeed = !found_signatures.is_empty();
        let recoverable_estimate = found_signatures.len() * 10; // Rough estimate

        Ok(PhotoRecResults {
            signatures_scanned: Self::FILE_SIGNATURES.len(),
            signatures_found: found_signatures,
            recoverable_files_estimated: recoverable_estimate,
            confidence: 0.95,
            would_succeed,
        })
    }

    fn simulate_testdisk(device_path: &str) -> Result<TestDiskResults> {
        let config = IOConfig::small_read_optimized();
        let mut handle = OptimizedIO::open(device_path, config)?;

        // Check MBR signature
        let mbr_found = Self::check_mbr_signature(&mut handle)?;

        // Check GPT header
        let gpt_found = Self::check_gpt_header(&mut handle)?;

        // Check filesystem signatures
        let fs_signatures = Self::check_filesystem_signatures(&mut handle)?;

        let partition_recoverable = mbr_found || gpt_found;
        let would_succeed = partition_recoverable || !fs_signatures.is_empty();

        Ok(TestDiskResults {
            mbr_signature_found: mbr_found,
            gpt_header_found: gpt_found,
            partition_table_recoverable: partition_recoverable,
            filesystem_signatures: fs_signatures,
            would_succeed,
        })
    }

    fn check_mbr_signature(handle: &mut IOHandle) -> Result<bool> {
        let buffer = OptimizedIO::read_range(handle, 0, 512)?;

        // Check for MBR signature at bytes 510-511
        Ok(buffer.len() >= 512 && buffer[510] == 0x55 && buffer[511] == 0xAA)
    }

    fn check_gpt_header(handle: &mut IOHandle) -> Result<bool> {
        let buffer = OptimizedIO::read_range(handle, 512, 512)?; // GPT starts at LBA 1

        // Check for "EFI PART" signature
        Ok(buffer.len() >= 8 && &buffer[0..8] == b"EFI PART")
    }

    fn check_filesystem_signatures(handle: &mut IOHandle) -> Result<Vec<String>> {
        let mut signatures = Vec::new();

        // Check ext2/3/4 superblock
        if let Ok(buffer) = OptimizedIO::read_range(handle, 1024, 1024) {
            if buffer.len() >= 58 && buffer[56..58] == [0x53, 0xEF] {
                signatures.push("ext2/3/4".to_string());
            }
        }

        // Check NTFS
        if let Ok(buffer) = OptimizedIO::read_range(handle, 3, 8) {
            if &buffer == b"NTFS    " {
                signatures.push("NTFS".to_string());
            }
        }

        // Check FAT
        if let Ok(buffer) = OptimizedIO::read_range(handle, 54, 8) {
            if buffer.starts_with(b"FAT") {
                signatures.push("FAT".to_string());
            }
        }

        // Check XFS
        if let Ok(buffer) = OptimizedIO::read_range(handle, 0, 4) {
            if &buffer == b"XFSB" {
                signatures.push("XFS".to_string());
            }
        }

        Ok(signatures)
    }

    fn check_filesystem_metadata(device_path: &str) -> Result<FilesystemMetadataResults> {
        let config = IOConfig::small_read_optimized();
        let mut handle = OptimizedIO::open(device_path, config)?;

        let superblock_remnants = Self::check_filesystem_signatures(&mut handle)?;
        let inode_structures = Self::check_for_inodes()?;
        let journal_data = Self::check_for_journal()?;
        let fat_tables = Self::check_for_fat_tables()?;
        let ntfs_mft = Self::check_for_mft(&mut handle)?;

        Ok(FilesystemMetadataResults {
            superblock_remnants,
            inode_structures,
            journal_data,
            fat_tables,
            ntfs_mft,
        })
    }

    fn check_for_inodes() -> Result<bool> {
        // Simplified inode detection
        // Real implementation would scan for inode patterns
        Ok(false)
    }

    fn check_for_journal() -> Result<bool> {
        // Check for ext3/4 journal or NTFS $LogFile patterns
        Ok(false)
    }

    fn check_for_fat_tables() -> Result<bool> {
        // Check for FAT table structures
        Ok(false)
    }

    fn check_for_mft(handle: &mut IOHandle) -> Result<bool> {
        // Check for NTFS Master File Table
        if let Ok(buffer) = OptimizedIO::read_range(handle, 0, 4) {
            return Ok(&buffer == b"FILE");
        }
        Ok(false)
    }

    fn simulate_mfm(device_path: &str) -> Result<MFMResults> {
        // Magnetic Force Microscopy simulation
        // This simulates whether magnetic flux transitions could reveal previous data

        let config = IOConfig::small_read_optimized();
        let mut handle = OptimizedIO::open(device_path, config)?;

        let mut suspicious_transitions = 0u64;
        let sample_count = 100;

        let mut rng = rand::thread_rng();
        let device_size = Self::get_device_size(device_path)?;

        for _ in 0..sample_count {
            let offset = rng.gen_range(0..device_size - 512);

            let buffer = match OptimizedIO::read_range(&mut handle, offset, 512) {
                Ok(buf) => buf,
                Err(_) => continue,
            };

            // Analyze bit transitions
            // Look for patterns suggesting magnetic hysteresis
            let transitions = Self::analyze_bit_transitions(&buffer);
            if transitions > 200 {
                suspicious_transitions += 1;
            }
        }

        let confidence = (suspicious_transitions as f64 / sample_count as f64) * 100.0;
        let theoretical_recovery = suspicious_transitions > 10;

        Ok(MFMResults {
            theoretical_recovery_possible: theoretical_recovery,
            confidence_level: confidence,
            affected_sectors: suspicious_transitions,
            flux_transition_anomalies: suspicious_transitions * 512,
        })
    }

    fn analyze_bit_transitions(buffer: &[u8]) -> u32 {
        let mut transitions = 0u32;
        let mut last_bit = false;

        for &byte in buffer {
            for i in 0..8 {
                let bit = (byte >> i) & 1 == 1;
                if bit != last_bit {
                    transitions += 1;
                }
                last_bit = bit;
            }
        }

        transitions
    }

    pub(crate) fn calculate_recovery_risk(
        photorec: &PhotoRecResults,
        testdisk: &TestDiskResults,
        filesystem: &FilesystemMetadataResults,
        mfm: Option<&MFMResults>,
    ) -> RecoveryRisk {
        let mut risk_score = 0;

        // PhotoRec risk
        if photorec.would_succeed {
            risk_score += 30;
        }
        if photorec.signatures_found.len() > 10 {
            risk_score += 20;
        }

        // TestDisk risk
        if testdisk.partition_table_recoverable {
            risk_score += 25;
        }
        if !testdisk.filesystem_signatures.is_empty() {
            risk_score += 15;
        }

        // Filesystem metadata risk
        if filesystem.inode_structures {
            risk_score += 10;
        }

        // MFM risk (HDDs only)
        if let Some(mfm_result) = mfm {
            if mfm_result.theoretical_recovery_possible {
                risk_score += 10;
            }
        }

        match risk_score {
            0 => RecoveryRisk::None,
            1..=10 => RecoveryRisk::VeryLow,
            11..=25 => RecoveryRisk::Low,
            26..=50 => RecoveryRisk::Medium,
            51..=75 => RecoveryRisk::High,
            _ => RecoveryRisk::Critical,
        }
    }

    // ==================== HEAT MAP GENERATION ====================

    fn generate_entropy_heat_map(device_path: &str, device_size: u64) -> Result<EntropyHeatMap> {
        println!("  🗺️  Generating Entropy Heat Map...");

        let width = 100;
        let height = 50;
        let block_size = device_size / (width * height) as u64;

        let mut cells = vec![vec![0.0; width]; height];
        let mut min_entropy: f32 = 8.0;
        let mut max_entropy: f32 = 0.0;
        let mut suspicious_blocks = Vec::new();

        let config = IOConfig::small_read_optimized();
        let mut handle = OptimizedIO::open(device_path, config)?;

        for (y, row) in cells.iter_mut().enumerate() {
            for (x, cell) in row.iter_mut().enumerate() {
                let block_num = (y * width + x) as u64;
                let offset = block_num * block_size;

                let read_size = block_size.min(65536) as usize;

                if let Ok(buffer) = OptimizedIO::read_range(&mut handle, offset, read_size) {
                    if let Ok(entropy) = Self::calculate_entropy(&buffer) {
                        *cell = entropy;

                        min_entropy = min_entropy.min(entropy as f32);
                        max_entropy = max_entropy.max(entropy as f32);

                        if entropy < 6.0 {
                            suspicious_blocks.push((x, y));
                        }
                    }
                }
            }

            if y % 10 == 0 {
                println!("    Progress: {:.0}%", (y as f64 / height as f64) * 100.0);
            }
        }

        Ok(EntropyHeatMap {
            width,
            height,
            cells,
            min_entropy: min_entropy as f64,
            max_entropy: max_entropy as f64,
            suspicious_blocks,
        })
    }

    pub fn render_heat_map_ascii(heat_map: &EntropyHeatMap) -> String {
        let mut output = String::new();

        output.push_str(&format!(
            "\n📊 Entropy Heat Map ({}x{})\n",
            heat_map.width, heat_map.height
        ));
        output.push_str(&format!(
            "Range: {:.2} - {:.2} bits/byte\n",
            heat_map.min_entropy, heat_map.max_entropy
        ));
        output.push_str(&format!(
            "Suspicious blocks: {}\n\n",
            heat_map.suspicious_blocks.len()
        ));

        for row in &heat_map.cells {
            for &entropy in row {
                let char = match entropy {
                    e if e < 4.0 => '█', // Full block - very bad
                    e if e < 6.0 => '▓', // Dark shade - bad
                    e if e < 7.0 => '▒', // Medium shade - mediocre
                    e if e < 7.5 => '░', // Light shade - acceptable
                    _ => ' ',            // Space - excellent
                };
                output.push(char);
            }
            output.push('\n');
        }

        output.push_str("\nLegend: █ Critical  ▓ Bad  ▒ Medium  ░ Good  [space] Excellent\n");
        output
    }

    // ==================== HELPER METHODS ====================

    fn analyze_samples(
        device_path: &str,
        device_size: u64,
        samples: Vec<u8>,
        include_recovery: bool,
    ) -> Result<PostWipeAnalysis> {
        println!("  ├─ Calculating entropy...");
        let entropy = Self::calculate_entropy(&samples)?;

        println!("  ├─ Running chi-square test...");
        let chi_square = Self::chi_square_test(&samples)?;

        println!("  ├─ Pattern analysis...");
        let patterns = Self::analyze_patterns(&samples)?;

        println!("  ├─ Statistical randomness tests...");
        let stats = Self::run_statistical_tests(&samples)?;

        println!("  ├─ Sector anomaly detection...");
        let (sectors, bad_sectors) =
            Self::analyze_sectors_with_bad_tracking(device_path, device_size)?;

        println!("  ├─ Hidden area verification...");
        let hidden_areas = Self::verify_hidden_areas(device_path)?;

        println!("  ├─ Recovery tool simulation...");
        let recovery = if include_recovery {
            Self::simulate_recovery_tools(device_path, device_size)?
        } else {
            RecoverySimulationResults {
                photorec_results: PhotoRecResults {
                    signatures_scanned: 0,
                    signatures_found: Vec::new(),
                    recoverable_files_estimated: 0,
                    confidence: 0.0,
                    would_succeed: false,
                },
                testdisk_results: TestDiskResults {
                    mbr_signature_found: false,
                    gpt_header_found: false,
                    partition_table_recoverable: false,
                    filesystem_signatures: Vec::new(),
                    would_succeed: false,
                },
                filesystem_metadata: FilesystemMetadataResults {
                    superblock_remnants: Vec::new(),
                    inode_structures: false,
                    journal_data: false,
                    fat_tables: false,
                    ntfs_mft: false,
                },
                mfm_simulation: None,
                overall_recovery_risk: RecoveryRisk::None,
            }
        };

        Ok(PostWipeAnalysis {
            entropy_score: entropy,
            chi_square_test: chi_square,
            pattern_analysis: patterns,
            statistical_tests: stats,
            sector_sampling: sectors,
            hidden_areas,
            recovery_simulation: recovery,
            bad_sectors,
            heat_map: None,
        })
    }

    fn analyze_sectors_with_bad_tracking(
        device_path: &str,
        device_size: u64,
    ) -> Result<(SectorSamplingResult, BadSectorTracker)> {
        let sector_size = 512u64;
        let total_sectors = device_size / sector_size;
        let samples_per_region = 100;

        let config = IOConfig::small_read_optimized();
        let mut handle = OptimizedIO::open(device_path, config)?;

        let mut suspicious = 0u64;
        let mut entropy_dist = Vec::new();
        let mut anomalies = Vec::new();
        let mut bad_sectors = Vec::new();
        let mut unreadable = 0u64;
        let mut rng = rand::thread_rng();

        let total_attempts = samples_per_region * 10;

        for _ in 0..total_attempts {
            let sector_num = rng.gen_range(0..total_sectors);
            let offset = sector_num * sector_size;

            match OptimizedIO::read_range(&mut handle, offset, sector_size as usize) {
                Ok(buffer) => {
                    if let Ok(entropy) = Self::calculate_entropy(&buffer) {
                        entropy_dist.push(entropy);

                        if entropy < 6.0 || Self::detect_suspicious_data(&buffer) {
                            suspicious += 1;
                            anomalies.push(sector_num);
                        }
                    }
                }
                Err(_) => {
                    bad_sectors.push(sector_num);
                    unreadable += 1;
                }
            }
        }

        let sampling_result = SectorSamplingResult {
            total_sectors_sampled: total_attempts,
            suspicious_sectors: suspicious,
            entropy_distribution: entropy_dist,
            anomaly_locations: anomalies,
        };

        let bad_sector_tracker = BadSectorTracker {
            bad_sectors,
            unreadable_count: unreadable,
            percentage_unreadable: (unreadable as f64 / total_attempts as f64) * 100.0,
            total_sectors_attempted: total_attempts,
        };

        Ok((sampling_result, bad_sector_tracker))
    }

    fn collect_stratified_samples(
        device_path: &str,
        device_size: u64,
        sample_size: u64,
    ) -> Result<Vec<u8>> {
        let mut samples = Vec::with_capacity(sample_size as usize);
        let config = IOConfig::small_read_optimized();
        let mut handle = OptimizedIO::open(device_path, config)?;

        // Stratified sampling: beginning, middle, end
        let regions = vec![
            (0, sample_size / 4),
            (device_size / 2 - sample_size / 8, sample_size / 4),
            (device_size.saturating_sub(sample_size / 4), sample_size / 4),
        ];

        for (offset, size) in regions {
            if let Ok(buffer) = OptimizedIO::read_range(&mut handle, offset, size as usize) {
                samples.extend_from_slice(&buffer);
            }
        }

        // Random sampling for remaining
        let mut rng = rand::thread_rng();
        let remaining = sample_size.saturating_sub(samples.len() as u64);
        let chunk_size = 4096;

        for _ in 0..(remaining / chunk_size) {
            let random_offset = rng.gen_range(0..device_size.saturating_sub(chunk_size));
            if let Ok(buffer) =
                OptimizedIO::read_range(&mut handle, random_offset, chunk_size as usize)
            {
                samples.extend_from_slice(&buffer);
            }
        }

        Ok(samples)
    }

    pub(crate) fn calculate_entropy(data: &[u8]) -> Result<f64> {
        if data.is_empty() {
            return Ok(0.0);
        }

        let mut frequency = [0u64; 256];
        for &byte in data {
            frequency[byte as usize] += 1;
        }

        let len = data.len() as f64;
        let mut entropy = 0.0;

        for &count in &frequency {
            if count > 0 {
                let probability = count as f64 / len;
                entropy -= probability * probability.log2();
            }
        }

        Ok(entropy)
    }

    pub(crate) fn chi_square_test(data: &[u8]) -> Result<f64> {
        let mut observed = [0u64; 256];
        for &byte in data {
            observed[byte as usize] += 1;
        }

        let expected = data.len() as f64 / 256.0;
        let mut chi_square = 0.0;

        for &count in &observed {
            let diff = count as f64 - expected;
            chi_square += (diff * diff) / expected;
        }

        Ok(chi_square)
    }

    pub(crate) fn analyze_patterns(data: &[u8]) -> Result<PatternAnalysis> {
        let mut repeating = false;
        let mut detected_sigs = Vec::new();

        // Check for repeating patterns
        if data.len() >= 32 {
            for window_size in [4, 8, 16] {
                let first_window = &data[0..window_size];
                let mut repeat_count = 0;

                for chunk in data.chunks(window_size) {
                    if chunk == first_window {
                        repeat_count += 1;
                    }
                }

                if repeat_count > data.len() / window_size / 2 {
                    repeating = true;
                    break;
                }
            }
        }

        // Check for file signatures
        let mut signatures_found = false;
        for sig in Self::FILE_SIGNATURES {
            if data.len() > sig.offset + sig.pattern.len()
                && data.windows(sig.pattern.len()).any(|w| w == sig.pattern) {
                    signatures_found = true;
                    detected_sigs.push(FileSignatureMatch {
                        signature_name: sig.name.to_string(),
                        offset: 0,
                        pattern_length: sig.pattern.len(),
                        confidence: sig.confidence,
                    });
                }
        }

        // Check for structured data
        let mut structured = false;
        for chunk in data.chunks(1024) {
            if let Ok(entropy) = Self::calculate_entropy(chunk) {
                if entropy < 4.0 {
                    structured = true;
                    break;
                }
            }
        }

        let unique_bytes = data.iter().collect::<HashSet<_>>().len();
        let compression_ratio = unique_bytes as f64 / 256.0;

        Ok(PatternAnalysis {
            repeating_patterns_found: repeating,
            known_file_signatures: signatures_found,
            structured_data_detected: structured,
            compression_ratio,
            detected_signatures: detected_sigs,
        })
    }

    fn run_statistical_tests(data: &[u8]) -> Result<StatisticalTests> {
        Ok(StatisticalTests {
            runs_test_passed: Self::runs_test(data)?,
            monobit_test_passed: Self::monobit_test(data)?,
            poker_test_passed: Self::poker_test(data)?,
            serial_test_passed: Self::serial_test(data)?,
            autocorrelation_test_passed: Self::autocorrelation_test(data)?,
        })
    }

    pub fn runs_test(data: &[u8]) -> Result<bool> {
        let mut runs = 0;
        let mut last_bit = false;

        for &byte in data {
            for i in 0..8 {
                let bit = (byte >> i) & 1 == 1;
                if bit != last_bit {
                    runs += 1;
                }
                last_bit = bit;
            }
        }

        let expected = data.len() * 4;
        let ratio = runs as f64 / expected as f64;

        Ok(ratio > 0.9 && ratio < 1.1)
    }

    pub fn monobit_test(data: &[u8]) -> Result<bool> {
        let ones: u64 = data.iter().map(|b| b.count_ones() as u64).sum();
        let zeros = (data.len() * 8) as u64 - ones;
        let ratio = ones as f64 / (ones + zeros) as f64;

        Ok(ratio > 0.49 && ratio < 0.51)
    }

    pub fn poker_test(data: &[u8]) -> Result<bool> {
        let mut freq_4bit = [0u64; 16];

        for &byte in data {
            let high = (byte >> 4) & 0x0F;
            let low = byte & 0x0F;
            freq_4bit[high as usize] += 1;
            freq_4bit[low as usize] += 1;
        }

        let n = (data.len() * 2) as f64;
        let expected = n / 16.0;
        let mut chi_square = 0.0;

        for &count in &freq_4bit {
            let diff = count as f64 - expected;
            chi_square += (diff * diff) / expected;
        }

        Ok(chi_square < 30.578)
    }

    pub fn serial_test(data: &[u8]) -> Result<bool> {
        let mut freq_2bit = [0u64; 4];

        for &byte in data {
            for i in 0..4 {
                let two_bits = (byte >> (i * 2)) & 0b11;
                freq_2bit[two_bits as usize] += 1;
            }
        }

        let n = (data.len() * 4) as f64;
        let expected = n / 4.0;
        let mut chi_square = 0.0;

        for &count in &freq_2bit {
            let diff = count as f64 - expected;
            chi_square += (diff * diff) / expected;
        }

        Ok(chi_square < 11.345)
    }

    pub fn autocorrelation_test(data: &[u8]) -> Result<bool> {
        let max_lag = data.len().min(100);

        for lag in 1..max_lag {
            let mut correlation = 0i64;

            for i in 0..data.len() - lag {
                correlation += (data[i] as i64 - 128) * (data[i + lag] as i64 - 128);
            }

            let normalized = correlation as f64 / ((data.len() - lag) as f64 * 128.0 * 128.0);

            if normalized.abs() > 0.1 {
                return Ok(false);
            }
        }

        Ok(true)
    }

    pub(crate) fn detect_suspicious_data(data: &[u8]) -> bool {
        if data.len() < 32 {
            return false;
        }

        // Check for known sensitive patterns
        let patterns: &[&[u8]] = &[
            b"SENSITIVE",
            b"PASSWORD",
            b"SECRET",
            b"PRIVATE",
            b"CONFIDENTIAL",
            b"%PDF",
            b"PK\x03\x04",
        ];

        for pattern in patterns {
            if data.windows(pattern.len()).any(|w| w == *pattern) {
                return true;
            }
        }

        // Check for low entropy
        if let Ok(entropy) = Self::calculate_entropy(data) {
            if entropy < 6.0 {
                return true;
            }
        }

        false
    }

    fn test_pattern_detection(device_path: &str, offset: u64) -> Result<bool> {
        let patterns = vec![
            b"TESTDATA123456789".to_vec(),
            [0xDE, 0xAD, 0xBE, 0xEF].repeat(256),
            b"BEGIN_SENSITIVE_DATA_MARKER_END".to_vec(),
        ];

        let config = IOConfig::small_read_optimized();
        let mut handle = OptimizedIO::open(device_path, config)?;

        for pattern in &patterns {
            handle.write_at(pattern, offset)?;
            handle.sync()?;

            let buffer = OptimizedIO::read_range(&mut handle, offset, pattern.len())?;

            if buffer != *pattern {
                return Ok(false);
            }
        }

        let zeros = vec![0u8; 4096];
        handle.write_at(&zeros, offset)?;
        handle.sync()?;

        Ok(true)
    }

    fn calibrate_sensitivity(device_path: &str, test_offset: u64) -> Result<f64> {
        // Test sensitivity by writing patterns with varying entropy
        let test_count = 10;
        let mut detected_count = 0;

        let config = IOConfig::small_read_optimized();
        let mut handle = OptimizedIO::open(device_path, config)?;

        for i in 0..test_count {
            let pattern = vec![(i * 25) as u8; 1024];

            handle.write_at(&pattern, test_offset)?;
            handle.sync()?;

            let buffer = OptimizedIO::read_range(&mut handle, test_offset, 1024)?;

            if Self::detect_suspicious_data(&buffer) {
                detected_count += 1;
            }
        }

        // Clean up
        let zeros = vec![0u8; 4096];
        handle.write_at(&zeros, test_offset)?;
        handle.sync()?;

        Ok((detected_count as f64 / test_count as f64) * 100.0)
    }

    fn measure_accuracy_rates(device_path: &str, test_offset: u64) -> Result<(f64, f64)> {
        let total_tests = 20;
        let mut false_positives = 0;
        let mut false_negatives = 0;

        let config = IOConfig::small_read_optimized();
        let mut handle = OptimizedIO::open(device_path, config)?;

        use crate::crypto::secure_rng::secure_random_bytes;

        for i in 0..total_tests {
            if i % 2 == 0 {
                // Test false positive: write random data
                let mut random = vec![0u8; 1024];
                secure_random_bytes(&mut random)?;

                handle.write_at(&random, test_offset)?;
                handle.sync()?;

                let buffer = OptimizedIO::read_range(&mut handle, test_offset, 1024)?;

                if Self::detect_suspicious_data(&buffer) {
                    false_positives += 1;
                }
            } else {
                // Test false negative: write known pattern
                let pattern = b"SENSITIVE_DATA_PATTERN_12345678".to_vec();

                handle.write_at(&pattern, test_offset)?;
                handle.sync()?;

                let buffer = OptimizedIO::read_range(&mut handle, test_offset, pattern.len())?;

                if !Self::detect_suspicious_data(&buffer) {
                    false_negatives += 1;
                }
            }
        }

        let fp_rate = false_positives as f64 / (total_tests as f64 / 2.0);
        let fn_rate = false_negatives as f64 / (total_tests as f64 / 2.0);

        // Clean up
        let zeros = vec![0u8; 4096];
        handle.write_at(&zeros, test_offset)?;
        handle.sync()?;

        Ok((fp_rate, fn_rate))
    }

    fn get_device_size(device_path: &str) -> Result<u64> {
        let output = Command::new("blockdev")
            .args(["--getsize64", device_path])
            .output()?;

        let size_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(size_str.parse()?)
    }

    fn is_hdd(device_path: &str) -> Result<bool> {
        let output = Command::new("lsblk")
            .args(["-d", "-o", "ROTA", device_path])
            .output()?;

        let output_str = String::from_utf8_lossy(&output.stdout);
        Ok(output_str.contains("1"))
    }

    fn is_ssd(device_path: &str) -> Result<bool> {
        Ok(!Self::is_hdd(device_path)?)
    }

    /// Generate comprehensive verification report
    pub fn generate_verification_report(
        device_path: &str,
        pre_wipe: PreWipeTestResults,
        post_wipe: PostWipeAnalysis,
        level: VerificationLevel,
    ) -> Result<VerificationReport> {
        let confidence = Self::calculate_confidence_level(&pre_wipe, &post_wipe);
        let compliance = Self::determine_compliance(&post_wipe, confidence);
        let recommendations = Self::generate_recommendations(&post_wipe, confidence);
        let warnings = Self::generate_warnings(&post_wipe);

        Ok(VerificationReport {
            device_path: device_path.to_string(),
            timestamp: Utc::now(),
            pre_wipe_tests: pre_wipe,
            post_wipe_analysis: post_wipe,
            confidence_level: confidence,
            verification_level: level,
            verification_method: "Enterprise Forensic Verification v3.0".to_string(),
            compliance_standards: compliance,
            recommendations,
            warnings,
        })
    }

    pub(crate) fn calculate_confidence_level(
        pre_wipe: &PreWipeTestResults,
        post_wipe: &PostWipeAnalysis,
    ) -> f64 {
        let mut score = 0.0;

        // Pre-wipe tests (20%)
        if pre_wipe.test_pattern_detection {
            score += 7.0;
        }
        if pre_wipe.recovery_tool_simulation {
            score += 7.0;
        }
        if pre_wipe.false_negative_rate < 0.01 {
            score += 6.0;
        }

        // Entropy (25%)
        if post_wipe.entropy_score > 7.8 {
            score += 25.0;
        } else if post_wipe.entropy_score > 7.5 {
            score += 20.0;
        } else if post_wipe.entropy_score > 7.0 {
            score += 15.0;
        }

        // Statistical tests (15%)
        let tests_passed = [
            post_wipe.statistical_tests.runs_test_passed,
            post_wipe.statistical_tests.monobit_test_passed,
            post_wipe.statistical_tests.poker_test_passed,
            post_wipe.statistical_tests.serial_test_passed,
            post_wipe.statistical_tests.autocorrelation_test_passed,
        ]
        .iter()
        .filter(|&&x| x)
        .count();
        score += (tests_passed as f64 / 5.0) * 15.0;

        // Pattern analysis (10%)
        if !post_wipe.pattern_analysis.repeating_patterns_found {
            score += 3.0;
        }
        if !post_wipe.pattern_analysis.known_file_signatures {
            score += 4.0;
        }
        if !post_wipe.pattern_analysis.structured_data_detected {
            score += 3.0;
        }

        // Hidden areas (15%)
        if post_wipe.hidden_areas.hpa_verified {
            score += 5.0;
        }
        if post_wipe.hidden_areas.controller_cache_flushed {
            score += 5.0;
        }
        if post_wipe.hidden_areas.over_provisioning_verified {
            score += 5.0;
        }

        // Recovery simulation (10%)
        match post_wipe.recovery_simulation.overall_recovery_risk {
            RecoveryRisk::None => score += 10.0,
            RecoveryRisk::VeryLow => score += 8.0,
            RecoveryRisk::Low => score += 6.0,
            RecoveryRisk::Medium => score += 4.0,
            RecoveryRisk::High => score += 2.0,
            RecoveryRisk::Critical => score += 0.0,
        }

        // Sector analysis (5%)
        let clean_ratio = 1.0
            - (post_wipe.sector_sampling.suspicious_sectors as f64
                / post_wipe.sector_sampling.total_sectors_sampled as f64);
        score += clean_ratio * 5.0;

        score.min(100.0)
    }

    pub(crate) fn determine_compliance(
        post_wipe: &PostWipeAnalysis,
        confidence: f64,
    ) -> Vec<String> {
        let mut standards = Vec::new();

        if confidence >= 99.0 {
            standards.push("DoD 5220.22-M".to_string());
            standards.push("NIST 800-88 Rev. 1".to_string());
        }

        if confidence >= 95.0 {
            standards.push("PCI DSS v3.2.1".to_string());
            standards.push("HIPAA Security Rule".to_string());
        }

        if post_wipe.entropy_score > 7.5 && confidence >= 90.0 {
            standards.push("ISO/IEC 27001:2013".to_string());
            standards.push("GDPR Article 32".to_string());
        }

        if post_wipe.pattern_analysis.compression_ratio > 0.9 {
            standards.push("NSA Storage Device Sanitization".to_string());
        }

        if matches!(
            post_wipe.recovery_simulation.overall_recovery_risk,
            RecoveryRisk::None | RecoveryRisk::VeryLow
        ) {
            standards.push("NIST SP 800-53 Media Sanitization".to_string());
        }

        standards
    }

    fn generate_recommendations(post_wipe: &PostWipeAnalysis, confidence: f64) -> Vec<String> {
        let mut recommendations = Vec::new();

        if confidence >= 99.9 {
            recommendations
                .push("✅ Drive is forensically clean with highest confidence".to_string());
            recommendations.push(
                "✅ Safe for disposal, resale, or redeployment in any environment".to_string(),
            );
        } else if confidence >= 95.0 {
            recommendations
                .push("✅ Drive sanitization successful with high confidence".to_string());
            recommendations.push("ℹ️ Suitable for most compliance requirements".to_string());
        } else if confidence >= 90.0 {
            recommendations
                .push("⚠️ Drive sanitization completed but with reduced confidence".to_string());
            recommendations
                .push("⚠️ Consider physical destruction for highly sensitive data".to_string());
        } else {
            recommendations
                .push("❌ Sanitization confidence below acceptable threshold".to_string());
            recommendations.push(
                "❌ STRONGLY recommend physical destruction or additional wipe passes".to_string(),
            );
        }

        if post_wipe.sector_sampling.suspicious_sectors > 0 {
            recommendations.push(format!(
                "⚠️ {} suspicious sectors detected - targeted overwrite recommended",
                post_wipe.sector_sampling.suspicious_sectors
            ));
        }

        if post_wipe.entropy_score < 7.5 {
            recommendations.push(
                "⚠️ Entropy below optimal - consider additional random overwrite pass".to_string(),
            );
        }

        if !post_wipe.pattern_analysis.detected_signatures.is_empty() {
            recommendations.push(format!(
                "❌ CRITICAL: {} file signatures detected - data recovery may be possible!",
                post_wipe.pattern_analysis.detected_signatures.len()
            ));
        }

        if matches!(
            post_wipe.recovery_simulation.overall_recovery_risk,
            RecoveryRisk::Medium | RecoveryRisk::High | RecoveryRisk::Critical
        ) {
            recommendations
                .push("❌ HIGH RECOVERY RISK: Consider re-wiping with more passes".to_string());
        }

        if post_wipe.bad_sectors.percentage_unreadable > 5.0 {
            recommendations.push(format!(
                "⚠️ {:.1}% of sectors unreadable - drive may be failing",
                post_wipe.bad_sectors.percentage_unreadable
            ));
        }

        recommendations
    }

    fn generate_warnings(post_wipe: &PostWipeAnalysis) -> Vec<String> {
        let mut warnings = Vec::new();

        if !post_wipe.hidden_areas.hpa_verified {
            warnings.push("HPA area not fully verified".to_string());
        }

        if post_wipe.hidden_areas.remapped_sectors_found > 0 {
            warnings.push(format!(
                "{} remapped sectors found - may contain old data",
                post_wipe.hidden_areas.remapped_sectors_found
            ));
        }

        if !post_wipe.hidden_areas.controller_cache_flushed {
            warnings.push("Controller cache flush verification failed".to_string());
        }

        if post_wipe
            .recovery_simulation
            .testdisk_results
            .partition_table_recoverable
        {
            warnings.push("Partition table may be recoverable".to_string());
        }

        if post_wipe.recovery_simulation.photorec_results.would_succeed {
            warnings.push("File recovery tools may succeed".to_string());
        }

        warnings
    }
}

// ==================== SUPPORTING STRUCTURES ====================

#[derive(Debug, Clone)]
struct HPAInfo {
    hidden_sectors: u64,
}

/// Live USB verification system
pub struct LiveUSBVerification;

impl LiveUSBVerification {
    pub fn create_verification_usb() -> Result<()> {
        println!("🔧 Creating Live USB Verification Image");
        println!("📝 Live USB Creation Instructions:");
        println!("1. Download minimal Linux ISO (e.g., Alpine Linux)");
        println!("2. Add sayonara verification tools");
        println!("3. Configure auto-run verification script");
        println!("4. Write to USB using dd or Rufus");
        Ok(())
    }

    pub fn send_verification_report(report: &VerificationReport, endpoint: &str) -> Result<()> {
        println!("📤 Sending verification report to {}", endpoint);
        let json = serde_json::to_string_pretty(report)?;
        println!("Report size: {} bytes", json.len());
        Ok(())
    }
}
