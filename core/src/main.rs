use anyhow::Result;
use clap::{Parser, Subcommand};
use sayonara_wipe::algorithms::{dod::DoDWipe, gutmann::GutmannWipe, random::RandomWipe};
use sayonara_wipe::crypto::certificates::{CertificateGenerator, VerificationResult, WipeDetails};
use sayonara_wipe::drives::{
    DriveDetector, FreezeMitigation, HDDWipe, HPADCOManager, NVMeWipe, SEDManager, SMARTMonitor,
    SSDWipe, TrimOperations,
};
use sayonara_wipe::verification::recovery_test::RecoveryTest;
use sayonara_wipe::verification::{
    EnhancedVerification, LiveUSBVerification, PostWipeAnalysis, PreWipeTestResults,
    VerificationLevel, VerificationReport,
};
use sayonara_wipe::*;
use std::io::{self, Write};
use std::time::{Duration, Instant};
use uuid::Uuid;

#[derive(Parser)]
#[command(name = "sayonara-wipe")]
#[command(about = "Advanced secure data wiping tool with comprehensive hardware support")]
#[command(version = "1.0.0")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Enable debug logging
    #[arg(long, global = true)]
    debug: bool,

    /// Disable safety checks (DANGEROUS!)
    #[arg(long, global = true)]
    unsafe_mode: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// List all detected drives with capabilities
    List {
        /// Show detailed capabilities
        #[arg(short, long)]
        detailed: bool,

        /// Include system drives
        #[arg(long)]
        include_system: bool,
    },

    /// Wipe a specific drive
    Wipe {
        /// Device path (e.g., /dev/sda)
        device: String,

        /// Wiping algorithm (dod, gutmann, random, zero, secure, crypto, sanitize, trim, auto)
        #[arg(short, long, default_value = "auto")]
        algorithm: String,

        /// Skip verification
        #[arg(long)]
        no_verify: bool,

        /// Output certificate path
        #[arg(short, long)]
        cert_output: Option<String>,

        /// Handle HPA/DCO (ignore, detect, remove-temp, remove-perm)
        #[arg(long, default_value = "detect")]
        hpa_dco: String,

        /// Skip TRIM after wipe
        #[arg(long)]
        no_trim: bool,

        /// Skip temperature monitoring
        #[arg(long)]
        no_temp_check: bool,

        /// Maximum temperature in Celsius
        #[arg(long, default_value = "65")]
        max_temp: u32,

        /// Skip freeze mitigation
        #[arg(long)]
        no_unfreeze: bool,

        /// Force operation even if drive is unhealthy
        #[arg(long)]
        force: bool,
    },

    /// Wipe ALL drives (EXTREMELY DANGEROUS!)
    WipeAll {
        /// Wiping algorithm
        #[arg(short, long, default_value = "auto")]
        algorithm: String,

        /// Skip verification
        #[arg(long)]
        no_verify: bool,

        /// Output directory for certificates
        #[arg(short, long, default_value = "./certificates")]
        cert_dir: String,

        /// Exclude specific drives (comma-separated)
        #[arg(long)]
        exclude: Option<String>,

        /// Handle HPA/DCO
        #[arg(long, default_value = "detect")]
        hpa_dco: String,

        /// Skip TRIM after wipe
        #[arg(long)]
        no_trim: bool,

        /// Force operation even if drives are unhealthy
        #[arg(long)]
        force: bool,
    },

    /// Verify a previous wipe
    Verify {
        /// Device path to verify
        device: String,

        /// Check for hidden areas
        #[arg(long)]
        check_hidden: bool,
    },

    /// Check drive health and capabilities
    Health {
        /// Device path (or "all" for all drives)
        device: String,

        /// Run SMART self-test
        #[arg(long)]
        self_test: bool,

        /// Monitor temperature continuously
        #[arg(long)]
        monitor: bool,
    },

    /// Manage self-encrypting drives
    Sed {
        /// Device path
        device: String,

        #[command(subcommand)]
        action: SedAction,
    },

    /// Enhanced wipe with mathematical verification (RECOMMENDED)
    EnhancedWipe {
        /// Device path (e.g., /dev/sda)
        device: String,

        /// Wiping algorithm (dod, gutmann, random, zero, secure, crypto, sanitize, trim, auto)
        #[arg(short, long, default_value = "auto")]
        algorithm: String,

        /// Output certificate path
        #[arg(short, long)]
        cert_output: Option<String>,

        /// Sample percentage for verification (0.1-10.0)
        #[arg(long, default_value = "1.0")]
        sample_percent: f64,

        /// Skip pre-wipe tests (not recommended)
        #[arg(long)]
        skip_pre_tests: bool,

        /// Required confidence level (90-100)
        #[arg(long, default_value = "95.0")]
        min_confidence: f64,

        /// NEW: Verification level (level1, level2, level3, level4)
        #[arg(long, default_value = "level1")]
        verification_level: String,

        /// Handle HPA/DCO (ignore, detect, remove-temp, remove-perm)
        #[arg(long, default_value = "detect")]
        hpa_dco: String,

        /// Skip TRIM after wipe
        #[arg(long)]
        no_trim: bool,

        /// Force operation even if drive is unhealthy
        #[arg(long)]
        force: bool,
    },

    /// Create Live USB for external verification
    CreateVerificationUSB {
        /// Output path for USB image
        output: String,
    },

    /// Verify from Live USB environment and report results
    LiveVerify {
        /// Device to verify
        device: String,

        /// Remote endpoint for reporting
        #[arg(long)]
        report_to: Option<String>,

        /// Sample percentage (0.1-10.0)
        #[arg(long, default_value = "1.0")]
        sample_percent: f64,
    },

    Custom,
}

#[derive(Subcommand)]
enum SedAction {
    /// Check SED status
    Status,

    /// Perform cryptographic erase
    CryptoErase {
        /// Password for locked drives
        #[arg(long)]
        password: Option<String>,
    },

    /// Unlock drive
    Unlock {
        /// Password
        password: String,
    },
}

/// Generate enhanced certificate with verification details
fn generate_enhanced_certificate(
    drive_info: &DriveInfo,
    config: &WipeConfig,
    verification_report: &VerificationReport,
    duration: Duration,
    cert_path: &str,
) -> Result<()> {
    use crate::crypto::certificates::{CertificateGenerator, VerificationResult, WipeDetails};

    let cert_gen = CertificateGenerator::new();

    // Create enhanced wipe details
    let wipe_details = WipeDetails {
        algorithm_used: format!("{:?}", config.algorithm),
        passes_completed: 1,
        duration_seconds: duration.as_secs(),
        operator_id: None,
    };

    // Create enhanced verification result
    let verification_result = VerificationResult {
        verified: verification_report.confidence_level >= 95.0,
        entropy_score: verification_report.post_wipe_analysis.entropy_score,
        recovery_test_passed: verification_report.confidence_level >= 99.0,
        verification_timestamp: verification_report.timestamp,
    };

    let certificate =
        cert_gen.generate_certificate(drive_info, wipe_details, verification_result)?;

    // Add enhanced verification data to certificate
    let mut enhanced_cert = serde_json::to_value(&certificate)?;
    enhanced_cert["enhanced_verification"] = serde_json::to_value(&verification_report)?;

    // Save enhanced certificate
    let cert_json = serde_json::to_string_pretty(&enhanced_cert)?;
    std::fs::write(cert_path, cert_json)?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    setup_signal_handlers()?;

    // Check for root privileges
    if !cli.unsafe_mode && !is_root() {
        eprintln!("Error: This program requires root privileges.");
        eprintln!("Please run with sudo or as root user.");
        std::process::exit(1);
    }

    // Set up logging
    if cli.debug {
        env_logger::init();
    }

    match &cli.command {
        Commands::List {
            detailed,
            include_system,
        } => {
            list_drives(*detailed, *include_system).await?;
        }
        Commands::Wipe {
            device,
            algorithm,
            no_verify,
            cert_output,
            hpa_dco,
            no_trim,
            no_temp_check,
            max_temp,
            no_unfreeze,
            force,
        } => {
            let config = build_wipe_config(
                algorithm,
                !no_verify,
                hpa_dco,
                !no_trim,
                !no_temp_check,
                *max_temp,
                !no_unfreeze,
            )?;
            wipe_drive(
                device,
                config,
                cert_output.as_deref(),
                *force,
                cli.unsafe_mode,
            )
            .await?;
        }
        Commands::WipeAll {
            algorithm,
            no_verify,
            cert_dir,
            exclude,
            hpa_dco,
            no_trim,
            force,
        } => {
            let config =
                build_wipe_config(algorithm, !no_verify, hpa_dco, !no_trim, true, 65, true)?;
            wipe_all_drives(
                config,
                cert_dir,
                exclude.as_deref(),
                cli.unsafe_mode,
                *force,
            )
            .await?;
        }
        Commands::Verify {
            device,
            check_hidden,
        } => {
            verify_drive(device, *check_hidden).await?;
        }
        Commands::Health {
            device,
            self_test,
            monitor,
        } => {
            check_health(device, *self_test, *monitor).await?;
        }
        Commands::Sed { device, action } => {
            handle_sed(device, action).await?;
        }
        Commands::EnhancedWipe {
            device,
            algorithm,
            cert_output,
            sample_percent,
            skip_pre_tests,
            min_confidence,
            verification_level,
            hpa_dco,
            no_trim,
            force,
        } => {
            let drives = DriveDetector::detect_all_drives()?;
            let drive_info = drives
                .into_iter()
                .find(|d| d.device_path == *device)
                .ok_or_else(|| anyhow::anyhow!("Drive not found: {}", device))?;

            // Safety checks
            if !cli.unsafe_mode {
                // Check if mounted
                if let Ok(is_mounted) = DriveDetector::is_mounted(device) {
                    if is_mounted {
                        eprintln!("Error: {} is currently mounted.", device);
                        eprintln!("Please unmount before wiping.");
                        return Ok(());
                    }
                }

                // Health check
                if !force {
                    if let Some(health) = &drive_info.health_status {
                        if *health == HealthStatus::Failed || *health == HealthStatus::Critical {
                            eprintln!("Error: Drive health is {:?}", health);
                            eprintln!("Use --force to override.");
                            return Ok(());
                        }
                    }
                }
            }

            // Parse verification level
            let level = match verification_level.to_lowercase().as_str() {
                "level1" | "1" => VerificationLevel::Level1RandomSampling,
                "level2" | "2" => VerificationLevel::Level2SystematicSampling,
                "level3" | "3" => VerificationLevel::Level3FullScan,
                "level4" | "4" => VerificationLevel::Level4ForensicScan,
                _ => {
                    eprintln!("Invalid verification level. Using Level 1 (Random Sampling)");
                    VerificationLevel::Level1RandomSampling
                }
            };

            // Build config
            let config = build_wipe_config(
                algorithm, true, // Always verify in enhanced mode
                hpa_dco, !no_trim, true, // Temperature monitoring
                65, true, // Freeze mitigation
            )?;

            // Safety confirmation with level info
            if !cli.unsafe_mode {
                println!("\n⚠️  WARNING: Enhanced Secure Wipe with Forensic Verification");
                println!("This will PERMANENTLY DESTROY all data on:");
                println!("  Device: {}", device);
                println!("  Model: {}", drive_info.model);
                println!("  Serial: {}", drive_info.serial);
                println!("  Size: {} GB", drive_info.size / (1024 * 1024 * 1024));
                println!("\nVerification Parameters:");
                println!("  Level: {:?}", level);
                println!("  Required confidence: {}%", min_confidence);

                // Show estimated time
                match level {
                    VerificationLevel::Level1RandomSampling => {
                        println!("  Estimated time: 1-5 minutes");
                    }
                    VerificationLevel::Level2SystematicSampling => {
                        println!("  Estimated time: 5-30 minutes");
                    }
                    VerificationLevel::Level3FullScan => {
                        println!("  Estimated time: 1-4 hours");
                    }
                    VerificationLevel::Level4ForensicScan => {
                        println!("  Estimated time: 2-8 hours");
                    }
                }

                print!("\nType 'DESTROY' to confirm: ");
                io::stdout().flush()?;

                let mut input = String::new();
                io::stdin().read_line(&mut input)?;

                if input.trim() != "DESTROY" {
                    println!("Operation cancelled");
                    return Ok(());
                }
            }

            // Execute enhanced wipe with selected level
            enhanced_wipe_with_verification(
                device,
                &drive_info,
                config,
                cert_output.as_deref(),
                *sample_percent, // IMPORTANT: Pass sample_percent
                *min_confidence,
                level,
                *skip_pre_tests, // IMPORTANT: Pass skip_pre_tests
            )
            .await?;
        }

        Commands::CreateVerificationUSB { output: _ } => {
            println!("🔧 Creating Live USB Verification Image");
            LiveUSBVerification::create_verification_usb()?;
            println!("✅ Instructions for USB creation have been generated");
        }

        Commands::LiveVerify {
            device,
            report_to,
            sample_percent: _,
        } => {
            // This would be run from the Live USB environment
            println!("🔍 Live Verification Mode");
            println!("Device: {}", device);

            // Get device size
            let device_size = {
                use std::process::Command;
                let output = Command::new("blockdev")
                    .args(["--getsize64", device])
                    .output()?;
                let size_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
                size_str.parse::<u64>()?
            };

            // Run verification
            println!("Running pre-wipe capability tests...");
            let pre_wipe = EnhancedVerification::pre_wipe_capability_test(device, 1024 * 1024)?;

            println!("Running post-wipe mathematical verification...");
            let post_wipe = EnhancedVerification::post_wipe_verification_with_level(
                device,
                device_size,
                VerificationLevel::Level1RandomSampling, // CORRECTED: Use the right function name
            )?;

            let report = EnhancedVerification::generate_verification_report(
                device,
                pre_wipe,
                post_wipe,
                VerificationLevel::Level1RandomSampling, // IMPORTANT: Add the level parameter
            )?;

            // Display results
            display_enhanced_verification_summary(&report);

            // Send to remote if configured
            if let Some(endpoint) = report_to {
                println!("📤 Sending report to {}...", endpoint);
                LiveUSBVerification::send_verification_report(&report, endpoint)?;
                println!("✅ Report sent successfully");
            }

            // Save local copy
            let local_report = format!(
                "verification_report_{}.json",
                chrono::Utc::now().format("%Y%m%d_%H%M%S")
            );
            let json = serde_json::to_string_pretty(&report)?;
            std::fs::write(&local_report, json)?;
            println!("📁 Report saved to: {}", local_report);
        }

        Commands::Custom => {
            print_customizations()?;
        }
    }

    Ok(())
}

/// Enhanced wipe with multi-level verification
async fn enhanced_wipe_with_verification(
    device: &str,
    drive_info: &DriveInfo,
    config: WipeConfig,
    cert_output: Option<&str>,
    _sample_percent: f64,                  // PARAMETER 5
    min_confidence: f64,                   // PARAMETER 6
    verification_level: VerificationLevel, // PARAMETER 7
    skip_pre_tests: bool,                  // PARAMETER 8
) -> Result<()> {
    println!("\n🚀 Starting Enhanced Secure Wipe with Forensic Verification");
    println!(
        "Device: {} ({} GB)",
        device,
        drive_info.size / (1024 * 1024 * 1024)
    );
    println!("Verification Level: {:?}", verification_level);
    println!("{}", "=".repeat(70));

    let start_time = Instant::now();

    // ===== STAGE 1: PRE-WIPE VERIFICATION CAPABILITY TEST =====
    let pre_wipe_results = if !skip_pre_tests {
        println!("\n📋 Stage 1: Pre-Wipe Verification Testing");
        println!("Testing our ability to detect data patterns...\n");

        let results = EnhancedVerification::pre_wipe_capability_test(
            device,
            1024 * 1024, // Use 1MB test area
        )?;

        // Display pre-wipe test results
        println!("✅ Verification System Test Results:");
        println!(
            "  ├─ Pattern Detection: {}",
            if results.test_pattern_detection {
                "✓ PASSED"
            } else {
                "✗ FAILED"
            }
        );
        println!(
            "  ├─ Recovery Tool Simulation: {}",
            if results.recovery_tool_simulation {
                "✓ PASSED"
            } else {
                "✗ FAILED"
            }
        );
        println!(
            "  ├─ Sensitivity Calibration: {:.1}%",
            results.sensitivity_calibration
        );
        println!(
            "  ├─ False Positive Rate: {:.2}%",
            results.false_positive_rate * 100.0
        );
        println!(
            "  └─ False Negative Rate: {:.2}%",
            results.false_negative_rate * 100.0
        );

        if !results.test_pattern_detection || !results.recovery_tool_simulation {
            eprintln!("\n⚠️  Warning: Verification system tests failed!");
            print!("Do you want to continue anyway? [y/N]: ");
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;

            if input.trim().to_lowercase() != "y" {
                return Err(anyhow::anyhow!("Operation cancelled by user"));
            }
        }

        results
    } else {
        println!("\n⚠️  Skipping pre-wipe tests (--skip-pre-tests enabled)");
        PreWipeTestResults {
            test_pattern_detection: true,
            recovery_tool_simulation: true,
            sensitivity_calibration: 95.0,
            false_positive_rate: 0.01,
            false_negative_rate: 0.01,
        }
    };

    // ===== STAGE 2: COMPLETE DATA WIPE =====
    println!("\n🔥 Stage 2: Complete Data Destruction");
    println!("Algorithm: {:?}", config.algorithm);

    // Execute the wipe
    println!("  └─ Executing wipe algorithm...");
    select_and_execute_wipe(device, drive_info, &config).await?;

    let wipe_duration = start_time.elapsed();
    println!(
        "✅ Wipe completed in {:.2} seconds",
        wipe_duration.as_secs_f64()
    );

    // ===== STAGE 3: MULTI-LEVEL VERIFICATION =====
    println!("\n🔬 Stage 3: Multi-Level Forensic Verification");
    println!("Level: {:?}", verification_level);

    // Display level-specific information
    match verification_level {
        VerificationLevel::Level1RandomSampling => {
            println!("⏱️  Estimated time: 1-5 minutes");
            println!("📊 Coverage: ~1% random sampling");
        }
        VerificationLevel::Level2SystematicSampling => {
            println!("⏱️  Estimated time: 5-30 minutes");
            println!("📊 Coverage: Systematic sampling (every 100th sector)");
        }
        VerificationLevel::Level3FullScan => {
            println!("⏱️  Estimated time: 1-4 hours (depends on drive size)");
            println!("📊 Coverage: 100% of accessible drive");
        }
        VerificationLevel::Level4ForensicScan => {
            println!("⏱️  Estimated time: 2-8 hours (comprehensive forensic analysis)");
            println!("📊 Coverage: 100% + hidden areas + MFM simulation");
        }
    }

    println!("\nAnalyzing wiped drive for data remnants...\n");

    let post_wipe_analysis = EnhancedVerification::post_wipe_verification_with_level(
        device,
        drive_info.size,
        verification_level,
    )?;

    // Display post-wipe analysis
    display_enhanced_post_wipe_analysis(&post_wipe_analysis);

    // ===== STAGE 4: CONFIDENCE CALCULATION & REPORT =====
    println!("\n📊 Stage 4: Generating Verification Report");

    let verification_report = EnhancedVerification::generate_verification_report(
        device,
        pre_wipe_results,
        post_wipe_analysis,
        verification_level, // IMPORTANT: Pass the level here
    )?;

    display_enhanced_verification_summary(&verification_report);

    // Check if confidence requirement was met
    if verification_report.confidence_level < min_confidence {
        eprintln!(
            "\n❌ Confidence level {:.1}% is below required {:.1}%",
            verification_report.confidence_level, min_confidence
        );

        // Show recovery risk
        println!("\n⚠️  Recovery Risk Assessment:");
        println!(
            "  Overall Risk: {:?}",
            verification_report
                .post_wipe_analysis
                .recovery_simulation
                .overall_recovery_risk
        );

        if !verification_report
            .post_wipe_analysis
            .pattern_analysis
            .detected_signatures
            .is_empty()
        {
            println!("  ❌ CRITICAL: File signatures detected!");
            println!("  Detected signatures:");
            for sig in &verification_report
                .post_wipe_analysis
                .pattern_analysis
                .detected_signatures
            {
                println!(
                    "    • {} (confidence: {:.0}%)",
                    sig.signature_name,
                    sig.confidence * 100.0
                );
            }
        }

        return Err(anyhow::anyhow!(
            "Verification confidence below required threshold"
        ));
    }

    // ===== STAGE 5: CERTIFICATE GENERATION =====
    if let Some(cert_path) = cert_output {
        println!("\n🏆 Stage 5: Generating Enhanced Certificate");
        generate_enhanced_certificate(
            drive_info,
            &config,
            &verification_report,
            wipe_duration,
            cert_path,
        )?;
        println!("✅ Certificate saved to: {}", cert_path);
    }

    // ===== STAGE 6: HEAT MAP VISUALIZATION =====
    if let Some(ref heat_map) = verification_report.post_wipe_analysis.heat_map {
        println!("\n🗺️  Stage 6: Entropy Heat Map");
        let ascii_map = EnhancedVerification::render_heat_map_ascii(heat_map);
        println!("{}", ascii_map);

        if !heat_map.suspicious_blocks.is_empty() {
            println!(
                "⚠️  {} suspicious blocks detected at low entropy",
                heat_map.suspicious_blocks.len()
            );
        }
    }

    // ===== STAGE 7: POST-WIPE OPERATIONS =====
    if config.use_trim_after && drive_info.capabilities.trim_support {
        println!("\n🧹 Stage 7: Post-Wipe TRIM");
        TrimOperations::secure_trim_with_verify(device)?;
    }

    // ===== FINAL SUMMARY =====
    println!("\n{}", "=".repeat(70));
    println!("🎉 FORENSIC VERIFICATION COMPLETE");
    println!("{}", "=".repeat(70));
    println!(
        "📊 Confidence Level: {:.1}%",
        verification_report.confidence_level
    );
    println!("🔒 Verification Level: {:?}", verification_level);

    println!("\n✅ Compliance Standards Met:");
    for standard in &verification_report.compliance_standards {
        println!("   • {}", standard);
    }

    if !verification_report.warnings.is_empty() {
        println!("\n⚠️  Warnings:");
        for warning in &verification_report.warnings {
            println!("   • {}", warning);
        }
    }

    println!("\n📝 Recommendations:");
    for recommendation in &verification_report.recommendations {
        println!("   {}", recommendation);
    }

    println!(
        "\n⏱️  Total Time: {:.2} seconds",
        start_time.elapsed().as_secs_f64()
    );

    Ok(())
}

/// Display enhanced post-wipe analysis results
fn display_enhanced_post_wipe_analysis(analysis: &PostWipeAnalysis) {
    println!("📈 Analysis Results:");

    // Entropy Score
    let entropy_icon = if analysis.entropy_score > 7.8 {
        "✅"
    } else if analysis.entropy_score > 7.5 {
        "⚠️"
    } else {
        "❌"
    };
    println!(
        "  ├─ {} Entropy Score: {:.4}/8.0",
        entropy_icon, analysis.entropy_score
    );

    // Chi-square test
    let chi_icon = if analysis.chi_square_test < 300.0 {
        "✅"
    } else {
        "⚠️"
    };
    println!(
        "  ├─ {} Chi-Square Test: {:.2}",
        chi_icon, analysis.chi_square_test
    );

    // Pattern Analysis
    println!("  ├─ Pattern Analysis:");
    println!(
        "  │  ├─ Repeating Patterns: {}",
        if analysis.pattern_analysis.repeating_patterns_found {
            "❌ FOUND"
        } else {
            "✅ None"
        }
    );
    println!(
        "  │  ├─ File Signatures: {}",
        if analysis.pattern_analysis.known_file_signatures {
            "❌ FOUND"
        } else {
            "✅ None"
        }
    );

    if !analysis.pattern_analysis.detected_signatures.is_empty() {
        println!("  │  │  Detected signatures:");
        for sig in &analysis.pattern_analysis.detected_signatures {
            println!(
                "  │  │    • {} at offset {}",
                sig.signature_name, sig.offset
            );
        }
    }

    println!(
        "  │  └─ Structured Data: {}",
        if analysis.pattern_analysis.structured_data_detected {
            "❌ FOUND"
        } else {
            "✅ None"
        }
    );

    // Statistical Tests
    let tests_passed = [
        analysis.statistical_tests.runs_test_passed,
        analysis.statistical_tests.monobit_test_passed,
        analysis.statistical_tests.poker_test_passed,
        analysis.statistical_tests.serial_test_passed,
        analysis.statistical_tests.autocorrelation_test_passed,
    ]
    .iter()
    .filter(|&&x| x)
    .count();

    println!("  ├─ Statistical Tests: {}/5 passed", tests_passed);

    // Hidden Areas
    println!("  ├─ Hidden Area Verification:");
    println!(
        "  │  ├─ HPA: {}",
        if analysis.hidden_areas.hpa_verified {
            "✅ Verified"
        } else {
            "⚠️ Failed"
        }
    );
    if let Some(entropy) = analysis.hidden_areas.hpa_entropy {
        println!("  │  │  Entropy: {:.2}", entropy);
    }
    println!(
        "  │  ├─ DCO: {}",
        if analysis.hidden_areas.dco_verified {
            "✅ Verified"
        } else {
            "⚠️ Failed"
        }
    );
    println!(
        "  │  ├─ Remapped Sectors: {}/{}",
        analysis.hidden_areas.remapped_sectors_verified,
        analysis.hidden_areas.remapped_sectors_found
    );
    println!(
        "  │  ├─ Controller Cache: {}",
        if analysis.hidden_areas.controller_cache_flushed {
            "✅ Flushed"
        } else {
            "⚠️ Not Verified"
        }
    );
    println!(
        "  │  └─ Over-Provisioning: {}",
        if analysis.hidden_areas.over_provisioning_verified {
            "✅ Verified"
        } else {
            "⚠️ N/A"
        }
    );

    // Recovery Simulation
    println!("  ├─ Recovery Tool Simulation:");
    println!(
        "  │  ├─ PhotoRec: {} (scanned {} signatures, found {})",
        if analysis.recovery_simulation.photorec_results.would_succeed {
            "❌ Would succeed"
        } else {
            "✅ Would fail"
        },
        analysis
            .recovery_simulation
            .photorec_results
            .signatures_scanned,
        analysis
            .recovery_simulation
            .photorec_results
            .signatures_found
            .len()
    );
    println!(
        "  │  ├─ TestDisk: {}",
        if analysis.recovery_simulation.testdisk_results.would_succeed {
            "❌ Would succeed"
        } else {
            "✅ Would fail"
        }
    );
    println!(
        "  │  └─ Overall Risk: {:?}",
        analysis.recovery_simulation.overall_recovery_risk
    );

    // MFM Simulation (if performed)
    if let Some(ref mfm) = analysis.recovery_simulation.mfm_simulation {
        println!("  ├─ MFM Analysis (HDD):");
        println!(
            "  │  ├─ Theoretical Recovery: {}",
            if mfm.theoretical_recovery_possible {
                "❌ Possible"
            } else {
                "✅ Impossible"
            }
        );
        println!("  │  └─ Confidence: {:.1}%", mfm.confidence_level);
    }

    // Bad Sectors
    if analysis.bad_sectors.unreadable_count > 0 {
        println!("  ├─ Bad Sectors:");
        println!(
            "  │  ├─ Unreadable: {}",
            analysis.bad_sectors.unreadable_count
        );
        println!(
            "  │  └─ Percentage: {:.2}%",
            analysis.bad_sectors.percentage_unreadable
        );
    }

    // Sector Sampling
    println!("  └─ Sector Analysis:");
    println!(
        "     ├─ Sectors Sampled: {}",
        analysis.sector_sampling.total_sectors_sampled
    );
    println!(
        "     ├─ Suspicious Sectors: {}",
        analysis.sector_sampling.suspicious_sectors
    );
    if !analysis.sector_sampling.anomaly_locations.is_empty() {
        println!(
            "     └─ Anomalies at sectors: {} locations",
            analysis.sector_sampling.anomaly_locations.len()
        );
    }
}

/// Display enhanced verification summary
fn display_enhanced_verification_summary(report: &VerificationReport) {
    let confidence_color = if report.confidence_level >= 99.0 {
        "🟢"
    } else if report.confidence_level >= 95.0 {
        "🟡"
    } else {
        "🔴"
    };

    println!("\n📋 Verification Summary:");
    println!(
        "  {} Confidence Level: {:.1}%",
        confidence_color, report.confidence_level
    );
    println!(
        "  ⏰ Timestamp: {}",
        report.timestamp.format("%Y-%m-%d %H:%M:%S UTC")
    );
    println!("  🔧 Method: {}", report.verification_method);
    println!("  📊 Level: {:?}", report.verification_level);
}

fn print_customizations() -> Result<()> {
    println!("\n{}", "=".repeat(80));
    println!("                SAYONARA-WIPE CUSTOMIZATION OPTIONS");
    println!("{}", "=".repeat(80));

    // ALGORITHMS
    println!("\n📋 WIPING ALGORITHMS");
    println!("{}", "-".repeat(80));
    println!("  dod        - DoD 5220.22-M (3-pass overwrite)");
    println!("  gutmann    - Gutmann method (35-pass overwrite, most thorough)");
    println!("  random     - Single pass with random data");
    println!("  zero       - Single pass with zeros");
    println!("  secure     - Hardware-based secure erase (ATA)");
    println!("  crypto     - Cryptographic erase (for Self-Encrypting Drives)");
    println!("  sanitize   - NVMe sanitize command");
    println!("  trim       - TRIM/discard only (for SSDs)");
    println!("  auto       - Automatically select best algorithm (default)");

    // HPA/DCO HANDLING
    println!("\n🔒 HPA/DCO (Hidden Protected Area / Device Configuration Overlay) HANDLING");
    println!("{}", "-".repeat(80));
    println!("  ignore       - Don't check for hidden areas");
    println!("  detect       - Detect and warn about hidden areas (default)");
    println!("  remove-temp  - Temporarily remove during wipe, restore after");
    println!("  remove-perm  - Permanently remove (DANGEROUS - may brick drive)");

    // GLOBAL FLAGS
    println!("\n🌐 GLOBAL FLAGS");
    println!("{}", "-".repeat(80));
    println!("  --debug       - Enable verbose debug logging");
    println!("  --unsafe-mode - Disable all safety checks (EXTREMELY DANGEROUS)");

    // COMMAND-SPECIFIC FLAGS
    println!("\n⚙️  COMMAND-SPECIFIC CUSTOMIZATIONS");
    println!("{}", "-".repeat(80));

    println!("\n  LIST Command:");
    println!("    -d, --detailed      - Show detailed drive capabilities");
    println!("    --include-system    - Include system drives in listing");

    println!("\n  WIPE Command:");
    println!("    -a, --algorithm     - Select wiping algorithm (see above)");
    println!("    --no-verify         - Skip post-wipe verification");
    println!("    -c, --cert-output   - Path for wipe certificate");
    println!("    --hpa-dco          - HPA/DCO handling mode (see above)");
    println!("    --no-trim          - Skip TRIM operation after wipe");
    println!("    --no-temp-check    - Disable temperature monitoring");
    println!("    --max-temp         - Maximum safe temperature in Celsius (default: 65)");
    println!("    --no-unfreeze      - Skip drive freeze mitigation");
    println!("    --force            - Force operation on unhealthy drives");

    println!("\n  WIPE-ALL Command:");
    println!("    -a, --algorithm     - Select wiping algorithm");
    println!("    --no-verify         - Skip verification");
    println!("    -c, --cert-dir      - Directory for certificates (default: ./certificates)");
    println!("    --exclude          - Comma-separated list of drives to exclude");
    println!("    --hpa-dco          - HPA/DCO handling mode");
    println!("    --no-trim          - Skip TRIM operations");
    println!("    --force            - Force operation on unhealthy drives");

    println!("\n  VERIFY Command:");
    println!("    --check-hidden     - Check for hidden areas (HPA/DCO)");

    println!("\n  HEALTH Command:");
    println!("    --self-test        - Run SMART self-test");
    println!("    --monitor          - Continuously monitor temperature");

    println!("\n  SED Command (Self-Encrypting Drives):");
    println!("    status             - Check SED status");
    println!("    crypto-erase       - Perform cryptographic erase");
    println!("      --password       - Password for locked drives");
    println!("    unlock             - Unlock the drive");
    println!("      <password>       - Required password");

    println!("\n  ENHANCED-WIPE Command:");
    println!("    -a, --algorithm       - Select wiping algorithm");
    println!("    -c, --cert-output     - Certificate output path");
    println!("    --sample-percent      - Verification sampling (0.1-10.0, default: 1.0)");
    println!("    --skip-pre-tests      - Skip pre-wipe capability tests (not recommended)");
    println!("    --min-confidence      - Required confidence level (90-100, default: 95.0)");
    println!("    --hpa-dco            - HPA/DCO handling mode");
    println!("    --no-trim            - Skip TRIM operation");
    println!("    --force              - Force operation on unhealthy drives");

    println!("\n  LIVE-VERIFY Command:");
    println!("    --report-to          - Remote endpoint for verification report");
    println!("    --sample-percent     - Verification sampling percentage (default: 1.0)");

    // DRIVE TYPES
    println!("\n💾 SUPPORTED DRIVE TYPES");
    println!("{}", "-".repeat(80));
    println!("  - HDD (Hard Disk Drives)");
    println!("  - SSD (SATA Solid State Drives)");
    println!("  - NVMe (NVMe Solid State Drives)");
    println!("  - USB (External USB drives)");
    println!("  - RAID (RAID controller attached drives)");

    // ENCRYPTION TYPES
    println!("\n🔐 SUPPORTED ENCRYPTION TYPES");
    println!("{}", "-".repeat(80));
    println!("  - OPAL 2.0 / OPAL 1.0");
    println!("  - TCG Enterprise");
    println!("  - ATA Security");
    println!("  - EDrive");
    println!("  - BitLocker");
    println!("  - LUKS");
    println!("  - FileVault");
    println!("  - VeraCrypt");

    // SANITIZE OPTIONS (NVMe)
    println!("\n🧹 NVMe SANITIZE OPTIONS");
    println!("{}", "-".repeat(80));
    println!("  - Block Erase    - Erase at block level");
    println!("  - Crypto Erase   - Cryptographic scramble");
    println!("  - Overwrite      - Overwrite with pattern");
    println!("  - Crypto Scramble - Change encryption key");

    // SAFETY FEATURES
    println!("\n🛡️  BUILT-IN SAFETY FEATURES");
    println!("{}", "-".repeat(80));
    println!("  - System drive detection and protection");
    println!("  - Mounted drive detection");
    println!("  - Drive health monitoring");
    println!("  - Temperature monitoring and throttling");
    println!("  - SMART status checking");
    println!("  - Failure prediction analysis");
    println!("  - Confirmation prompts for destructive operations");
    println!("  - Drive freeze detection and mitigation");

    // VERIFICATION FEATURES
    println!("\n✅ VERIFICATION FEATURES");
    println!("{}", "-".repeat(80));
    println!("  - Post-wipe recovery testing");
    println!("  - Entropy analysis");
    println!("  - Mathematical verification");
    println!("  - Pattern detection");
    println!("  - TRIM effectiveness verification");
    println!("  - Hidden area detection");
    println!("  - Live USB external verification");
    println!("  - Confidence level scoring (0-100%)");

    // CERTIFICATE FEATURES
    println!("\n📜 CERTIFICATE GENERATION");
    println!("{}", "-".repeat(80));
    println!("  - Cryptographically signed certificates");
    println!("  - Timestamp and operator ID tracking");
    println!("  - Algorithm and pass count documentation");
    println!("  - Verification results included");
    println!("  - Drive serial number and model recorded");
    println!("  - Entropy scores and confidence levels");
    println!("  - JSON format for easy parsing");

    // VENDOR-SPECIFIC SUPPORT
    println!("\n🏭 VENDOR-SPECIFIC RAID CONTROLLER SUPPORT");
    println!("{}", "-".repeat(80));
    println!("  - Dell PERC controllers");
    println!("  - HP SmartArray controllers");
    println!("  - LSI MegaRAID controllers");
    println!("  - Adaptec RAID controllers");

    // TEMPERATURE THRESHOLDS
    println!("\n🌡️  TEMPERATURE MANAGEMENT");
    println!("{}", "-".repeat(80));
    println!("  - Default max temperature: 65°C");
    println!("  - Configurable via --max-temp flag");
    println!("  - Automatic pausing when temperature exceeds threshold");
    println!("  - Continuous monitoring during operations");
    println!("  - Warning/Critical threshold detection");

    // FREEZE MITIGATION STRATEGIES
    println!("\n❄️  FREEZE MITIGATION STRATEGIES");
    println!("{}", "-".repeat(80));
    println!("  - Sleep command method");
    println!("  - SATA link reset");
    println!("  - Hot-plug simulation");
    println!("  - Vendor-specific commands");
    println!("  - Power cycle methods");

    // EXAMPLES
    println!("\n📖 USAGE EXAMPLES");
    println!("{}", "-".repeat(80));
    println!("  Basic wipe:");
    println!("    sudo sayonara-wipe wipe /dev/sdb");
    println!();
    println!("  Secure wipe with Gutmann algorithm:");
    println!("    sudo sayonara-wipe wipe /dev/sdb -a gutmann");
    println!();
    println!("  Enhanced wipe with verification:");
    println!("    sudo sayonara-wipe enhanced-wipe /dev/sdb --min-confidence 99.0");
    println!();
    println!("  Wipe with certificate and custom temperature:");
    println!("    sudo sayonara-wipe wipe /dev/sdb -c cert.json --max-temp 70");
    println!();
    println!("  List all drives with details:");
    println!("    sudo sayonara-wipe list --detailed");
    println!();
    println!("  Check drive health:");
    println!("    sudo sayonara-wipe health /dev/sdb --self-test");
    println!();
    println!("  SED cryptographic erase:");
    println!("    sudo sayonara-wipe sed /dev/sdb crypto-erase");

    println!("\n{}", "=".repeat(80));
    println!("For more information, visit: https://github.com/your-repo/sayonara-wipe");
    println!("{}", "=".repeat(80));
    println!();

    Ok(())
}

fn is_root() -> bool {
    unsafe { libc::geteuid() == 0 }
}

fn build_wipe_config(
    algorithm: &str,
    verify: bool,
    hpa_dco: &str,
    use_trim: bool,
    temp_monitoring: bool,
    max_temp: u32,
    freeze_mitigation: bool,
) -> Result<WipeConfig> {
    let algorithm = match algorithm.to_lowercase().as_str() {
        "dod" => Algorithm::DoD5220,
        "gutmann" => Algorithm::Gutmann,
        "random" => Algorithm::Random,
        "zero" => Algorithm::Zero,
        "secure" => Algorithm::SecureErase,
        "crypto" => Algorithm::CryptoErase,
        "sanitize" => Algorithm::Sanitize,
        "trim" => Algorithm::TrimOnly,
        "auto" => Algorithm::SecureErase, // Will fallback based on capabilities
        _ => return Err(anyhow::anyhow!("Unknown algorithm: {}", algorithm)),
    };

    let hpa_dco_handling = match hpa_dco {
        "ignore" => HPADCOHandling::Ignore,
        "detect" => HPADCOHandling::Detect,
        "remove-temp" => HPADCOHandling::TemporaryRemove,
        "remove-perm" => HPADCOHandling::PermanentRemove,
        _ => HPADCOHandling::Detect,
    };

    Ok(WipeConfig {
        algorithm,
        verify,
        multiple_passes: None,
        preserve_partition_table: false,
        unlock_encrypted: false,
        handle_hpa_dco: hpa_dco_handling,
        use_trim_after: use_trim,
        temperature_monitoring: temp_monitoring,
        max_temperature_celsius: Some(max_temp),
        freeze_mitigation,
        sed_crypto_erase: true,
    })
}

async fn list_drives(detailed: bool, include_system: bool) -> Result<()> {
    println!("Detecting drives...");
    let drives = DriveDetector::detect_all_drives()?;

    if drives.is_empty() {
        println!("No drives detected.");
        return Ok(());
    }

    println!("\nDetected drives:");

    let mut filtered_count = 0;
    let mut displayed_count = 0;

    if detailed {
        for drive in drives {
            if !include_system && DriveDetector::is_system_drive(&drive.device_path)? {
                filtered_count += 1;
                continue;
            }

            print_drive_detailed(&drive)?;
            displayed_count += 1;
        }
    } else {
        println!(
            "{:<15} {:<20} {:<15} {:<10} {:<10} {:<10}",
            "Device", "Model", "Serial", "Size", "Type", "Health"
        );
        println!("{}", "-".repeat(90));

        for drive in drives {
            if !include_system && DriveDetector::is_system_drive(&drive.device_path)? {
                filtered_count += 1;
                continue;
            }

            let size_gb = drive.size / (1024 * 1024 * 1024);
            let health = drive
                .health_status
                .map(|h| format!("{:?}", h))
                .unwrap_or_else(|| "Unknown".to_string());

            println!(
                "{:<15} {:<20} {:<15} {:<10} {:<10} {:<10}",
                drive.device_path,
                truncate_string(&drive.model, 20),
                truncate_string(&drive.serial, 15),
                format!("{}GB", size_gb),
                format!("{:?}", drive.drive_type),
                health
            );
            displayed_count += 1;
        }
    }

    if filtered_count > 0 {
        println!(
            "\nℹ️  {} system drive(s) hidden for safety.",
            filtered_count
        );
        println!("   Use --include-system to show all drives.");
    }

    if displayed_count == 0 && filtered_count > 0 {
        println!("\n⚠️  No non-system drives found.");
    }

    Ok(())
}

fn print_drive_detailed(drive: &DriveInfo) -> Result<()> {
    println!("\n{}", "=".repeat(60));
    println!("Device: {}", drive.device_path);
    println!("Model: {}", drive.model);
    println!("Serial: {}", drive.serial);
    println!("Size: {} GB", drive.size / (1024 * 1024 * 1024));
    println!("Type: {:?}", drive.drive_type);

    if let Some(health) = &drive.health_status {
        println!("Health: {:?}", health);
    }

    if let Some(temp) = drive.temperature_celsius {
        println!("Temperature: {}°C", temp);
    }

    println!("\nCapabilities:");
    let caps = &drive.capabilities;
    println!("  Secure Erase: {}", caps.secure_erase);
    println!("  Enhanced Erase: {}", caps.enhanced_erase);
    println!("  Crypto Erase: {}", caps.crypto_erase);
    println!("  TRIM Support: {}", caps.trim_support);
    println!("  Freeze Status: {:?}", caps.freeze_status);

    if caps.hpa_enabled {
        println!("  ⚠ HPA Enabled (hidden area present)");
    }
    if caps.dco_enabled {
        println!("  ⚠ DCO Enabled (device configuration overlay)");
    }

    if let Some(sed_type) = &caps.sed_type {
        println!("  SED Type: {:?}", sed_type);
    }

    if !caps.sanitize_options.is_empty() {
        println!("  NVMe Sanitize: {:?}", caps.sanitize_options);
    }

    println!("Encryption: {:?}", drive.encryption_status);

    Ok(())
}

async fn wipe_drive(
    device: &str,
    config: WipeConfig,
    cert_output: Option<&str>,
    force: bool,
    unsafe_mode: bool,
) -> Result<()> {
    // Detect the specific drive
    let drives = DriveDetector::detect_all_drives()?;
    let drive_info = drives
        .into_iter()
        .find(|d| d.device_path == device)
        .ok_or_else(|| anyhow::anyhow!("Drive not found: {}", device))?;

    // Safety checks
    if !unsafe_mode {
        if DriveDetector::is_system_drive(device)? {
            eprintln!("Error: {} appears to be a system drive.", device);
            eprintln!("Use --unsafe-mode to override (DANGEROUS!)");
            return Ok(());
        }

        if DriveDetector::is_mounted(device)? {
            eprintln!("Error: {} is currently mounted.", device);
            eprintln!("Please unmount before wiping.");
            return Ok(());
        }
    }

    // Health check
    if !force {
        if let Some(health) = &drive_info.health_status {
            if *health == HealthStatus::Failed || *health == HealthStatus::Critical {
                eprintln!("Error: Drive health is {:?}", health);
                eprintln!("Use --force to override.");
                return Ok(());
            }
        }

        if !SMARTMonitor::check_safe_to_operate(device)? {
            eprintln!("Error: Drive is not safe to operate.");
            eprintln!("Use --force to override.");
            return Ok(());
        }
    }

    // Confirmation
    if !unsafe_mode {
        println!(
            "\nWARNING: This will permanently erase ALL data on {}",
            device
        );
        println!("Drive: {} ({})", drive_info.model, drive_info.serial);
        println!("Size: {} GB", drive_info.size / (1024 * 1024 * 1024));

        if drive_info.capabilities.hpa_enabled || drive_info.capabilities.dco_enabled {
            println!("\n⚠ Hidden areas detected:");
            if drive_info.capabilities.hpa_enabled {
                println!("  - HPA (Host Protected Area) is enabled");
            }
            if drive_info.capabilities.dco_enabled {
                println!("  - DCO (Device Configuration Overlay) is enabled");
            }
        }

        print!("\nType 'YES' to confirm: ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        if input.trim() != "YES" {
            println!("Operation cancelled.");
            return Ok(());
        }
    }

    // Create wipe session
    let session = WipeSession {
        session_id: Uuid::new_v4().to_string(),
        start_time: chrono::Utc::now(),
        end_time: None,
        drives: vec![],
        config: config.clone(),
        operator_id: None,
    };

    // Perform the wipe
    wipe_single_drive(device, &drive_info, config, cert_output, session, force).await
}

async fn wipe_single_drive(
    device: &str,
    drive_info: &DriveInfo,
    config: WipeConfig,
    cert_output: Option<&str>,
    mut session: WipeSession,
    force: bool,
) -> Result<()> {
    println!(
        "\nStarting wipe of {} ({}, {})",
        device, drive_info.model, drive_info.serial
    );

    let start_time = Instant::now();
    let mut warnings = Vec::new();

    // Phase 1: Preparation
    println!("\nPhase 1: Preparation");

    // Handle freeze mitigation
    if config.freeze_mitigation && drive_info.capabilities.is_frozen {
        println!("Drive is frozen, attempting mitigation...");
        match FreezeMitigation::unfreeze_drive(device) {
            Ok(_) => println!("✓ Drive unfrozen successfully"),
            Err(e) => {
                let msg = format!("Failed to unfreeze: {}", e);
                warnings.push(msg.clone());
                eprintln!("⚠ {}", msg);
            }
        }
    }

    // Handle HPA/DCO
    let mut hpa_original = None;
    match config.handle_hpa_dco {
        HPADCOHandling::Detect => {
            if let Ok((hpa, dco)) = HPADCOManager::check_hidden_areas(device) {
                if hpa.is_some() || dco.is_some() {
                    warnings.push("Hidden areas detected but not removed".to_string());
                }
            }
        }
        HPADCOHandling::TemporaryRemove => {
            if let Ok(Some(hpa)) = HPADCOManager::detect_hpa(device) {
                hpa_original = Some(hpa.current_max_sectors);
                println!("Temporarily removing HPA...");
                HPADCOManager::remove_hpa_temporary(device)?;
                println!("✓ HPA temporarily removed");
            }
        }
        HPADCOHandling::PermanentRemove => {
            if HPADCOManager::detect_hpa(device)?.is_some() {
                println!("Permanently removing HPA...");
                HPADCOManager::remove_hpa_temporary(device)?;
                println!("✓ HPA permanently removed");
            }
            if HPADCOManager::detect_dco(device)?.is_some() {
                println!("Removing DCO...");
                HPADCOManager::remove_dco(device)?;
                println!("✓ DCO removed");
            }
        }
        _ => {}
    }

    // Temperature monitoring - ENHANCED VERSION
    if config.temperature_monitoring {
        println!("\n🌡️  Pre-flight Temperature Check");

        match SMARTMonitor::monitor_temperature(device) {
            Ok(temp_mon) => {
                println!("   Current: {}°C", temp_mon.current_celsius);
                println!("   Warning threshold: {}°C", temp_mon.warning_threshold);
                println!("   Critical threshold: {}°C", temp_mon.critical_threshold);

                if temp_mon.current_celsius > temp_mon.warning_threshold {
                    println!("\n⚠️  Drive temperature above safe operating threshold");

                    match SMARTMonitor::wait_for_safe_temperature(device, 300) {
                        Ok(_) => println!("✅ Temperature normalized"),
                        Err(e) => {
                            eprintln!("❌ Temperature safety check failed: {}", e);
                            if !force {
                                return Err(e.into());
                            }
                            eprintln!("⚠️  Proceeding anyway due to --force flag");
                        }
                    }
                } else {
                    println!("✅ Temperature within safe range");
                }
            }
            Err(e) => {
                eprintln!("⚠️  Temperature monitoring unavailable: {}", e);
                eprintln!("   Continuing without temperature safety checks");
                warnings.push(format!("Temperature monitoring disabled: {}", e));
            }
        }
    } else {
        println!("ℹ️  Temperature monitoring disabled by user");
    }

    // Phase 2: Wipe
    println!("\nPhase 2: Wiping");

    let wipe_result = match select_and_execute_wipe(device, drive_info, &config).await {
        Ok(_) => Ok(()),
        Err(e) => {
            warnings.push(format!("Wipe error: {}", e));

            // Check if this was a user interrupt
            if e.to_string().contains("interrupted") || e.to_string().contains("Interrupted") {
                eprintln!("\n❌ Wipe operation cancelled by user");
                return Err(e);
            }

            Err(e)
        }
    };

    // If wipe failed (not interrupted), continue to cleanup but skip verification
    if wipe_result.is_err() {
        eprintln!("\n⚠️  Wipe failed, skipping post-wipe operations");
        return wipe_result;
    }

    // Phase 3: Post-wipe operations
    println!("\nPhase 3: Post-wipe operations");

    // TRIM after wipe
    if config.use_trim_after && drive_info.capabilities.trim_support {
        println!("Performing TRIM operation...");
        match TrimOperations::secure_trim_with_verify(device) {
            Ok(_) => println!("✓ TRIM completed"),
            Err(e) => warnings.push(format!("TRIM failed: {}", e)),
        }
    }

    // Restore HPA if needed
    if let Some(original_sectors) = hpa_original {
        println!("Restoring original HPA configuration...");
        HPADCOManager::restore_hpa(device, original_sectors)?;
        println!("✓ HPA restored");
    }

    let wipe_duration = start_time.elapsed();
    println!(
        "\nWipe completed in {:.2} seconds",
        wipe_duration.as_secs_f64()
    );

    // Phase 4: Verification
    let verification_result = if config.verify {
        println!("\nPhase 4: Verification");
        let verified = RecoveryTest::verify_wipe(device, drive_info.size)?;
        let entropy_score = 7.8; // This would come from the actual verification

        VerificationResult {
            verified,
            entropy_score,
            recovery_test_passed: verified,
            verification_timestamp: chrono::Utc::now(),
        }
    } else {
        VerificationResult {
            verified: false,
            entropy_score: 0.0,
            recovery_test_passed: false,
            verification_timestamp: chrono::Utc::now(),
        }
    };

    // Generate certificate
    if let Some(cert_path) = cert_output {
        println!("\nGenerating certificate...");
        let cert_gen = CertificateGenerator::new();
        let wipe_details = WipeDetails {
            algorithm_used: format!("{:?}", config.algorithm),
            passes_completed: 1,
            duration_seconds: wipe_duration.as_secs(),
            operator_id: session.operator_id.clone(),
        };

        let certificate =
            cert_gen.generate_certificate(drive_info, wipe_details, verification_result.clone())?;
        cert_gen.save_certificate(&certificate, cert_path)?;
        println!("✓ Certificate saved to: {}", cert_path);
    }

    // Update session
    session.drives.push(DriveWipeRecord {
        drive_info: drive_info.clone(),
        status: if wipe_result.is_ok() {
            WipeStatus::Completed
        } else {
            WipeStatus::Failed
        },
        start_time: chrono::Utc::now() - chrono::Duration::seconds(wipe_duration.as_secs() as i64),
        end_time: Some(chrono::Utc::now()),
        error_message: wipe_result.err().map(|e| e.to_string()),
        certificate_path: cert_output.map(|s| s.to_string()),
        verification_passed: Some(verification_result.verified),
    });

    if !warnings.is_empty() {
        println!("\nWarnings:");
        for warning in warnings {
            println!("  ⚠ {}", warning);
        }
    }

    println!("\n✓ Operation completed successfully!");
    Ok(())
}

async fn select_and_execute_wipe(
    device: &str,
    drive_info: &DriveInfo,
    config: &WipeConfig,
) -> Result<()> {
    // Check if this is an advanced drive type that needs specialized handling
    match drive_info.drive_type {
        DriveType::SMR
        | DriveType::Optane
        | DriveType::HybridSSHD
        | DriveType::EMMC
        | DriveType::UFS => {
            // Use the advanced wipe orchestrator for these drive types
            println!(
                "🔬 Detected advanced drive type: {:?}",
                drive_info.drive_type
            );
            println!("Using specialized wipe strategy...\n");

            use sayonara_wipe::WipeOrchestrator;
            let mut orchestrator = WipeOrchestrator::new(device.to_string(), config.clone())
                .map_err(|e| anyhow::anyhow!("Orchestrator initialization failed: {}", e))?;

            orchestrator
                .execute()
                .await
                .map_err(|e| anyhow::anyhow!("Advanced wipe failed: {}", e))?;

            return Ok(());
        }
        DriveType::NVMe => {
            // Check if it's an advanced NVMe (ZNS, multi-namespace, etc.)
            use sayonara_wipe::drives::NVMeAdvanced;
            if NVMeAdvanced::detect_advanced_features(device).unwrap_or(false) {
                println!("🔬 Detected advanced NVMe features (ZNS/Multi-namespace)");
                println!("Using specialized wipe strategy...\n");

                use sayonara_wipe::WipeOrchestrator;
                let mut orchestrator = WipeOrchestrator::new(device.to_string(), config.clone())
                    .map_err(|e| anyhow::anyhow!("Orchestrator initialization failed: {}", e))?;

                orchestrator
                    .execute()
                    .await
                    .map_err(|e| anyhow::anyhow!("Advanced NVMe wipe failed: {}", e))?;

                return Ok(());
            }
            // Otherwise fall through to standard NVMe handling below
        }
        _ => {
            // Standard drives - use existing implementations
        }
    }

    // Auto-select best method based on capabilities and config
    let algorithm = if config.algorithm == Algorithm::SecureErase {
        // Auto-select based on drive capabilities
        if drive_info.capabilities.crypto_erase && config.sed_crypto_erase {
            Algorithm::CryptoErase
        } else if drive_info.drive_type == DriveType::NVMe
            && !drive_info.capabilities.sanitize_options.is_empty()
        {
            Algorithm::Sanitize
        } else if drive_info.capabilities.secure_erase {
            Algorithm::SecureErase
        } else {
            Algorithm::DoD5220
        }
    } else {
        config.algorithm.clone()
    };

    println!("Using algorithm: {:?}", algorithm);

    match algorithm {
        Algorithm::DoD5220 => {
            DoDWipe::wipe_drive(
                device,
                drive_info.size,
                drive_info.drive_type.clone(),
                &config,
            )?;
        }
        Algorithm::Gutmann => {
            GutmannWipe::wipe_drive(
                device,
                drive_info.size,
                drive_info.drive_type.clone(),
                &config,
            )?;
        }
        Algorithm::Random => {
            RandomWipe::wipe_drive(
                device,
                drive_info.size,
                drive_info.drive_type.clone(),
                &config,
            )?;
        }
        Algorithm::Zero => {
            // Simple zero overwrite
            DoDWipe::wipe_drive(
                device,
                drive_info.size,
                drive_info.drive_type.clone(),
                &config,
            )?; // Reuse with zero pattern
        }
        Algorithm::SecureErase => {
            // Try hardware secure erase with graceful fallback to software methods
            match drive_info.drive_type {
                DriveType::SSD => match SSDWipe::secure_erase(device) {
                    Ok(_) => {
                        println!("✅ Hardware secure erase completed successfully");
                    }
                    Err(e) => {
                        println!("\n⚠️  Hardware secure erase failed: {}", e);
                        println!("   Reason: Drive may not support ATA secure erase or is frozen");
                        println!("   Falling back to DoD 5220.22-M (3-pass software wipe)...");
                        println!("   This will take longer but will securely wipe the drive.\n");
                        DoDWipe::wipe_drive(
                            device,
                            drive_info.size,
                            drive_info.drive_type.clone(),
                            &config,
                        )?;
                    }
                },
                DriveType::NVMe => match NVMeWipe::secure_erase(device) {
                    Ok(_) => {
                        println!("✅ Hardware secure erase completed successfully");
                    }
                    Err(e) => {
                        println!("\n⚠️  Hardware secure erase failed: {}", e);
                        println!(
                            "   Reason: Drive may not support Format NVM or Sanitize commands"
                        );
                        println!("   Falling back to DoD 5220.22-M (3-pass software wipe)...");
                        println!("   This will take longer but will securely wipe the drive.\n");
                        DoDWipe::wipe_drive(
                            device,
                            drive_info.size,
                            drive_info.drive_type.clone(),
                            &config,
                        )?;
                    }
                },
                DriveType::HDD => match HDDWipe::secure_erase(device) {
                    Ok(_) => {
                        println!("✅ Hardware secure erase completed successfully");
                    }
                    Err(e) => {
                        println!("\n⚠️  Hardware secure erase failed: {}", e);
                        println!("   Reason: Drive may not support ATA secure erase or is frozen");
                        println!("   Falling back to DoD 5220.22-M (3-pass software wipe)...");
                        println!("   This will take longer but will securely wipe the drive.\n");
                        DoDWipe::wipe_drive(
                            device,
                            drive_info.size,
                            drive_info.drive_type.clone(),
                            &config,
                        )?;
                    }
                },
                _ => {
                    println!("ℹ️  Hardware secure erase not available for this drive type");
                    println!("   Using DoD 5220.22-M (3-pass software wipe)...\n");
                    DoDWipe::wipe_drive(
                        device,
                        drive_info.size,
                        drive_info.drive_type.clone(),
                        &config,
                    )?;
                }
            }
        }
        Algorithm::CryptoErase => {
            // Try crypto erase with fallback to DoD
            if let Ok(sed_info) = SEDManager::detect_sed(device) {
                match SEDManager::crypto_erase(device, &sed_info) {
                    Ok(_) => {
                        println!("✅ Cryptographic erase completed successfully");
                    }
                    Err(e) => {
                        println!("\n⚠️  Cryptographic erase failed: {}", e);
                        println!("   Reason: Drive may be locked or does not support crypto erase");
                        println!("   Falling back to DoD 5220.22-M (3-pass software wipe)...");
                        println!("   This will take longer but will securely wipe the drive.\n");
                        DoDWipe::wipe_drive(
                            device,
                            drive_info.size,
                            drive_info.drive_type.clone(),
                            &config,
                        )?;
                    }
                }
            } else {
                println!("\n⚠️  Self-Encrypting Drive (SED) not detected");
                println!("   Falling back to DoD 5220.22-M (3-pass software wipe)...\n");
                DoDWipe::wipe_drive(
                    device,
                    drive_info.size,
                    drive_info.drive_type.clone(),
                    &config,
                )?;
            }
        }
        Algorithm::Sanitize => {
            // Try NVMe sanitize with fallback to DoD
            if drive_info.drive_type == DriveType::NVMe {
                match NVMeWipe::secure_erase(device) {
                    Ok(_) => {
                        println!("✅ NVMe sanitize completed successfully");
                    }
                    Err(e) => {
                        println!("\n⚠️  NVMe sanitize failed: {}", e);
                        println!(
                            "   Reason: Drive may not support Sanitize or Format NVM commands"
                        );
                        println!("   Falling back to DoD 5220.22-M (3-pass software wipe)...");
                        println!("   This will take longer but will securely wipe the drive.\n");
                        DoDWipe::wipe_drive(
                            device,
                            drive_info.size,
                            drive_info.drive_type.clone(),
                            &config,
                        )?;
                    }
                }
            } else {
                println!("\n⚠️  Sanitize command only available for NVMe drives");
                println!("   Falling back to DoD 5220.22-M (3-pass software wipe)...\n");
                DoDWipe::wipe_drive(
                    device,
                    drive_info.size,
                    drive_info.drive_type.clone(),
                    &config,
                )?;
            }
        }
        Algorithm::TrimOnly => {
            if drive_info.capabilities.trim_support {
                TrimOperations::secure_trim_with_verify(device)?;
            } else {
                return Err(anyhow::anyhow!("TRIM not supported on this drive"));
            }
        }
    }

    Ok(())
}

async fn wipe_all_drives(
    config: WipeConfig,
    cert_dir: &str,
    exclude: Option<&str>,
    unsafe_mode: bool,
    force: bool,
) -> Result<()> {
    let drives = DriveDetector::detect_all_drives()?;

    if drives.is_empty() {
        println!("No drives detected.");
        return Ok(());
    }

    // Parse exclusion list
    let excluded_drives: Vec<&str> = exclude
        .map(|s| s.split(',').map(|s| s.trim()).collect())
        .unwrap_or_default();

    // Filter drives
    let mut drives_to_wipe = Vec::new();
    for drive in drives {
        if excluded_drives.contains(&drive.device_path.as_str()) {
            continue;
        }
        if !unsafe_mode && DriveDetector::is_system_drive(&drive.device_path)? {
            println!("Skipping system drive: {}", drive.device_path);
            continue;
        }
        if !unsafe_mode && DriveDetector::is_mounted(&drive.device_path)? {
            println!("Skipping mounted drive: {}", drive.device_path);
            continue;
        }
        drives_to_wipe.push(drive);
    }

    if drives_to_wipe.is_empty() {
        println!("No drives to wipe after applying filters.");
        return Ok(());
    }

    // Show what will be wiped
    println!("The following drives will be wiped:");
    for drive in &drives_to_wipe {
        println!(
            "  - {} ({}, {} GB)",
            drive.device_path,
            drive.model,
            drive.size / (1024 * 1024 * 1024)
        );
    }

    if !unsafe_mode {
        println!("\n⚠ WARNING: This action is IRREVERSIBLE!");
        print!("Type 'DESTROY_ALL_DATA' to confirm: ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        if input.trim() != "DESTROY_ALL_DATA" {
            println!("Operation cancelled.");
            return Ok(());
        }
    }

    // Create certificate directory
    std::fs::create_dir_all(cert_dir)?;

    // Create session
    let session = WipeSession {
        session_id: Uuid::new_v4().to_string(),
        start_time: chrono::Utc::now(),
        end_time: None,
        drives: vec![],
        config: config.clone(),
        operator_id: None,
    };

    let total_drives = drives_to_wipe.len();
    let mut successful = 0;
    let mut failed = 0;

    // Wipe each drive
    for (index, drive) in drives_to_wipe.iter().enumerate() {
        println!("\n{}", "=".repeat(60));
        println!(
            "Wiping drive {}/{}: {}",
            index + 1,
            total_drives,
            drive.device_path
        );
        println!("{}", "=".repeat(60));

        let cert_filename = drive.device_path.replace("/", "_").replace("dev_", "");
        let cert_path = format!("{}/cert_{}.json", cert_dir, cert_filename);

        let result = wipe_single_drive(
            &drive.device_path,
            drive,
            config.clone(),
            Some(&cert_path),
            session.clone(),
            force,
        )
        .await;

        match result {
            Ok(_) => {
                successful += 1;
                println!("✓ Successfully wiped {}", drive.device_path);
            }
            Err(e) => {
                failed += 1;
                println!("✗ Failed to wipe {}: {}", drive.device_path, e);
            }
        }
    }

    // Summary
    println!("\n{}", "=".repeat(60));
    println!("SUMMARY");
    println!("{}", "=".repeat(60));
    println!("Total drives: {}", total_drives);
    println!("Successful: {}", successful);
    println!("Failed: {}", failed);

    Ok(())
}

async fn verify_drive(device: &str, check_hidden: bool) -> Result<()> {
    let drives = DriveDetector::detect_all_drives()?;
    let drive_info = drives
        .into_iter()
        .find(|d| d.device_path == device)
        .ok_or_else(|| anyhow::anyhow!("Drive not found: {}", device))?;

    println!(
        "Verifying wipe on {} ({}, {})",
        device, drive_info.model, drive_info.serial
    );

    // Check for hidden areas if requested
    if check_hidden {
        println!("\nChecking for hidden areas...");
        let (hpa, dco) = HPADCOManager::check_hidden_areas(device)?;

        if hpa.is_some() || dco.is_some() {
            println!("⚠ WARNING: Hidden areas detected!");
            if let Some(h) = hpa {
                println!("  HPA: {} bytes hidden", h.hidden_size_bytes);
            }
            if let Some(d) = dco {
                println!("  DCO: {} bytes hidden", d.hidden_size_bytes);
            }
            println!("  These areas may contain recoverable data!");
        } else {
            println!("✓ No hidden areas detected");
        }
    }

    // Run verification test
    println!("\nRunning recovery test...");
    let verified = RecoveryTest::verify_wipe(device, drive_info.size)?;

    if verified {
        println!("✓ Verification PASSED - No recoverable data detected");
    } else {
        println!("✗ Verification FAILED - Recoverable data may be present");
    }

    // Check TRIM effectiveness if applicable
    if drive_info.capabilities.trim_support {
        println!("\nChecking TRIM effectiveness...");
        if TrimOperations::verify_trim_effectiveness(device, 100)? {
            println!("✓ TRIM appears to be effective");
        } else {
            println!("⚠ TRIM may not be fully effective");
        }
    }

    Ok(())
}

async fn check_health(device: &str, self_test: bool, monitor: bool) -> Result<()> {
    if device == "all" {
        // Check all drives
        let drives = DriveDetector::detect_all_drives()?;

        println!("Health Status for All Drives:");
        println!("{}", "=".repeat(80));

        for drive in drives {
            print_health_status(&drive.device_path).await?;
            println!();
        }
    } else {
        // Check specific drive
        if monitor {
            // Continuous monitoring
            println!("Monitoring {} (Press Ctrl+C to stop)...", device);
            loop {
                print!("\x1B[2J\x1B[1;1H"); // Clear screen
                print_health_status(device).await?;
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        } else {
            print_health_status(device).await?;

            if self_test {
                println!("\nRunning SMART self-test...");
                use crate::drives::operations::smart::{SMARTMonitor, SelfTestType};
                SMARTMonitor::run_self_test(device, SelfTestType::Short)?;
                println!(
                    "Self-test started. Check progress with 'smartctl -l selftest {}'",
                    device
                );
            }
        }
    }

    Ok(())
}

async fn print_health_status(device: &str) -> Result<()> {
    let health = SMARTMonitor::get_health(device)?;

    println!("Device: {}", device);
    println!("Overall Health: {:?}", health.overall_health);

    if let Some(temp) = health.temperature_celsius {
        let temp_status = if temp > 60 {
            "⚠ HIGH"
        } else if temp > 50 {
            "! Warm"
        } else {
            "✓ Normal"
        };
        println!("Temperature: {}°C {}", temp, temp_status);
    }

    if let Some(hours) = health.power_on_hours {
        println!("Power On Hours: {} ({} days)", hours, hours / 24);
    }

    if let Some(cycles) = health.power_cycle_count {
        println!("Power Cycles: {}", cycles);
    }

    // Critical attributes
    let mut has_issues = false;

    if let Some(reallocated) = health.reallocated_sectors {
        if reallocated > 0 {
            println!("⚠ Reallocated Sectors: {}", reallocated);
            has_issues = true;
        }
    }

    if let Some(pending) = health.pending_sectors {
        if pending > 0 {
            println!("⚠ Pending Sectors: {}", pending);
            has_issues = true;
        }
    }

    if let Some(uncorrectable) = health.uncorrectable_errors {
        if uncorrectable > 0 {
            println!("⚠ Uncorrectable Errors: {}", uncorrectable);
            has_issues = true;
        }
    }

    // SSD specific
    if let Some(wear) = health.wear_level {
        let wear_status = if wear > 90 {
            "⚠ CRITICAL"
        } else if wear > 80 {
            "! High"
        } else {
            "✓ Normal"
        };
        println!("SSD Wear Level: {}% {}", wear, wear_status);
    }

    // NVMe specific
    if let Some(spare) = health.available_spare {
        let spare_status = if spare < 10 {
            "⚠ CRITICAL"
        } else if spare < 20 {
            "! Low"
        } else {
            "✓ Normal"
        };
        println!("Available Spare: {}% {}", spare, spare_status);
    }

    if !has_issues && health.overall_health == HealthStatus::Good {
        println!("✓ No issues detected");
    }

    // Failure prediction
    use crate::drives::operations::smart::SMARTMonitor;
    let prediction = SMARTMonitor::predict_failure(device)?;

    if prediction.risk_score > 0 {
        println!("\nFailure Risk Assessment:");
        println!("  Risk Level: {:?}", prediction.risk_level);
        println!("  Risk Score: {}/100", prediction.risk_score);

        if let Some(days) = prediction.estimated_days_remaining {
            println!("  Estimated Time to Failure: {} days", days);
        }

        if !prediction.failure_indicators.is_empty() {
            println!("  Indicators:");
            for indicator in &prediction.failure_indicators {
                println!("    - {}", indicator);
            }
        }

        println!("  Recommendation: {}", prediction.recommendation);
    }

    Ok(())
}

async fn handle_sed(device: &str, action: &SedAction) -> Result<()> {
    match action {
        SedAction::Status => {
            let sed_info = SEDManager::detect_sed(device)?;

            println!("Self-Encrypting Drive Status for {}", device);
            println!("{}", "=".repeat(50));

            match sed_info.sed_type {
                SEDType::None => {
                    println!("No SED capabilities detected");
                }
                _ => {
                    println!("SED Type: {:?}", sed_info.sed_type);
                    println!("Locked: {}", sed_info.locked);
                    println!("Enabled: {}", sed_info.enabled);
                    println!("Frozen: {}", sed_info.frozen);

                    if let Some(max_tries) = sed_info.max_password_tries {
                        println!("Max Password Tries: {}", max_tries);
                    }

                    println!("Crypto Erase Support: {}", sed_info.supports_crypto_erase);
                    println!(
                        "Instant Secure Erase: {}",
                        sed_info.supports_instant_secure_erase
                    );

                    if let Some(fw) = sed_info.firmware_version {
                        println!("Firmware: {}", fw);
                    }
                }
            }
        }

        SedAction::CryptoErase { password } => {
            let sed_info = SEDManager::detect_sed(device)?;

            if sed_info.sed_type == SEDType::None {
                eprintln!("Error: No SED capabilities detected on this drive");
                return Ok(());
            }

            if !sed_info.supports_crypto_erase {
                eprintln!("Error: This drive does not support cryptographic erase");
                return Ok(());
            }

            // Handle locked drives
            if sed_info.locked {
                if let Some(pwd) = password {
                    println!("Unlocking drive...");
                    SEDManager::unlock_sed(device, pwd, &sed_info)?;
                } else {
                    eprintln!("Error: Drive is locked. Please provide password with --password");
                    return Ok(());
                }
            }

            println!("WARNING: Cryptographic erase will instantly destroy all data!");
            print!("Type 'ERASE' to confirm: ");
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;

            if input.trim() != "ERASE" {
                println!("Operation cancelled.");
                return Ok(());
            }

            println!("Performing cryptographic erase...");
            SEDManager::crypto_erase(device, &sed_info)?;

            // Verify
            if SEDManager::verify_crypto_erase(device)? {
                println!("✓ Cryptographic erase completed successfully");
            } else {
                println!("⚠ Cryptographic erase completed but verification shows unexpected data");
            }
        }

        SedAction::Unlock { password } => {
            let sed_info = SEDManager::detect_sed(device)?;

            if !sed_info.locked {
                println!("Drive is not locked");
                return Ok(());
            }

            println!("Attempting to unlock drive...");
            SEDManager::unlock_sed(device, password, &sed_info)?;
            println!("✓ Drive unlocked successfully");
        }
    }

    Ok(())
}

fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

// Additional helper functions for parallel operations
#[allow(dead_code)]
async fn wipe_drives_parallel(
    drives: Vec<DriveInfo>,
    config: WipeConfig,
    cert_dir: &str,
    max_parallel: usize,
    force: bool,
) -> Result<Vec<DriveWipeRecord>> {
    use futures::stream::{self, StreamExt};

    let results = stream::iter(drives)
        .map(|drive| {
            let config = config.clone();
            let cert_dir = cert_dir.to_string();
            async move {
                let cert_filename = drive.device_path.replace("/", "_").replace("dev_", "");
                let cert_path = format!("{}/cert_{}.json", cert_dir, cert_filename);

                let session = WipeSession {
                    session_id: Uuid::new_v4().to_string(),
                    start_time: chrono::Utc::now(),
                    end_time: None,
                    drives: vec![],
                    config: config.clone(),
                    operator_id: None,
                };

                match wipe_single_drive(
                    &drive.device_path,
                    &drive,
                    config,
                    Some(&cert_path),
                    session,
                    force,
                )
                .await
                {
                    Ok(_) => DriveWipeRecord {
                        drive_info: drive.clone(),
                        status: WipeStatus::Completed,
                        start_time: chrono::Utc::now(),
                        end_time: Some(chrono::Utc::now()),
                        error_message: None,
                        certificate_path: Some(cert_path),
                        verification_passed: Some(true),
                    },
                    Err(e) => DriveWipeRecord {
                        drive_info: drive.clone(),
                        status: WipeStatus::Failed,
                        start_time: chrono::Utc::now(),
                        end_time: Some(chrono::Utc::now()),
                        error_message: Some(e.to_string()),
                        certificate_path: None,
                        verification_passed: Some(false),
                    },
                }
            }
        })
        .buffer_unordered(max_parallel)
        .collect::<Vec<_>>()
        .await;

    Ok(results)
}

// Signal handler for graceful shutdown
fn setup_signal_handlers() -> Result<()> {
    use signal_hook::{consts::SIGINT, iterator::Signals};

    let mut signals = Signals::new(&[SIGINT])?;

    std::thread::spawn(move || {
        for sig in signals.forever() {
            match sig {
                SIGINT => {
                    eprintln!("\n\n🛑 Interrupt received! Stopping wipe operation...");
                    eprintln!("   Please wait for current buffer to finish writing...");
                    sayonara_wipe::set_interrupted();
                }
                _ => {}
            }
        }
    });

    Ok(())
}
