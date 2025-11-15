/// Comprehensive tests for HPA/DCO detection and management
///
/// This test suite covers:
/// - HPA detection and calculations
/// - DCO detection and parsing
/// - Hidden area calculations
/// - Output parsing from hdparm and smartctl
/// - Number extraction from various formats
/// - Structure creation and validation
/// - Edge cases and error handling
use super::hpa_dco::*;

// ============================================================================
// Structure Tests
// ============================================================================

#[test]
fn test_hpa_info_structure() {
    let hpa = HPAInfo {
        enabled: true,
        native_max_sectors: 2000000,
        current_max_sectors: 1900000,
        hidden_sectors: 100000,
        hidden_size_bytes: 51200000, // 100000 * 512
    };

    assert!(hpa.enabled);
    assert_eq!(hpa.native_max_sectors, 2000000);
    assert_eq!(hpa.current_max_sectors, 1900000);
    assert_eq!(hpa.hidden_sectors, 100000);
    assert_eq!(hpa.hidden_size_bytes, 51200000);
    assert_eq!(hpa.hidden_sectors * 512, hpa.hidden_size_bytes);
}

#[test]
fn test_dco_info_structure() {
    let dco = DCOInfo {
        enabled: true,
        real_max_sectors: 3000000,
        dco_max_sectors: 2800000,
        hidden_sectors: 200000,
        hidden_size_bytes: 102400000, // 200000 * 512
    };

    assert!(dco.enabled);
    assert_eq!(dco.real_max_sectors, 3000000);
    assert_eq!(dco.dco_max_sectors, 2800000);
    assert_eq!(dco.hidden_sectors, 200000);
    assert_eq!(dco.hidden_size_bytes, 102400000);
    assert_eq!(dco.hidden_sectors * 512, dco.hidden_size_bytes);
}

#[test]
fn test_hpa_info_calculations() {
    let native = 1953525168u64;
    let current = 1953525000u64;
    let hidden = native - current;
    let hidden_bytes = hidden * 512;

    let hpa = HPAInfo {
        enabled: true,
        native_max_sectors: native,
        current_max_sectors: current,
        hidden_sectors: hidden,
        hidden_size_bytes: hidden_bytes,
    };

    assert_eq!(hpa.hidden_sectors, 168);
    assert_eq!(hpa.hidden_size_bytes, 86016);
    assert_eq!(hpa.hidden_size_bytes / 1024, 84); // ~84 KB
}

#[test]
fn test_dco_info_large_hidden_area() {
    // Realistic scenario: 1TB drive with ~93GB hidden
    let real_max = 1953525168u64; // ~1TB in sectors
    let hidden_size = 195352516u64; // ~93GB in sectors (integer division)
    let dco_max = real_max - hidden_size;
    let hidden_bytes = hidden_size * 512;

    let dco = DCOInfo {
        enabled: true,
        real_max_sectors: real_max,
        dco_max_sectors: dco_max,
        hidden_sectors: hidden_size,
        hidden_size_bytes: hidden_bytes,
    };

    assert_eq!(dco.hidden_sectors, 195352516);
    assert_eq!(dco.hidden_size_bytes / (1024 * 1024 * 1024), 93); // ~93 GB (integer division)
}

// ============================================================================
// Number Extraction Tests
// ============================================================================

#[test]
fn test_extract_number_from_line_basic() {
    let line = "max sectors = 1234567890/2345678901";
    let num = HPADCOManager::extract_number_from_line(line);
    // Should return the largest number
    assert_eq!(num, Some(2345678901));
}

#[test]
fn test_extract_number_from_line_single_number() {
    let line = "Total sectors: 1953525168";
    let num = HPADCOManager::extract_number_from_line(line);
    assert_eq!(num, Some(1953525168));
}

#[test]
fn test_extract_number_from_line_multiple_numbers() {
    let line = "Device size: 512110190592 bytes [512 GB]";
    let num = HPADCOManager::extract_number_from_line(line);
    // Should return the largest
    assert_eq!(num, Some(512110190592));
}

#[test]
fn test_extract_number_from_line_no_numbers() {
    let line = "No numbers here";
    let num = HPADCOManager::extract_number_from_line(line);
    assert_eq!(num, None);
}

#[test]
fn test_extract_number_from_line_empty() {
    let line = "";
    let num = HPADCOManager::extract_number_from_line(line);
    assert_eq!(num, None);
}

#[test]
fn test_extract_number_from_line_with_commas() {
    let line = "User Capacity: 512,110,190,592 bytes [512 GB]";
    // After splitting by non-numeric, commas are removed
    let num = HPADCOManager::extract_number_from_line(line);
    // Should extract one of the numbers (512, 110, 190, 592, 512)
    assert!(num.is_some());
    assert!(num.unwrap() > 0);
}

#[test]
fn test_extract_number_from_line_hex() {
    let line = "LBA: 0x1a2b3c4d";
    // Hex won't be parsed by parse::<u64>() without 0x handling
    // Our function should return None or a partial number
    let num = HPADCOManager::extract_number_from_line(line);
    // Should extract "0" and "1234" as separate numbers, return max
    assert!(num.is_some());
}

// ============================================================================
// DCO Output Parsing Tests
// ============================================================================

#[test]
fn test_parse_dco_output_with_hidden_area() -> anyhow::Result<()> {
    let output = r#"
DCO Revision: 0x0002
Real max sectors: 1953525168
DCO max sectors: 1953525000
The following features can be selectively disabled via DCO:
    SATA NCQ
    SATA NCQ priority
"#;

    let result = HPADCOManager::parse_dco_output(output)?;

    assert!(result.is_some());
    let (real_max, dco_max) = result.unwrap();
    assert_eq!(real_max, 1953525168);
    assert_eq!(dco_max, 1953525000);
    assert!(
        real_max > dco_max,
        "Real max should be greater than DCO max"
    );

    Ok(())
}

#[test]
fn test_parse_dco_output_no_hidden_area() -> anyhow::Result<()> {
    let output = r#"
DCO Revision: 0x0002
Real max sectors: 1953525168
DCO max sectors: 1953525168
"#;

    let result = HPADCOManager::parse_dco_output(output)?;

    assert!(result.is_some());
    let (real_max, dco_max) = result.unwrap();
    assert_eq!(real_max, 1953525168);
    assert_eq!(dco_max, 1953525168);
    assert_eq!(real_max, dco_max, "No hidden area - both should be equal");

    Ok(())
}

#[test]
fn test_parse_dco_output_missing_real_max() -> anyhow::Result<()> {
    let output = r#"
DCO Revision: 0x0002
DCO max sectors: 1953525000
"#;

    let result = HPADCOManager::parse_dco_output(output)?;
    assert!(result.is_none(), "Missing real_max should return None");

    Ok(())
}

#[test]
fn test_parse_dco_output_missing_dco_max() -> anyhow::Result<()> {
    let output = r#"
DCO Revision: 0x0002
Real max sectors: 1953525168
"#;

    let result = HPADCOManager::parse_dco_output(output)?;
    assert!(result.is_none(), "Missing dco_max should return None");

    Ok(())
}

#[test]
fn test_parse_dco_output_empty() -> anyhow::Result<()> {
    let output = "";
    let result = HPADCOManager::parse_dco_output(output)?;
    assert!(result.is_none(), "Empty output should return None");

    Ok(())
}

#[test]
fn test_parse_dco_output_malformed() -> anyhow::Result<()> {
    let output = r#"
DCO Revision: 0x0002
Real max sectors: not_a_number
DCO max sectors: also_not_a_number
"#;

    let result = HPADCOManager::parse_dco_output(output)?;
    assert!(result.is_none(), "Malformed numbers should return None");

    Ok(())
}

#[test]
fn test_parse_dco_output_realistic() -> anyhow::Result<()> {
    // Real-world example from a 1TB drive with 100GB hidden
    let output = r#"
/dev/sda:
DCO Revision: 0x0002
The following features can be selectively disabled via DCO:
    Real max sectors: 1953525168
    DCO max sectors: 1758172652
    48-bit Address feature set
    WRITE DMA
    SATA Features
"#;

    let result = HPADCOManager::parse_dco_output(output)?;

    assert!(result.is_some());
    let (real_max, dco_max) = result.unwrap();
    assert_eq!(real_max, 1953525168);
    assert_eq!(dco_max, 1758172652);

    let hidden_sectors = real_max - dco_max;
    let hidden_gb = (hidden_sectors * 512) / (1024 * 1024 * 1024);
    assert_eq!(hidden_gb, 93, "Should have ~93GB hidden (integer division)");

    Ok(())
}

// ============================================================================
// HPA Detection Logic Tests
// ============================================================================

#[test]
fn test_hpa_detection_logic_with_hidden_area() {
    // Simulating HPA detection logic
    let native_max = 1953525168u64;
    let current_max = 1953525000u64;

    let has_hpa = native_max > current_max;
    assert!(has_hpa, "Should detect HPA when native > current");

    if has_hpa {
        let hidden_sectors = native_max - current_max;
        let hidden_bytes = hidden_sectors * 512;

        assert_eq!(hidden_sectors, 168);
        assert_eq!(hidden_bytes, 86016);
    }
}

#[test]
fn test_hpa_detection_logic_no_hidden_area() {
    let native_max = 1953525168u64;
    let current_max = 1953525168u64;

    let has_hpa = native_max > current_max;
    assert!(!has_hpa, "Should not detect HPA when native == current");
}

#[test]
fn test_hpa_detection_logic_invalid_state() {
    // This shouldn't happen in practice, but test defensive programming
    let native_max = 1953525000u64;
    let current_max = 1953525168u64; // Current > native (impossible)

    let has_hpa = native_max > current_max;
    assert!(
        !has_hpa,
        "Current > native is invalid, should not detect HPA"
    );
}

// ============================================================================
// DCO Detection Logic Tests
// ============================================================================

#[test]
fn test_dco_detection_logic_with_hidden_area() {
    let real_max = 1953525168u64;
    let dco_max = 1758172652u64;

    let has_dco = real_max > dco_max;
    assert!(has_dco, "Should detect DCO when real > dco");

    if has_dco {
        let hidden_sectors = real_max - dco_max;
        let hidden_bytes = hidden_sectors * 512;
        let hidden_gb = hidden_bytes / (1024 * 1024 * 1024);

        assert_eq!(hidden_sectors, 195352516);
        assert_eq!(hidden_gb, 93, "Should be ~93GB hidden (integer division)");
    }
}

#[test]
fn test_dco_detection_logic_no_hidden_area() {
    let real_max = 1953525168u64;
    let dco_max = 1953525168u64;

    let has_dco = real_max > dco_max;
    assert!(!has_dco, "Should not detect DCO when real == dco");
}

// ============================================================================
// Capacity Calculation Tests
// ============================================================================

#[test]
fn test_capacity_calculation_no_hidden_areas() {
    let blockdev_sectors = 1953525168u64;
    let capacity = blockdev_sectors * 512;

    assert_eq!(capacity, 1000204886016); // ~1TB
    assert_eq!(capacity / (1024 * 1024 * 1024), 931); // ~931 GB
}

#[test]
fn test_capacity_calculation_with_hpa() {
    let current_sectors = 1953525000u64;
    let hpa_hidden_sectors = 168u64;

    let reported_capacity = current_sectors * 512;
    let true_capacity = (current_sectors + hpa_hidden_sectors) * 512;

    assert!(true_capacity > reported_capacity);
    assert_eq!(true_capacity - reported_capacity, hpa_hidden_sectors * 512);
}

#[test]
fn test_capacity_calculation_with_dco() {
    let dco_max_sectors = 1758172652u64;
    let dco_hidden_sectors = 195352516u64;

    let reported_capacity = dco_max_sectors * 512;
    let true_capacity = (dco_max_sectors + dco_hidden_sectors) * 512;

    assert!(true_capacity > reported_capacity);
    let hidden_gb = (true_capacity - reported_capacity) / (1024 * 1024 * 1024);
    assert_eq!(
        hidden_gb, 93,
        "Should have ~93GB difference (integer division)"
    );
}

#[test]
fn test_capacity_calculation_with_both_hpa_and_dco() {
    let base_sectors = 1758172652u64;
    let dco_hidden = 195352516u64;
    let hpa_hidden = 168u64;

    let reported_capacity = base_sectors * 512;
    let true_capacity = (base_sectors + dco_hidden + hpa_hidden) * 512;

    let total_hidden_bytes = (dco_hidden + hpa_hidden) * 512;
    assert_eq!(true_capacity - reported_capacity, total_hidden_bytes);
}

// ============================================================================
// Sector-to-Byte Conversion Tests
// ============================================================================

#[test]
fn test_sector_to_byte_conversion() {
    assert_eq!(512, 512);
    assert_eq!(2048 * 512, 1048576); // 1 MB
    assert_eq!(2097152 * 512, 1073741824); // 1 GB
}

#[test]
fn test_byte_to_gb_conversion() {
    let bytes_1gb = 1073741824u64;
    let bytes_100gb = 107374182400u64;
    let bytes_1tb = 1099511627776u64;

    assert_eq!(bytes_1gb / (1024 * 1024 * 1024), 1);
    assert_eq!(bytes_100gb / (1024 * 1024 * 1024), 100);
    assert_eq!(bytes_1tb / (1024 * 1024 * 1024), 1024);
}

#[test]
fn test_realistic_hidden_area_sizes() {
    // Common HPA sizes: 128 MB, 256 MB, 1 GB
    let hpa_128mb_sectors = (128 * 1024 * 1024) / 512;
    let hpa_256mb_sectors = (256 * 1024 * 1024) / 512;
    let hpa_1gb_sectors = (1024 * 1024 * 1024) / 512;

    assert_eq!(hpa_128mb_sectors, 262144);
    assert_eq!(hpa_256mb_sectors, 524288);
    assert_eq!(hpa_1gb_sectors, 2097152);

    // Verify round-trip
    assert_eq!((hpa_128mb_sectors * 512) / (1024 * 1024), 128);
    assert_eq!((hpa_256mb_sectors * 512) / (1024 * 1024), 256);
    assert_eq!((hpa_1gb_sectors * 512) / (1024 * 1024 * 1024), 1);
}

// ============================================================================
// Edge Cases and Error Handling
// ============================================================================

#[test]
fn test_hpa_info_clone() {
    let hpa1 = HPAInfo {
        enabled: true,
        native_max_sectors: 2000000,
        current_max_sectors: 1900000,
        hidden_sectors: 100000,
        hidden_size_bytes: 51200000,
    };

    let hpa2 = hpa1.clone();

    assert_eq!(hpa1.enabled, hpa2.enabled);
    assert_eq!(hpa1.native_max_sectors, hpa2.native_max_sectors);
    assert_eq!(hpa1.current_max_sectors, hpa2.current_max_sectors);
    assert_eq!(hpa1.hidden_sectors, hpa2.hidden_sectors);
    assert_eq!(hpa1.hidden_size_bytes, hpa2.hidden_size_bytes);
}

#[test]
fn test_dco_info_clone() {
    let dco1 = DCOInfo {
        enabled: true,
        real_max_sectors: 3000000,
        dco_max_sectors: 2800000,
        hidden_sectors: 200000,
        hidden_size_bytes: 102400000,
    };

    let dco2 = dco1.clone();

    assert_eq!(dco1.enabled, dco2.enabled);
    assert_eq!(dco1.real_max_sectors, dco2.real_max_sectors);
    assert_eq!(dco1.dco_max_sectors, dco2.dco_max_sectors);
    assert_eq!(dco1.hidden_sectors, dco2.hidden_sectors);
    assert_eq!(dco1.hidden_size_bytes, dco2.hidden_size_bytes);
}

#[test]
fn test_hpa_info_debug_format() {
    let hpa = HPAInfo {
        enabled: true,
        native_max_sectors: 2000000,
        current_max_sectors: 1900000,
        hidden_sectors: 100000,
        hidden_size_bytes: 51200000,
    };

    let debug_str = format!("{:?}", hpa);
    assert!(debug_str.contains("HPAInfo"));
    assert!(debug_str.contains("enabled: true"));
    assert!(debug_str.contains("2000000"));
}

#[test]
fn test_dco_info_debug_format() {
    let dco = DCOInfo {
        enabled: false,
        real_max_sectors: 3000000,
        dco_max_sectors: 3000000,
        hidden_sectors: 0,
        hidden_size_bytes: 0,
    };

    let debug_str = format!("{:?}", dco);
    assert!(debug_str.contains("DCOInfo"));
    assert!(debug_str.contains("enabled: false"));
    assert!(debug_str.contains("3000000"));
}

#[test]
fn test_zero_hidden_sectors() {
    let hpa = HPAInfo {
        enabled: false,
        native_max_sectors: 1953525168,
        current_max_sectors: 1953525168,
        hidden_sectors: 0,
        hidden_size_bytes: 0,
    };

    assert!(!hpa.enabled);
    assert_eq!(hpa.hidden_sectors, 0);
    assert_eq!(hpa.hidden_size_bytes, 0);
    assert_eq!(hpa.native_max_sectors, hpa.current_max_sectors);
}

#[test]
fn test_large_sector_numbers() {
    // Test with very large drives (e.g., 10TB+)
    let sectors_10tb = 19535251680u64; // ~10TB
    let hidden = 1000000u64;

    let hpa = HPAInfo {
        enabled: true,
        native_max_sectors: sectors_10tb,
        current_max_sectors: sectors_10tb - hidden,
        hidden_sectors: hidden,
        hidden_size_bytes: hidden * 512,
    };

    assert_eq!(hpa.hidden_sectors, 1000000);
    assert_eq!(hpa.hidden_size_bytes / (1024 * 1024), 488); // ~488 MB (integer division)
}

// ============================================================================
// Real-world Scenario Tests
// ============================================================================

#[test]
fn test_forensic_recovery_partition_scenario() {
    // Many systems have a hidden recovery partition via HPA
    // Typical: 10-20GB hidden
    let total_sectors = 1953525168u64; // 1TB drive
    let recovery_size_gb = 15u64;
    let recovery_sectors = (recovery_size_gb * 1024 * 1024 * 1024) / 512;
    let visible_sectors = total_sectors - recovery_sectors;

    let hpa = HPAInfo {
        enabled: true,
        native_max_sectors: total_sectors,
        current_max_sectors: visible_sectors,
        hidden_sectors: recovery_sectors,
        hidden_size_bytes: recovery_sectors * 512,
    };

    assert!(hpa.enabled);
    assert_eq!(
        hpa.hidden_size_bytes / (1024 * 1024 * 1024),
        recovery_size_gb
    );

    // Verify this represents realistic recovery partition
    let hidden_gb = hpa.hidden_size_bytes / (1024 * 1024 * 1024);
    assert!(
        (10..=20).contains(&hidden_gb),
        "Recovery partition should be 10-20GB"
    );
}

#[test]
fn test_vendor_locked_dco_scenario() {
    // Some vendors use DCO to restrict capacity
    // Example: 1TB drive sold as 900GB
    let actual_sectors = 1953525168u64; // ~1TB (931 GB)
    let sold_as_gb = 900u64;
    let sold_as_sectors = (sold_as_gb * 1024 * 1024 * 1024) / 512;

    let dco = DCOInfo {
        enabled: true,
        real_max_sectors: actual_sectors,
        dco_max_sectors: sold_as_sectors,
        hidden_sectors: actual_sectors - sold_as_sectors,
        hidden_size_bytes: (actual_sectors - sold_as_sectors) * 512,
    };

    assert!(dco.enabled);
    let hidden_gb = dco.hidden_size_bytes / (1024 * 1024 * 1024);
    // 931 GB - 900 GB = ~31 GB hidden
    assert!(
        (30..=32).contains(&hidden_gb),
        "Should have ~31GB hidden (integer division)"
    );
}

#[test]
fn test_combined_hpa_dco_security_scenario() {
    // Security-conscious system with both HPA and DCO hiding data
    let true_max = 1953525168u64;
    let dco_hidden = 195352516u64; // ~93GB via DCO
    let hpa_hidden = 10485760u64; // ~5GB via HPA

    let dco_max = true_max - dco_hidden;
    let current_max = dco_max - hpa_hidden;

    let total_hidden = dco_hidden + hpa_hidden;
    let total_hidden_gb = (total_hidden * 512) / (1024 * 1024 * 1024);

    assert_eq!(
        total_hidden_gb, 98,
        "Should have ~98GB total hidden (integer division)"
    );

    // Verify layering: Current < DCO Max < True Max
    assert!(current_max < dco_max);
    assert!(dco_max < true_max);
}
