#[cfg(test)]
mod tests {
    use crate::verification::enhanced::*;

    type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

    // ==================== ENTROPY CALCULATION TESTS ====================

    #[test]
    fn test_entropy_calculation_extremes() {
        // All zeros - minimum entropy
        let zeros = vec![0u8; 10000];
        let entropy = EnhancedVerification::calculate_entropy(&zeros).unwrap();
        assert!(
            entropy < 0.1,
            "All zeros should have near-zero entropy, got {}",
            entropy
        );

        // All ones - minimum entropy
        let ones = vec![0xFF; 10000];
        let entropy = EnhancedVerification::calculate_entropy(&ones).unwrap();
        assert!(
            entropy < 0.1,
            "All ones should have near-zero entropy, got {}",
            entropy
        );

        // Perfect distribution - maximum entropy
        let mut perfect = Vec::new();
        for _ in 0..40 {
            for i in 0..=255u8 {
                perfect.push(i);
            }
        }
        let entropy = EnhancedVerification::calculate_entropy(&perfect).unwrap();
        assert!(
            entropy > 7.99,
            "Perfect distribution should have ~8 bits entropy, got {}",
            entropy
        );

        // Random data - high entropy
        use crate::crypto::secure_rng::secure_random_bytes;
        let mut random = vec![0u8; 10000];
        secure_random_bytes(&mut random).unwrap();
        let entropy = EnhancedVerification::calculate_entropy(&random).unwrap();
        assert!(
            entropy > 7.5,
            "Random data should have high entropy, got {}",
            entropy
        );
    }

    #[test]
    fn test_entropy_calculation_empty() {
        let empty: Vec<u8> = vec![];
        let entropy = EnhancedVerification::calculate_entropy(&empty).unwrap();
        assert_eq!(entropy, 0.0, "Empty data should have zero entropy");
    }

    // ==================== CHI-SQUARE TESTS ====================

    #[test]
    fn test_chi_square_uniform_distribution() {
        // Create uniform distribution
        let mut data = Vec::new();
        for _ in 0..4 {
            for i in 0..=255u8 {
                data.push(i);
            }
        }

        let chi_square = EnhancedVerification::chi_square_test(&data).unwrap();

        // Chi-square should be close to 0 for perfect uniform distribution
        assert!(
            chi_square < 300.0,
            "Chi-square too high for uniform distribution: {}",
            chi_square
        );
    }

    #[test]
    fn test_chi_square_non_uniform() {
        // Create highly non-uniform distribution (all zeros)
        let data = vec![0u8; 10000];
        let chi_square = EnhancedVerification::chi_square_test(&data).unwrap();

        // Chi-square should be very high
        assert!(
            chi_square > 1000.0,
            "Chi-square should be high for non-uniform distribution: {}",
            chi_square
        );
    }

    // ==================== PATTERN ANALYSIS TESTS ====================

    #[test]
    fn test_repeating_pattern_detection() -> Result<()> {
        // Create data with repeating pattern
        let pattern = vec![0xAA, 0xBB, 0xCC, 0xDD];
        let mut data = Vec::new();
        for _ in 0..1000 {
            data.extend_from_slice(&pattern);
        }

        let analysis = EnhancedVerification::analyze_patterns(&data)?;
        assert!(
            analysis.repeating_patterns_found,
            "Should detect repeating patterns"
        );

        Ok(())
    }

    #[test]
    fn test_file_signature_detection() -> Result<()> {
        let mut data = vec![0u8; 10000];

        // Insert PDF signature
        data[100..104].copy_from_slice(b"%PDF");

        // Insert JPEG signature
        data[5000..5003].copy_from_slice(b"\xFF\xD8\xFF");

        let analysis = EnhancedVerification::analyze_patterns(&data)?;
        assert!(
            analysis.known_file_signatures,
            "Should detect file signatures"
        );
        assert!(
            analysis.detected_signatures.len() >= 2,
            "Should find at least 2 signatures"
        );

        Ok(())
    }

    #[test]
    fn test_no_signatures_in_random_data() -> Result<()> {
        use crate::crypto::secure_rng::secure_random_bytes;
        let mut data = vec![0u8; 10000];
        secure_random_bytes(&mut data)?;

        let analysis = EnhancedVerification::analyze_patterns(&data)?;

        // Random data should not have known file signatures
        // (extremely unlikely, but possible by chance)
        // We test for the absence of common signatures
        let common_sigs = analysis
            .detected_signatures
            .iter()
            .filter(|s| s.signature_name == "PDF" || s.signature_name == "JPEG")
            .count();

        assert_eq!(
            common_sigs, 0,
            "Random data should not have common file signatures"
        );

        Ok(())
    }

    // ==================== STATISTICAL TESTS ====================

    #[test]
    fn test_runs_test_random_data() -> Result<()> {
        use crate::crypto::secure_rng::secure_random_bytes;
        let mut data = vec![0u8; 10000];
        secure_random_bytes(&mut data)?;

        let passed = EnhancedVerification::runs_test(&data)?;
        assert!(passed, "Runs test should pass for random data");

        Ok(())
    }

    #[test]
    fn test_runs_test_non_random() -> Result<()> {
        // All zeros - no runs
        let data = vec![0u8; 1000];
        let passed = EnhancedVerification::runs_test(&data)?;
        assert!(!passed, "Runs test should fail for all zeros");

        Ok(())
    }

    #[test]
    fn test_monobit_test_random() -> Result<()> {
        use crate::crypto::secure_rng::secure_random_bytes;
        let mut data = vec![0u8; 10000];
        secure_random_bytes(&mut data)?;

        let passed = EnhancedVerification::monobit_test(&data)?;
        assert!(passed, "Monobit test should pass for random data");

        Ok(())
    }

    #[test]
    fn test_monobit_test_biased() -> Result<()> {
        // All ones - highly biased
        let data = vec![0xFF; 1000];
        let passed = EnhancedVerification::monobit_test(&data)?;
        assert!(!passed, "Monobit test should fail for biased data");

        Ok(())
    }

    #[test]
    fn test_poker_test() -> Result<()> {
        use crate::crypto::secure_rng::secure_random_bytes;
        let mut data = vec![0u8; 10000];
        secure_random_bytes(&mut data)?;

        let passed = EnhancedVerification::poker_test(&data)?;
        assert!(passed, "Poker test should pass for random data");

        Ok(())
    }

    #[test]
    fn test_serial_test() -> Result<()> {
        use crate::crypto::secure_rng::secure_random_bytes;
        let mut data = vec![0u8; 10000];
        secure_random_bytes(&mut data)?;

        let passed = EnhancedVerification::serial_test(&data)?;
        assert!(passed, "Serial test should pass for random data");

        Ok(())
    }

    #[test]
    fn test_autocorrelation_test() -> Result<()> {
        use crate::crypto::secure_rng::secure_random_bytes;
        let mut data = vec![0u8; 10000];
        secure_random_bytes(&mut data)?;

        let passed = EnhancedVerification::autocorrelation_test(&data)?;
        assert!(passed, "Autocorrelation test should pass for random data");

        Ok(())
    }

    // ==================== SUSPICIOUS DATA DETECTION ====================

    #[test]
    fn test_suspicious_data_detection_positive() {
        // Data with suspicious patterns
        let mut data = vec![0u8; 1000];
        data[100..108].copy_from_slice(b"PASSWORD");

        assert!(
            EnhancedVerification::detect_suspicious_data(&data),
            "Should detect PASSWORD string"
        );
    }

    #[test]
    fn test_suspicious_data_detection_negative() {
        use crate::crypto::secure_rng::secure_random_bytes;
        let mut data = vec![0u8; 1000];
        secure_random_bytes(&mut data).unwrap();

        assert!(
            !EnhancedVerification::detect_suspicious_data(&data),
            "Random data should not be flagged as suspicious"
        );
    }

    #[test]
    fn test_suspicious_data_low_entropy() {
        // Low entropy data (structured)
        let data = vec![0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF].repeat(100);

        assert!(
            EnhancedVerification::detect_suspicious_data(&data),
            "Low entropy data should be flagged as suspicious"
        );
    }

    // ==================== FILE SIGNATURE TESTS ====================

    #[test]
    fn test_file_signature_database_completeness() {
        let signatures = EnhancedVerification::FILE_SIGNATURES;

        assert!(
            signatures.len() >= 30,
            "Should have at least 30 file signatures"
        );

        // Check for key signatures
        let signature_names: Vec<&str> = signatures.iter().map(|s| s.name).collect();

        assert!(signature_names.contains(&"PDF"), "Should include PDF");
        assert!(signature_names.contains(&"JPEG"), "Should include JPEG");
        assert!(signature_names.contains(&"PNG"), "Should include PNG");
        assert!(signature_names.contains(&"ZIP"), "Should include ZIP");
        assert!(
            signature_names.contains(&"Windows EXE"),
            "Should include EXE"
        );
        assert!(signature_names.contains(&"Linux ELF"), "Should include ELF");
    }

    #[test]
    fn test_file_signature_confidence_levels() {
        let signatures = EnhancedVerification::FILE_SIGNATURES;

        for sig in signatures {
            assert!(
                sig.confidence >= 0.0 && sig.confidence <= 1.0,
                "Confidence for {} should be between 0 and 1",
                sig.name
            );
            assert!(
                !sig.pattern.is_empty(),
                "Pattern for {} should not be empty",
                sig.name
            );
        }
    }

    // ==================== RECOVERY RISK CALCULATION ====================

    #[test]
    fn test_recovery_risk_none() {
        let photorec = PhotoRecResults {
            signatures_scanned: 50,
            signatures_found: vec![],
            recoverable_files_estimated: 0,
            confidence: 0.95,
            would_succeed: false,
        };

        let testdisk = TestDiskResults {
            mbr_signature_found: false,
            gpt_header_found: false,
            partition_table_recoverable: false,
            filesystem_signatures: vec![],
            would_succeed: false,
        };

        let filesystem = FilesystemMetadataResults {
            superblock_remnants: vec![],
            inode_structures: false,
            journal_data: false,
            fat_tables: false,
            ntfs_mft: false,
        };

        let risk =
            EnhancedVerification::calculate_recovery_risk(&photorec, &testdisk, &filesystem, None);

        assert_eq!(risk, RecoveryRisk::None, "Should have no recovery risk");
    }

    #[test]
    fn test_recovery_risk_critical() {
        let photorec = PhotoRecResults {
            signatures_scanned: 50,
            signatures_found: vec![FileSignatureMatch {
                signature_name: "PDF".to_string(),
                offset: 1000,
                pattern_length: 4,
                confidence: 0.99,
            }],
            recoverable_files_estimated: 100,
            confidence: 0.95,
            would_succeed: true,
        };

        let testdisk = TestDiskResults {
            mbr_signature_found: true,
            gpt_header_found: true,
            partition_table_recoverable: true,
            filesystem_signatures: vec!["ext4".to_string(), "NTFS".to_string()],
            would_succeed: true,
        };

        let filesystem = FilesystemMetadataResults {
            superblock_remnants: vec!["ext4".to_string()],
            inode_structures: true,
            journal_data: true,
            fat_tables: false,
            ntfs_mft: true,
        };

        let risk =
            EnhancedVerification::calculate_recovery_risk(&photorec, &testdisk, &filesystem, None);

        assert!(
            matches!(risk, RecoveryRisk::High | RecoveryRisk::Critical),
            "Should have high or critical recovery risk, got {:?}",
            risk
        );
    }

    // ==================== CONFIDENCE LEVEL TESTS ====================

    #[test]
    fn test_confidence_calculation_perfect() {
        let pre_wipe = PreWipeTestResults {
            test_pattern_detection: true,
            recovery_tool_simulation: true,
            sensitivity_calibration: 95.0,
            false_positive_rate: 0.01,
            false_negative_rate: 0.01,
        };

        let post_wipe = create_perfect_post_wipe_analysis();

        let confidence = EnhancedVerification::calculate_confidence_level(&pre_wipe, &post_wipe);

        // Perfect conditions should yield very high confidence (typically 94-100%)
        assert!(
            confidence >= 93.0,
            "Perfect wipe should have >=93% confidence, got {}",
            confidence
        );
    }

    #[test]
    fn test_confidence_calculation_poor() {
        let pre_wipe = PreWipeTestResults {
            test_pattern_detection: false,
            recovery_tool_simulation: false,
            sensitivity_calibration: 50.0,
            false_positive_rate: 0.2,
            false_negative_rate: 0.2,
        };

        let post_wipe = create_poor_post_wipe_analysis();

        let confidence = EnhancedVerification::calculate_confidence_level(&pre_wipe, &post_wipe);

        assert!(
            confidence < 50.0,
            "Poor wipe should have <50% confidence, got {}",
            confidence
        );
    }

    // ==================== HEAT MAP TESTS ====================

    #[test]
    fn test_heat_map_ascii_rendering() {
        let heat_map = EntropyHeatMap {
            width: 10,
            height: 5,
            cells: vec![
                vec![8.0, 8.0, 8.0, 8.0, 8.0, 8.0, 8.0, 8.0, 8.0, 8.0],
                vec![7.5, 7.5, 7.5, 7.5, 7.5, 7.5, 7.5, 7.5, 7.5, 7.5],
                vec![7.0, 7.0, 7.0, 7.0, 7.0, 7.0, 7.0, 7.0, 7.0, 7.0],
                vec![6.0, 6.0, 6.0, 6.0, 6.0, 6.0, 6.0, 6.0, 6.0, 6.0],
                vec![3.0, 3.0, 3.0, 3.0, 3.0, 3.0, 3.0, 3.0, 3.0, 3.0],
            ],
            min_entropy: 3.0,
            max_entropy: 8.0,
            suspicious_blocks: vec![(0, 4), (1, 4)],
        };

        let ascii = EnhancedVerification::render_heat_map_ascii(&heat_map);

        assert!(ascii.contains("Heat Map"), "Should contain title");
        assert!(ascii.contains("3.00 - 8.00"), "Should show range");
        assert!(ascii.contains("█"), "Should contain block characters");
        assert!(ascii.len() > 100, "Should be substantial output");
    }

    // ==================== COMPLIANCE DETERMINATION TESTS ====================

    #[test]
    fn test_compliance_standards_high_confidence() {
        let post_wipe = create_perfect_post_wipe_analysis();
        let standards = EnhancedVerification::determine_compliance(&post_wipe, 99.5);

        assert!(standards.contains(&"DoD 5220.22-M".to_string()));
        assert!(standards.contains(&"NIST 800-88 Rev. 1".to_string()));
        assert!(standards.len() >= 4, "Should meet multiple standards");
    }

    #[test]
    fn test_compliance_standards_low_confidence() {
        let post_wipe = create_poor_post_wipe_analysis();
        let standards = EnhancedVerification::determine_compliance(&post_wipe, 70.0);

        assert!(
            standards.len() < 3,
            "Should meet few standards at low confidence"
        );
    }

    // ==================== HELPER FUNCTIONS ====================

    fn create_perfect_post_wipe_analysis() -> PostWipeAnalysis {
        PostWipeAnalysis {
            entropy_score: 7.99,
            chi_square_test: 250.0,
            pattern_analysis: PatternAnalysis {
                repeating_patterns_found: false,
                known_file_signatures: false,
                structured_data_detected: false,
                compression_ratio: 0.98,
                detected_signatures: vec![],
            },
            statistical_tests: StatisticalTests {
                runs_test_passed: true,
                monobit_test_passed: true,
                poker_test_passed: true,
                serial_test_passed: true,
                autocorrelation_test_passed: true,
            },
            sector_sampling: SectorSamplingResult {
                total_sectors_sampled: 1000,
                suspicious_sectors: 0,
                entropy_distribution: vec![7.9; 1000],
                anomaly_locations: vec![],
            },
            hidden_areas: HiddenAreaVerification {
                hpa_verified: true,
                hpa_sectors_checked: 0,
                hpa_entropy: None,
                dco_verified: true,
                dco_sectors_checked: 0,
                remapped_sectors_found: 0,
                remapped_sectors_verified: 0,
                controller_cache_flushed: true,
                over_provisioning_verified: true,
                wear_leveling_checked: true,
                hidden_area_warnings: vec![],
            },
            recovery_simulation: RecoverySimulationResults {
                photorec_results: PhotoRecResults {
                    signatures_scanned: 50,
                    signatures_found: vec![],
                    recoverable_files_estimated: 0,
                    confidence: 0.95,
                    would_succeed: false,
                },
                testdisk_results: TestDiskResults {
                    mbr_signature_found: false,
                    gpt_header_found: false,
                    partition_table_recoverable: false,
                    filesystem_signatures: vec![],
                    would_succeed: false,
                },
                filesystem_metadata: FilesystemMetadataResults {
                    superblock_remnants: vec![],
                    inode_structures: false,
                    journal_data: false,
                    fat_tables: false,
                    ntfs_mft: false,
                },
                mfm_simulation: None,
                overall_recovery_risk: RecoveryRisk::None,
            },
            bad_sectors: BadSectorTracker {
                bad_sectors: vec![],
                unreadable_count: 0,
                percentage_unreadable: 0.0,
                total_sectors_attempted: 1000,
            },
            heat_map: None,
        }
    }

    fn create_poor_post_wipe_analysis() -> PostWipeAnalysis {
        PostWipeAnalysis {
            entropy_score: 5.5,
            chi_square_test: 500.0,
            pattern_analysis: PatternAnalysis {
                repeating_patterns_found: true,
                known_file_signatures: true,
                structured_data_detected: true,
                compression_ratio: 0.5,
                detected_signatures: vec![FileSignatureMatch {
                    signature_name: "PDF".to_string(),
                    offset: 1000,
                    pattern_length: 4,
                    confidence: 0.99,
                }],
            },
            statistical_tests: StatisticalTests {
                runs_test_passed: false,
                monobit_test_passed: false,
                poker_test_passed: false,
                serial_test_passed: false,
                autocorrelation_test_passed: false,
            },
            sector_sampling: SectorSamplingResult {
                total_sectors_sampled: 1000,
                suspicious_sectors: 100,
                entropy_distribution: vec![5.0; 1000],
                anomaly_locations: (0..100).collect(),
            },
            hidden_areas: HiddenAreaVerification {
                hpa_verified: false,
                hpa_sectors_checked: 100,
                hpa_entropy: Some(4.0),
                dco_verified: false,
                dco_sectors_checked: 0,
                remapped_sectors_found: 50,
                remapped_sectors_verified: 25,
                controller_cache_flushed: false,
                over_provisioning_verified: false,
                wear_leveling_checked: false,
                hidden_area_warnings: vec!["Multiple issues".to_string()],
            },
            recovery_simulation: RecoverySimulationResults {
                photorec_results: PhotoRecResults {
                    signatures_scanned: 50,
                    signatures_found: vec![FileSignatureMatch {
                        signature_name: "PDF".to_string(),
                        offset: 1000,
                        pattern_length: 4,
                        confidence: 0.99,
                    }],
                    recoverable_files_estimated: 100,
                    confidence: 0.95,
                    would_succeed: true,
                },
                testdisk_results: TestDiskResults {
                    mbr_signature_found: true,
                    gpt_header_found: true,
                    partition_table_recoverable: true,
                    filesystem_signatures: vec!["ext4".to_string()],
                    would_succeed: true,
                },
                filesystem_metadata: FilesystemMetadataResults {
                    superblock_remnants: vec!["ext4".to_string()],
                    inode_structures: true,
                    journal_data: true,
                    fat_tables: false,
                    ntfs_mft: true,
                },
                mfm_simulation: Some(MFMResults {
                    theoretical_recovery_possible: true,
                    confidence_level: 75.0,
                    affected_sectors: 50,
                    flux_transition_anomalies: 1000,
                }),
                overall_recovery_risk: RecoveryRisk::Critical,
            },
            bad_sectors: BadSectorTracker {
                bad_sectors: (0..50).collect(),
                unreadable_count: 50,
                percentage_unreadable: 5.0,
                total_sectors_attempted: 1000,
            },
            heat_map: None,
        }
    }

    // ==================== INTEGRATION TESTS ====================
    // Full verification integration test has been moved to:
    // tests/hardware_integration.rs::test_verification_after_wipe
    // This test uses mock drives and can run without physical hardware or root

    // ==================== BENCHMARK TESTS ====================

    #[test]
    fn bench_entropy_calculation() {
        use std::time::Instant;

        let data = vec![0xAAu8; 1024 * 1024]; // 1MB

        let start = Instant::now();
        for _ in 0..100 {
            let _ = EnhancedVerification::calculate_entropy(&data);
        }
        let duration = start.elapsed();

        let per_iteration = duration.as_millis() / 100;
        println!("Entropy calculation: {} ms per MB", per_iteration);

        assert!(per_iteration < 50, "Entropy calculation should be fast");
    }

    #[test]
    fn bench_chi_square_test() {
        use std::time::Instant;

        let data = vec![0xAAu8; 1024 * 1024]; // 1MB

        let start = Instant::now();
        for _ in 0..100 {
            let _ = EnhancedVerification::chi_square_test(&data);
        }
        let duration = start.elapsed();

        let per_iteration = duration.as_millis() / 100;
        println!("Chi-square test: {} ms per MB", per_iteration);

        assert!(per_iteration < 50, "Chi-square test should be fast");
    }
}
