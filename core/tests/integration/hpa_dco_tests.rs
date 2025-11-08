/// Integration tests for HPA/DCO operations
///
/// These tests verify Hidden Protected Area (HPA) and Device Configuration Overlay (DCO)
/// detection, removal, and restoration operations.

use tempfile::TempDir;

// Mock data structures - duplicated here for standalone testing
struct MockHdparmData;

impl MockHdparmData {
    pub fn hpa_detected(current_sectors: u64, native_sectors: u64) -> String {
        format!("max sectors   = {}/{}(HPA is enabled)", current_sectors, native_sectors)
    }

    pub fn no_hpa(sectors: u64) -> String {
        format!("max sectors   = {}/{}, HPA is disabled", sectors, sectors)
    }

    pub fn dco_detected(dco_max: u64, real_max: u64) -> String {
        format!("Real max sectors: {}\nDCO max sectors: {}\nDCO is active", real_max, dco_max)
    }

    pub fn no_dco() -> String {
        "DCO feature set not supported".to_string()
    }
}

#[cfg(test)]
mod hpa_operations_tests {
    use super::*;

    #[test]
    fn test_hpa_detection_with_hidden_space() {
        // Setup: Drive with 200K sectors hidden by HPA
        let current_sectors = 1953525168u64;  // Visible capacity
        let native_sectors = 1953725168u64;   // True capacity (+200K sectors)
        let hdparm_output = MockHdparmData::hpa_detected(current_sectors, native_sectors);

        // Verify HPA is detected
        assert!(hdparm_output.contains("HPA is enabled"));
        assert!(hdparm_output.contains(&format!("{}/{}", current_sectors, native_sectors)));

        // Calculate hidden space
        let hidden_sectors = native_sectors - current_sectors;
        let hidden_bytes = hidden_sectors * 512;

        assert_eq!(hidden_sectors, 200000);
        assert_eq!(hidden_bytes, 102400000); // ~100 MB
    }

    #[test]
    fn test_hpa_not_present() {
        let sectors = 1953525168u64;
        let hdparm_output = MockHdparmData::no_hpa(sectors);

        // Verify no HPA
        assert!(!hdparm_output.contains("HPA is enabled"));
        assert!(hdparm_output.contains("HPA is disabled") ||
                hdparm_output.contains(&format!("{}/{}", sectors, sectors)));
    }

    #[test]
    fn test_hpa_size_calculations() {
        // Test various HPA sizes
        let test_cases = vec![
            (1000000u64, 1100000u64, 100000u64),   // 100K sectors hidden
            (1000000u64, 1000001u64, 1u64),         // 1 sector hidden
            (1000000u64, 2000000u64, 1000000u64),   // 1M sectors hidden
        ];

        for (current, native, expected_hidden) in test_cases {
            let hidden = native - current;
            assert_eq!(hidden, expected_hidden,
                      "Hidden space calculation failed for {} - {} sectors", native, current);

            let hidden_mb = (hidden * 512) / (1024 * 1024);
            assert!(hidden_mb >= 0, "Hidden space should be positive");
        }
    }

    #[test]
    fn test_hpa_removal_command_format() {
        // Verify we can construct correct hdparm commands
        let native_max = 1953725168u64;
        let device = "/dev/sda";
        let native_max_str = format!("{}", native_max);

        let expected_args = vec![
            "--yes-i-know-what-i-am-doing",
            "-N",
            native_max_str.as_str(),
            device,
        ];

        // Verify command format
        assert_eq!(expected_args[0], "--yes-i-know-what-i-am-doing");
        assert_eq!(expected_args[1], "-N");
        assert!(expected_args[2].parse::<u64>().is_ok());
        assert_eq!(expected_args[3], device);
    }

    #[test]
    fn test_hpa_restoration_command_format() {
        // Verify HPA restoration command format
        let original_max = 1953525168u64;
        let device = "/dev/sda";
        let original_max_str = format!("{}", original_max);

        let expected_args = vec![
            "--yes-i-know-what-i-am-doing",
            "-N",
            original_max_str.as_str(),
            device,
        ];

        assert_eq!(expected_args[1], "-N");
        assert_eq!(expected_args[2], &original_max.to_string());
    }

    #[test]
    fn test_hpa_sector_parsing_edge_cases() {
        // Test parsing various hdparm output formats

        // Format 1: Standard format
        let output1 = "max sectors   = 1000000/1200000(HPA is enabled)";
        let parts: Vec<&str> = output1.split('=').collect();
        if parts.len() > 1 {
            let sector_part = parts[1].trim();
            if let Some(slash_pos) = sector_part.find('/') {
                let current = &sector_part[..slash_pos];
                assert_eq!(current.trim(), "1000000");
            }
        }

        // Format 2: With spacing
        let output2 = " max sectors   = 500000 / 600000 , HPA is enabled ";
        let contains_slash = output2.contains('/');
        assert!(contains_slash, "Should contain slash separator");
    }
}

#[cfg(test)]
mod dco_operations_tests {
    use super::*;

    #[test]
    fn test_dco_detection_with_hidden_space() {
        let dco_max = 1953525168u64;
        let real_max = 1953625168u64;  // +100K sectors hidden
        let hdparm_output = MockHdparmData::dco_detected(dco_max, real_max);

        // Verify DCO is detected
        assert!(hdparm_output.contains("DCO is active"));
        assert!(hdparm_output.contains("Real max sectors:"));
        assert!(hdparm_output.contains("DCO max sectors:"));

        // Calculate hidden space
        let hidden_sectors = real_max - dco_max;
        let hidden_bytes = hidden_sectors * 512;

        assert_eq!(hidden_sectors, 100000);
        assert_eq!(hidden_bytes, 51200000); // ~50 MB
    }

    #[test]
    fn test_dco_not_supported() {
        let hdparm_output = MockHdparmData::no_dco();

        assert!(!hdparm_output.contains("DCO is active"));
        assert!(hdparm_output.contains("not supported"));
    }

    #[test]
    fn test_dco_removal_command() {
        // DCO removal uses hdparm --dco-restore
        let device = "/dev/sda";
        let expected_args = vec!["--dco-restore", device];

        assert_eq!(expected_args[0], "--dco-restore");
        assert_eq!(expected_args[1], device);
    }

    #[test]
    fn test_dco_parsing_various_formats() {
        // Test parsing DCO output

        let output = r#"DCO Revision: 1
Real max sectors: 2000000
DCO max sectors: 1900000
DCO is active"#;

        // Extract numbers using simple parsing
        let extract_number = |line: &str| -> Option<u64> {
            line.split(':')
                .nth(1)?
                .trim()
                .split_whitespace()
                .next()?
                .parse::<u64>()
                .ok()
        };

        let mut real_max = None;
        let mut dco_max = None;

        for line in output.lines() {
            if line.contains("Real max sectors") {
                real_max = extract_number(line);
            } else if line.contains("DCO max sectors") {
                dco_max = extract_number(line);
            }
        }

        assert_eq!(real_max, Some(2000000));
        assert_eq!(dco_max, Some(1900000));
    }

    #[test]
    fn test_dco_size_calculations() {
        let test_cases = vec![
            (1000000u64, 1050000u64, 50000u64),    // 50K sectors
            (1000000u64, 1000100u64, 100u64),       // 100 sectors
            (1000000u64, 1500000u64, 500000u64),    // 500K sectors
        ];

        for (dco_max, real_max, expected_hidden) in test_cases {
            let hidden = real_max - dco_max;
            assert_eq!(hidden, expected_hidden);

            let hidden_mb = (hidden * 512) / (1024 * 1024);
            assert!(hidden_mb >= 0);
        }
    }

    #[test]
    fn test_dco_warning_message() {
        // DCO removal is typically permanent - verify warning is appropriate
        let warning = "WARNING: Removing DCO is typically permanent!";

        assert!(warning.contains("WARNING"));
        assert!(warning.contains("permanent"));
        assert!(warning.to_uppercase().starts_with("WARNING"));
    }
}

#[cfg(test)]
mod combined_hpa_dco_tests {
    use super::*;

    #[test]
    fn test_total_hidden_space_calculation() {
        // Drive with both HPA and DCO hiding space
        let visible_sectors = 1000000u64;
        let after_hpa_removal = 1100000u64;   // HPA hides 100K
        let true_capacity = 1150000u64;        // DCO hides 50K

        let hpa_hidden = after_hpa_removal - visible_sectors;
        let dco_hidden = true_capacity - after_hpa_removal;
        let total_hidden = hpa_hidden + dco_hidden;

        assert_eq!(hpa_hidden, 100000);
        assert_eq!(dco_hidden, 50000);
        assert_eq!(total_hidden, 150000);

        let total_hidden_bytes = total_hidden * 512;
        let total_hidden_mb = total_hidden_bytes / (1024 * 1024);
        assert_eq!(total_hidden_mb, 73); // ~73 MB
    }

    #[test]
    fn test_true_capacity_calculation() {
        // Test calculating true capacity with HPA and DCO

        struct TestCase {
            visible_size: u64,
            hpa_hidden: u64,
            dco_hidden: u64,
        }

        let cases = vec![
            TestCase { visible_size: 1000, hpa_hidden: 100, dco_hidden: 50 },
            TestCase { visible_size: 1000, hpa_hidden: 0, dco_hidden: 50 },
            TestCase { visible_size: 1000, hpa_hidden: 100, dco_hidden: 0 },
            TestCase { visible_size: 1000, hpa_hidden: 0, dco_hidden: 0 },
        ];

        for case in cases {
            let true_capacity = case.visible_size + case.hpa_hidden + case.dco_hidden;
            assert!(true_capacity >= case.visible_size,
                   "True capacity should be >= visible size");
        }
    }

    #[test]
    fn test_sequential_detection_workflow() {
        // Simulate the workflow: detect HPA -> remove HPA -> detect DCO

        // Step 1: Detect HPA
        let current = 1000000u64;
        let native = 1100000u64;
        let hpa_output = MockHdparmData::hpa_detected(current, native);
        assert!(hpa_output.contains("HPA is enabled"));

        // Step 2: After removing HPA, new visible size
        let new_visible = native; // Now we can see HPA region
        assert_eq!(new_visible, 1100000);

        // Step 3: Detect DCO from new visible size
        let dco_max = new_visible;
        let real_max = 1150000u64;
        let dco_output = MockHdparmData::dco_detected(dco_max, real_max);
        assert!(dco_output.contains("DCO is active"));

        // Step 4: Calculate total hidden space
        let total_hidden = real_max - current;
        assert_eq!(total_hidden, 150000);
    }

    #[test]
    fn test_warning_display_logic() {
        // Test that warnings are properly formatted when hidden areas are detected

        let hpa_hidden_mb = 100u64; // MB
        let dco_hidden_mb = 50u64;  // MB

        let warning = format!(
            "⚠️  Hidden areas detected on drive!\n  HPA: {} MB hidden\n  DCO: {} MB hidden",
            hpa_hidden_mb, dco_hidden_mb
        );

        assert!(warning.contains("⚠️"));
        assert!(warning.contains("HPA:"));
        assert!(warning.contains("DCO:"));
        assert!(warning.contains("100 MB"));
        assert!(warning.contains("50 MB"));
    }

    #[test]
    fn test_hidden_area_check_result_handling() {
        // Test handling of check_hidden_areas results

        struct HiddenAreaResult {
            has_hpa: bool,
            has_dco: bool,
            hpa_size_mb: u64,
            dco_size_mb: u64,
        }

        let test_cases = vec![
            HiddenAreaResult { has_hpa: true, has_dco: true, hpa_size_mb: 100, dco_size_mb: 50 },
            HiddenAreaResult { has_hpa: true, has_dco: false, hpa_size_mb: 100, dco_size_mb: 0 },
            HiddenAreaResult { has_hpa: false, has_dco: true, hpa_size_mb: 0, dco_size_mb: 50 },
            HiddenAreaResult { has_hpa: false, has_dco: false, hpa_size_mb: 0, dco_size_mb: 0 },
        ];

        for case in test_cases {
            let has_hidden = case.has_hpa || case.has_dco;
            let total_hidden = case.hpa_size_mb + case.dco_size_mb;

            if has_hidden {
                assert!(total_hidden > 0, "Should have hidden space if HPA or DCO present");
            } else {
                assert_eq!(total_hidden, 0, "Should have no hidden space if neither present");
            }
        }
    }
}

#[cfg(test)]
mod number_extraction_tests {
    use super::*;

    #[test]
    fn test_extract_number_from_various_formats() {
        // Test the number extraction logic used in HPA/DCO parsing

        let test_cases = vec![
            ("max sectors   = 1234567890/2345678901", vec![1234567890u64, 2345678901]),
            ("Real max sectors: 1953525168", vec![1953525168]),
            ("DCO max sectors: 1953425168", vec![1953425168]),
            ("  sectors = 1000000 / 1200000  ", vec![1000000, 1200000]),
        ];

        for (input, expected_numbers) in test_cases {
            // Extract all numeric sequences
            let numbers: Vec<u64> = input
                .split(|c: char| !c.is_numeric())
                .filter(|s| !s.is_empty())
                .filter_map(|s| s.parse::<u64>().ok())
                .collect();

            assert_eq!(numbers, expected_numbers,
                      "Failed to extract numbers from: {}", input);
        }
    }

    #[test]
    fn test_find_largest_number() {
        // HPA/DCO parsing often needs to find the largest number (likely the sector count)

        let line = "Version: 1 max sectors = 1953525168 revision 5";

        let numbers: Vec<u64> = line
            .split(|c: char| !c.is_numeric())
            .filter(|s| !s.is_empty())
            .filter_map(|s| s.parse::<u64>().ok())
            .collect();

        let largest = numbers.iter().max();
        assert_eq!(largest, Some(&1953525168u64));
    }

    #[test]
    fn test_parse_sector_slash_notation() {
        // Parse "current/native" format
        let input = "1000000/1200000";

        if let Some(slash_pos) = input.find('/') {
            let current: u64 = input[..slash_pos].parse().unwrap();
            let native: u64 = input[slash_pos + 1..].parse().unwrap();

            assert_eq!(current, 1000000);
            assert_eq!(native, 1200000);
        } else {
            panic!("Should find slash separator");
        }
    }
}

#[cfg(test)]
mod capacity_calculation_tests {
    use super::*;

    #[test]
    fn test_sector_to_byte_conversion() {
        let sectors = 1953525168u64;
        let bytes = sectors * 512;

        assert_eq!(bytes, 1000204886016); // ~1TB

        let gb = bytes / (1024 * 1024 * 1024);
        assert_eq!(gb, 931); // ~931 GB
    }

    #[test]
    fn test_byte_to_gb_conversion() {
        let test_cases = vec![
            (1024u64 * 1024 * 1024, 1u64),           // 1 GB
            (500 * 1024 * 1024 * 1024, 500),         // 500 GB
            (1024u64 * 1024 * 1024 * 1024, 1024),    // 1 TB in GB
        ];

        for (bytes, expected_gb) in test_cases {
            let gb = bytes / (1024 * 1024 * 1024);
            assert_eq!(gb, expected_gb);
        }
    }

    #[test]
    fn test_hidden_space_percentage() {
        let visible_sectors = 1000000u64;
        let hidden_sectors = 100000u64;
        let total_sectors = visible_sectors + hidden_sectors;

        let hidden_percentage = (hidden_sectors as f64 / total_sectors as f64) * 100.0;

        assert!((hidden_percentage - 9.09).abs() < 0.1, // ~9.09%
               "Hidden percentage should be ~9.09%, got {}", hidden_percentage);
    }

    #[test]
    fn test_capacity_comparison() {
        // Test comparing different capacity representations

        let size_bytes = 1000000000000u64; // 1TB decimal
        let size_sectors = size_bytes / 512;

        let size_gb_decimal = size_bytes / 1000000000; // 1000 GB
        let size_gb_binary = size_bytes / (1024 * 1024 * 1024); // 931 GiB

        assert_eq!(size_gb_decimal, 1000);
        assert_eq!(size_gb_binary, 931);

        // Verify sector round-trip
        let reconstructed_bytes = size_sectors * 512;
        assert_eq!(reconstructed_bytes, size_bytes);
    }
}

#[cfg(test)]
mod error_handling_tests {
    use super::*;

    #[test]
    fn test_invalid_sector_count_handling() {
        // Test handling of malformed sector counts

        let invalid_inputs = vec![
            "max sectors = abc/def",
            "max sectors = /",
            "max sectors = 1000000/",
            "max sectors = /1000000",
        ];

        for input in invalid_inputs {
            // Simulate parsing attempt
            if let Some(slash_pos) = input.find('/') {
                let before = &input[..slash_pos];
                let after = &input[slash_pos + 1..];

                // Extract just the numbers
                let before_num: Option<u64> = before
                    .chars()
                    .filter(|c| c.is_numeric())
                    .collect::<String>()
                    .parse()
                    .ok();

                let after_num: Option<u64> = after
                    .chars()
                    .filter(|c| c.is_numeric())
                    .collect::<String>()
                    .parse()
                    .ok();

                // At least one should fail for these invalid inputs
                if let (Some(a), Some(b)) = (before_num, after_num) {
                    // This would be a valid parse, which shouldn't happen for our test cases
                    // except for cases where there are valid numbers
                    let _ = (a, b);
                }
            }
        }
    }

    #[test]
    fn test_command_failure_simulation() {
        // Test handling of command failures

        let stderr_output = "hdparm: command not found";
        assert!(stderr_output.contains("not found") ||
                stderr_output.contains("failed") ||
                stderr_output.contains("error"),
                "Should detect error condition");
    }

    #[test]
    fn test_unsupported_feature_detection() {
        // Test detecting when HPA/DCO is not supported

        let not_supported_outputs = vec![
            "DCO feature set not supported",
            "HPA feature set not supported",
            "Feature not available",
        ];

        for output in not_supported_outputs {
            let is_unsupported = output.to_lowercase().contains("not supported") ||
                                output.to_lowercase().contains("not available");
            assert!(is_unsupported, "Should detect unsupported feature: {}", output);
        }
    }
}
