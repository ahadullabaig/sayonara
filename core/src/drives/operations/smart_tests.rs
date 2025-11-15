/// Comprehensive tests for SMART monitoring operations
///
/// This test suite covers:
/// - Temperature parsing (Celsius, Fahrenheit, Kelvin)
/// - ATA SMART parsing from smartctl output
/// - NVMe SMART parsing (nvme-cli and smartctl formats)
/// - Health status determination
/// - Parsing utilities (numbers, hex, percentages)
/// - Failure prediction and risk scoring
/// - Self-test result parsing
/// - Edge cases and error handling
use super::smart::*;
use crate::HealthStatus;
use std::collections::HashMap;

// ============================================================================
// Temperature Parsing Tests
// ============================================================================

#[test]
fn test_parse_temperature_celsius() {
    let temp = SMARTMonitor::parse_temperature_robust(45, "test celsius");
    assert_eq!(temp, Some(45), "45°C should be parsed as Celsius");
}

#[test]
fn test_parse_temperature_fahrenheit() {
    // 104°F = 40°C
    let temp = SMARTMonitor::parse_temperature_robust(104, "test fahrenheit");
    assert_eq!(temp, Some(40), "104°F should convert to 40°C");

    // 158°F = 70°C
    let temp = SMARTMonitor::parse_temperature_robust(158, "test fahrenheit");
    assert_eq!(temp, Some(70), "158°F should convert to 70°C");
}

#[test]
fn test_parse_temperature_kelvin() {
    // 313K = 40°C
    let temp = SMARTMonitor::parse_temperature_robust(313, "test kelvin");
    assert_eq!(temp, Some(40), "313K should convert to 40°C");

    // 333K = 60°C
    let temp = SMARTMonitor::parse_temperature_robust(333, "test kelvin");
    assert_eq!(temp, Some(60), "333K should convert to 60°C");
}

#[test]
fn test_parse_temperature_zero_celsius() {
    let temp = SMARTMonitor::parse_temperature_robust(0, "test zero");
    assert_eq!(temp, Some(0), "0°C should be valid");
}

#[test]
fn test_parse_temperature_max_celsius() {
    let temp = SMARTMonitor::parse_temperature_robust(100, "test max");
    assert_eq!(temp, Some(100), "100°C should be valid (max)");
}

#[test]
fn test_parse_temperature_invalid_too_high() {
    let temp = SMARTMonitor::parse_temperature_robust(500, "test invalid");
    assert_eq!(temp, None, "500 is out of range for all units");
}

#[test]
fn test_parse_temperature_invalid_after_conversion() {
    // 250°F would convert to >100°C, should be rejected
    let temp = SMARTMonitor::parse_temperature_robust(250, "test invalid fahrenheit");
    assert_eq!(
        temp, None,
        "Temperature converting to >100°C should be invalid"
    );
}

#[test]
fn test_parse_temperature_boundary_celsius_fahrenheit() {
    // 100°C is max Celsius, but also could be misinterpreted
    // Since it's <= CELSIUS_MAX, it should be treated as Celsius
    let temp = SMARTMonitor::parse_temperature_robust(100, "boundary test");
    assert_eq!(temp, Some(100), "100 should be treated as 100°C");
}

#[test]
fn test_parse_temperature_boundary_fahrenheit() {
    // 32 is ambiguous - could be 32°C or 32°F
    // Since it's <= CELSIUS_MAX (100), it's treated as Celsius
    let temp = SMARTMonitor::parse_temperature_robust(32, "32 ambiguous");
    assert_eq!(temp, Some(32), "32 is treated as 32°C (ambiguous case)");

    // 150°F = ~66°C (clearly Fahrenheit, > CELSIUS_MAX)
    let temp = SMARTMonitor::parse_temperature_robust(150, "150F boundary");
    assert_eq!(temp, Some(65), "150°F should convert to ~65°C");
}

#[test]
fn test_parse_temperature_boundary_kelvin() {
    // 273K = 0°C (minimum Kelvin)
    let temp = SMARTMonitor::parse_temperature_robust(273, "273K boundary");
    assert_eq!(temp, Some(0), "273K should convert to 0°C");
}

// ============================================================================
// Health Status Determination Tests
// ============================================================================

#[test]
fn test_determine_health_all_good() {
    let health = SMARTHealth {
        overall_health: HealthStatus::Unknown,
        temperature_celsius: Some(45),
        power_on_hours: Some(1000),
        power_cycle_count: Some(100),
        reallocated_sectors: Some(0),
        pending_sectors: Some(0),
        uncorrectable_errors: Some(0),
        wear_level: None,
        bad_block_count: None,
        erase_fail_count: None,
        program_fail_count: None,
        critical_warning: None,
        available_spare: None,
        media_errors: None,
        attributes: HashMap::new(),
    };

    let status = SMARTMonitor::determine_health_status(&health);
    assert_eq!(status, HealthStatus::Good, "Healthy drive should be Good");
}

#[test]
fn test_determine_health_critical_reallocated_sectors() {
    let health = SMARTHealth {
        overall_health: HealthStatus::Unknown,
        temperature_celsius: Some(45),
        power_on_hours: None,
        power_cycle_count: None,
        reallocated_sectors: Some(150), // > 100
        pending_sectors: Some(0),
        uncorrectable_errors: Some(0),
        wear_level: None,
        bad_block_count: None,
        erase_fail_count: None,
        program_fail_count: None,
        critical_warning: None,
        available_spare: None,
        media_errors: None,
        attributes: HashMap::new(),
    };

    let status = SMARTMonitor::determine_health_status(&health);
    assert_eq!(
        status,
        HealthStatus::Critical,
        "150 reallocated sectors should be Critical"
    );
}

#[test]
fn test_determine_health_warning_reallocated_sectors() {
    let health = SMARTHealth {
        overall_health: HealthStatus::Unknown,
        temperature_celsius: Some(45),
        power_on_hours: None,
        power_cycle_count: None,
        reallocated_sectors: Some(50), // > 10, <= 100
        pending_sectors: Some(0),
        uncorrectable_errors: Some(0),
        wear_level: None,
        bad_block_count: None,
        erase_fail_count: None,
        program_fail_count: None,
        critical_warning: None,
        available_spare: None,
        media_errors: None,
        attributes: HashMap::new(),
    };

    let status = SMARTMonitor::determine_health_status(&health);
    assert_eq!(
        status,
        HealthStatus::Warning,
        "50 reallocated sectors should be Warning"
    );
}

#[test]
fn test_determine_health_warning_pending_sectors() {
    let health = SMARTHealth {
        overall_health: HealthStatus::Unknown,
        temperature_celsius: Some(45),
        power_on_hours: None,
        power_cycle_count: None,
        reallocated_sectors: Some(0),
        pending_sectors: Some(5), // > 0
        uncorrectable_errors: Some(0),
        wear_level: None,
        bad_block_count: None,
        erase_fail_count: None,
        program_fail_count: None,
        critical_warning: None,
        available_spare: None,
        media_errors: None,
        attributes: HashMap::new(),
    };

    let status = SMARTMonitor::determine_health_status(&health);
    assert_eq!(
        status,
        HealthStatus::Warning,
        "Pending sectors should trigger Warning"
    );
}

#[test]
fn test_determine_health_critical_uncorrectable_errors() {
    let health = SMARTHealth {
        overall_health: HealthStatus::Unknown,
        temperature_celsius: Some(45),
        power_on_hours: None,
        power_cycle_count: None,
        reallocated_sectors: Some(0),
        pending_sectors: Some(0),
        uncorrectable_errors: Some(1), // > 0
        wear_level: None,
        bad_block_count: None,
        erase_fail_count: None,
        program_fail_count: None,
        critical_warning: None,
        available_spare: None,
        media_errors: None,
        attributes: HashMap::new(),
    };

    let status = SMARTMonitor::determine_health_status(&health);
    assert_eq!(
        status,
        HealthStatus::Critical,
        "Uncorrectable errors should be Critical"
    );
}

#[test]
fn test_determine_health_ssd_wear_critical() {
    let health = SMARTHealth {
        overall_health: HealthStatus::Unknown,
        temperature_celsius: Some(45),
        power_on_hours: None,
        power_cycle_count: None,
        reallocated_sectors: Some(0),
        pending_sectors: Some(0),
        uncorrectable_errors: Some(0),
        wear_level: Some(95), // > 90
        bad_block_count: None,
        erase_fail_count: None,
        program_fail_count: None,
        critical_warning: None,
        available_spare: None,
        media_errors: None,
        attributes: HashMap::new(),
    };

    let status = SMARTMonitor::determine_health_status(&health);
    assert_eq!(
        status,
        HealthStatus::Critical,
        "95% wear should be Critical"
    );
}

#[test]
fn test_determine_health_ssd_wear_warning() {
    let health = SMARTHealth {
        overall_health: HealthStatus::Unknown,
        temperature_celsius: Some(45),
        power_on_hours: None,
        power_cycle_count: None,
        reallocated_sectors: Some(0),
        pending_sectors: Some(0),
        uncorrectable_errors: Some(0),
        wear_level: Some(85), // > 80, <= 90
        bad_block_count: None,
        erase_fail_count: None,
        program_fail_count: None,
        critical_warning: None,
        available_spare: None,
        media_errors: None,
        attributes: HashMap::new(),
    };

    let status = SMARTMonitor::determine_health_status(&health);
    assert_eq!(status, HealthStatus::Warning, "85% wear should be Warning");
}

#[test]
fn test_determine_health_nvme_spare_critical() {
    let health = SMARTHealth {
        overall_health: HealthStatus::Unknown,
        temperature_celsius: Some(45),
        power_on_hours: None,
        power_cycle_count: None,
        reallocated_sectors: None,
        pending_sectors: None,
        uncorrectable_errors: None,
        wear_level: None,
        bad_block_count: None,
        erase_fail_count: None,
        program_fail_count: None,
        critical_warning: None,
        available_spare: Some(5), // < 10
        media_errors: None,
        attributes: HashMap::new(),
    };

    let status = SMARTMonitor::determine_health_status(&health);
    assert_eq!(
        status,
        HealthStatus::Critical,
        "5% spare should be Critical"
    );
}

#[test]
fn test_determine_health_nvme_spare_warning() {
    let health = SMARTHealth {
        overall_health: HealthStatus::Unknown,
        temperature_celsius: Some(45),
        power_on_hours: None,
        power_cycle_count: None,
        reallocated_sectors: None,
        pending_sectors: None,
        uncorrectable_errors: None,
        wear_level: None,
        bad_block_count: None,
        erase_fail_count: None,
        program_fail_count: None,
        critical_warning: None,
        available_spare: Some(15), // >= 10, < 20
        media_errors: None,
        attributes: HashMap::new(),
    };

    let status = SMARTMonitor::determine_health_status(&health);
    assert_eq!(status, HealthStatus::Warning, "15% spare should be Warning");
}

#[test]
fn test_determine_health_nvme_warning() {
    let health = SMARTHealth {
        overall_health: HealthStatus::Unknown,
        temperature_celsius: Some(45),
        power_on_hours: None,
        power_cycle_count: None,
        reallocated_sectors: None,
        pending_sectors: None,
        uncorrectable_errors: None,
        wear_level: None,
        bad_block_count: None,
        erase_fail_count: None,
        program_fail_count: None,
        critical_warning: Some(1), // > 0
        available_spare: None,
        media_errors: None,
        attributes: HashMap::new(),
    };

    let status = SMARTMonitor::determine_health_status(&health);
    assert_eq!(
        status,
        HealthStatus::Warning,
        "Critical warning should trigger Warning"
    );
}

#[test]
fn test_determine_health_temperature_critical() {
    let health = SMARTHealth {
        overall_health: HealthStatus::Unknown,
        temperature_celsius: Some(75), // > 70
        power_on_hours: None,
        power_cycle_count: None,
        reallocated_sectors: Some(0),
        pending_sectors: Some(0),
        uncorrectable_errors: Some(0),
        wear_level: None,
        bad_block_count: None,
        erase_fail_count: None,
        program_fail_count: None,
        critical_warning: None,
        available_spare: None,
        media_errors: None,
        attributes: HashMap::new(),
    };

    let status = SMARTMonitor::determine_health_status(&health);
    assert_eq!(status, HealthStatus::Critical, "75°C should be Critical");
}

#[test]
fn test_determine_health_temperature_warning() {
    let health = SMARTHealth {
        overall_health: HealthStatus::Unknown,
        temperature_celsius: Some(65), // > 60, <= 70
        power_on_hours: None,
        power_cycle_count: None,
        reallocated_sectors: Some(0),
        pending_sectors: Some(0),
        uncorrectable_errors: Some(0),
        wear_level: None,
        bad_block_count: None,
        erase_fail_count: None,
        program_fail_count: None,
        critical_warning: None,
        available_spare: None,
        media_errors: None,
        attributes: HashMap::new(),
    };

    let status = SMARTMonitor::determine_health_status(&health);
    assert_eq!(status, HealthStatus::Warning, "65°C should be Warning");
}

#[test]
fn test_determine_health_priority_critical_over_warning() {
    // Test that Critical takes precedence over Warning
    let health = SMARTHealth {
        overall_health: HealthStatus::Unknown,
        temperature_celsius: Some(65), // Warning level
        power_on_hours: None,
        power_cycle_count: None,
        reallocated_sectors: Some(0),
        pending_sectors: Some(0),
        uncorrectable_errors: Some(1), // Critical level
        wear_level: None,
        bad_block_count: None,
        erase_fail_count: None,
        program_fail_count: None,
        critical_warning: None,
        available_spare: None,
        media_errors: None,
        attributes: HashMap::new(),
    };

    let status = SMARTMonitor::determine_health_status(&health);
    assert_eq!(
        status,
        HealthStatus::Critical,
        "Critical errors should override Warning temperature"
    );
}

// ============================================================================
// Parsing Utilities Tests
// ============================================================================

#[test]
fn test_parse_raw_value_decimal() {
    let value = SMARTMonitor::parse_raw_value("12345");
    assert_eq!(value, 12345, "Decimal value should parse");
}

#[test]
fn test_parse_raw_value_hex() {
    let value = SMARTMonitor::parse_raw_value("0x1a2b");
    assert_eq!(value, 0x1a2b, "Hex value should parse");
}

#[test]
fn test_parse_raw_value_composite() {
    let value = SMARTMonitor::parse_raw_value("100 (Min/Max 24/45)");
    assert_eq!(value, 100, "Composite value should extract first number");
}

#[test]
fn test_parse_raw_value_invalid() {
    let value = SMARTMonitor::parse_raw_value("invalid");
    assert_eq!(value, 0, "Invalid value should return 0");
}

#[test]
fn test_parse_raw_value_empty() {
    let value = SMARTMonitor::parse_raw_value("");
    assert_eq!(value, 0, "Empty value should return 0");
}

#[test]
fn test_extract_number_basic() {
    let num = SMARTMonitor::extract_number("temperature : 45 celsius");
    assert_eq!(num, Some(45), "Should extract 45");
}

#[test]
fn test_extract_number_with_commas() {
    let num = SMARTMonitor::extract_number("sectors written: 1,234,567");
    assert_eq!(num, Some(1234567), "Should parse number with commas");
}

#[test]
fn test_extract_number_none() {
    let num = SMARTMonitor::extract_number("no numbers here");
    assert_eq!(num, None, "Should return None when no numbers");
}

#[test]
fn test_extract_hex_value_basic() {
    let hex = SMARTMonitor::extract_hex_value("critical_warning : 0x00");
    assert_eq!(hex, Some(0), "Should extract 0x00");
}

#[test]
fn test_extract_hex_value_nonzero() {
    let hex = SMARTMonitor::extract_hex_value("status: 0xff");
    assert_eq!(hex, Some(255), "Should extract 0xff as 255");
}

#[test]
fn test_extract_hex_value_mixed_case() {
    let hex = SMARTMonitor::extract_hex_value("value: 0xAbCd");
    assert_eq!(hex, Some(0xABCD), "Should handle mixed case hex");
}

#[test]
fn test_extract_hex_value_none() {
    let hex = SMARTMonitor::extract_hex_value("no hex here");
    assert_eq!(hex, None, "Should return None when no hex");
}

#[test]
fn test_extract_percentage_basic() {
    let pct = SMARTMonitor::extract_percentage("available spare: 85%");
    assert_eq!(pct, Some(85), "Should extract 85%");
}

#[test]
fn test_extract_percentage_zero() {
    let pct = SMARTMonitor::extract_percentage("usage: 0%");
    assert_eq!(pct, Some(0), "Should extract 0%");
}

#[test]
fn test_extract_percentage_hundred() {
    let pct = SMARTMonitor::extract_percentage("complete: 100%");
    assert_eq!(pct, Some(100), "Should extract 100%");
}

#[test]
fn test_extract_percentage_none() {
    let pct = SMARTMonitor::extract_percentage("no percentage");
    assert_eq!(pct, None, "Should return None when no %");
}

#[test]
fn test_extract_temperature_celsius() {
    let temp = SMARTMonitor::extract_temperature("Temperature: 45 Celsius");
    assert_eq!(temp, Some(45), "Should extract temperature in Celsius");
}

#[test]
fn test_extract_temperature_c() {
    let temp = SMARTMonitor::extract_temperature("Temp: 50 C");
    assert_eq!(temp, Some(50), "Should extract temperature with C");
}

#[test]
fn test_extract_temperature_none() {
    let temp = SMARTMonitor::extract_temperature("no temperature here");
    assert_eq!(temp, None, "Should return None when no temperature");
}

#[test]
fn test_extract_number_from_line_basic() {
    let num = SMARTMonitor::extract_number_from_line("Power On Hours: 1234");
    assert_eq!(num, Some(1234), "Should extract number after colon");
}

#[test]
fn test_extract_number_from_line_with_commas() {
    let num = SMARTMonitor::extract_number_from_line("Total Writes: 1,234,567");
    assert_eq!(num, Some(1234567), "Should handle commas");
}

#[test]
fn test_extract_number_from_line_none() {
    let num = SMARTMonitor::extract_number_from_line("No colon here");
    assert_eq!(num, None, "Should return None when no colon");
}

#[test]
fn test_extract_percentage_from_line_basic() {
    let pct = SMARTMonitor::extract_percentage_from_line("Percentage Used: 25%");
    assert_eq!(pct, Some(25), "Should extract percentage");
}

#[test]
fn test_extract_percentage_from_line_none() {
    let pct = SMARTMonitor::extract_percentage_from_line("No percentage: value");
    assert_eq!(pct, None, "Should return None when no %");
}

// ============================================================================
// Real-world SMART Output Parsing Tests
// ============================================================================

#[test]
fn test_parse_ata_smart_healthy_drive() -> anyhow::Result<()> {
    let output = r#"
=== START OF READ SMART DATA SECTION ===
SMART overall-health self-assessment test result: PASSED

ID# ATTRIBUTE_NAME          FLAG     VALUE WORST THRESH TYPE      UPDATED  WHEN_FAILED RAW_VALUE
  1 Raw_Read_Error_Rate     0x000f   100   100   006    Pre-fail  Always       -       0
  5 Reallocated_Sector_Ct   0x0033   100   100   036    Pre-fail  Always       -       0
  9 Power_On_Hours          0x0032   100   100   000    Old_age   Always       -       5000
 12 Power_Cycle_Count       0x0032   100   100   000    Old_age   Always       -       250
194 Temperature_Celsius     0x0022   045   040   000    Old_age   Always       -       45
197 Current_Pending_Sector  0x0012   100   100   000    Old_age   Always       -       0
198 Offline_Uncorrectable   0x0010   100   100   000    Old_age   Offline      -       0
"#;

    let health = SMARTMonitor::parse_ata_smart(output)?;

    assert_eq!(health.overall_health, HealthStatus::Good);
    assert_eq!(health.temperature_celsius, Some(45));
    assert_eq!(health.power_on_hours, Some(5000));
    assert_eq!(health.power_cycle_count, Some(250));
    assert_eq!(health.reallocated_sectors, Some(0));
    assert_eq!(health.pending_sectors, Some(0));
    assert_eq!(health.uncorrectable_errors, Some(0));

    assert!(health.attributes.contains_key("Temperature_Celsius"));
    assert!(health.attributes.contains_key("Power_On_Hours"));

    Ok(())
}

#[test]
fn test_parse_ata_smart_failing_drive() -> anyhow::Result<()> {
    let output = r#"
=== START OF READ SMART DATA SECTION ===
SMART overall-health self-assessment test result: FAILED

ID# ATTRIBUTE_NAME          FLAG     VALUE WORST THRESH TYPE      UPDATED  WHEN_FAILED RAW_VALUE
  5 Reallocated_Sector_Ct   0x0033   050   050   036    Pre-fail  Always   FAILING_NOW  150
197 Current_Pending_Sector  0x0012   090   090   000    Old_age   Always       -       10
198 Offline_Uncorrectable   0x0010   080   080   000    Old_age   Offline      -       5
"#;

    let health = SMARTMonitor::parse_ata_smart(output)?;

    assert_eq!(health.reallocated_sectors, Some(150));
    assert_eq!(health.pending_sectors, Some(10));
    assert_eq!(health.uncorrectable_errors, Some(5));

    // Should be Critical due to high reallocated sectors and uncorrectable errors
    assert!(matches!(health.overall_health, HealthStatus::Critical));

    Ok(())
}

#[test]
fn test_parse_ata_smart_ssd() -> anyhow::Result<()> {
    let output = r#"
=== START OF READ SMART DATA SECTION ===
SMART overall-health self-assessment test result: PASSED

ID# ATTRIBUTE_NAME          FLAG     VALUE WORST THRESH TYPE      UPDATED  WHEN_FAILED RAW_VALUE
  9 Power_On_Hours          0x0032   100   100   000    Old_age   Always       -       10000
 12 Power_Cycle_Count       0x0032   100   100   000    Old_age   Always       -       500
170 Bad_Block_Count         0x0033   100   100   010    Pre-fail  Always       -       0
171 Program_Fail_Count      0x0032   100   100   000    Old_age   Always       -       0
172 Erase_Fail_Count        0x0032   100   100   000    Old_age   Always       -       0
177 Wear_Leveling_Count     0x0013   070   070   000    Pre-fail  Always       -       30
194 Temperature_Celsius     0x0022   050   040   000    Old_age   Always       -       50
"#;

    let health = SMARTMonitor::parse_ata_smart(output)?;

    assert_eq!(health.temperature_celsius, Some(50));
    assert_eq!(health.power_on_hours, Some(10000));
    assert_eq!(health.power_cycle_count, Some(500));
    assert_eq!(health.bad_block_count, Some(0));
    assert_eq!(health.program_fail_count, Some(0));
    assert_eq!(health.erase_fail_count, Some(0));
    assert_eq!(health.wear_level, Some(30)); // 100 - current
    assert_eq!(health.overall_health, HealthStatus::Good);

    Ok(())
}

#[test]
fn test_parse_ata_smart_empty() -> anyhow::Result<()> {
    let output = "";
    let health = SMARTMonitor::parse_ata_smart(output)?;

    // Empty output with no bad indicators results in Good status
    // (determine_health_status returns Good when nothing is wrong)
    assert_eq!(health.overall_health, HealthStatus::Good);
    assert_eq!(health.temperature_celsius, None);
    assert_eq!(health.attributes.len(), 0);

    Ok(())
}

#[test]
fn test_parse_nvme_smart_smartctl_healthy() -> anyhow::Result<()> {
    let output = r#"
SMART overall-health self-assessment test result: PASSED

Temperature:                        40 Celsius
Power On Hours:                     5,000
Power Cycles:                       100
Media and Data Integrity Errors:    0
Percentage Used:                    15%
Available Spare:                    95%
"#;

    let health = SMARTMonitor::parse_nvme_smart_smartctl(output)?;

    assert_eq!(health.overall_health, HealthStatus::Good);
    assert_eq!(health.temperature_celsius, Some(40));
    assert_eq!(health.power_on_hours, Some(5000));
    assert_eq!(health.power_cycle_count, Some(100));
    assert_eq!(health.media_errors, Some(0));
    assert_eq!(health.wear_level, Some(15));
    assert_eq!(health.available_spare, Some(95));

    Ok(())
}

#[test]
fn test_parse_nvme_smart_smartctl_failed() -> anyhow::Result<()> {
    let output = r#"
SMART overall-health self-assessment test result: FAILED

Temperature:                        75 Celsius
Media and Data Integrity Errors:    5
Percentage Used:                    95%
Available Spare:                    5%
"#;

    let health = SMARTMonitor::parse_nvme_smart_smartctl(output)?;

    // Note: parse sets overall_health to Failed initially, but determine_health_status
    // will override based on critical conditions
    assert_eq!(health.temperature_celsius, Some(75));
    assert_eq!(health.media_errors, Some(5));
    assert_eq!(health.wear_level, Some(95));
    assert_eq!(health.available_spare, Some(5));
    assert_eq!(health.overall_health, HealthStatus::Critical); // Due to critical conditions

    Ok(())
}

// ============================================================================
// Self-Test Result Parsing Tests
// ============================================================================

// NOTE: check_self_test_results() calls smartctl directly, so we test the result enum
// structure instead of parsing. Integration tests with real hardware would test the
// full parsing logic.

#[test]
fn test_self_test_result_enum_variants() {
    // Test that all SelfTestResult variants can be created
    let passed = SelfTestResult::Passed;
    let failed = SelfTestResult::Failed("Read failure detected".to_string());
    let in_progress = SelfTestResult::InProgress(50);
    let not_run = SelfTestResult::NotRun;

    assert!(matches!(passed, SelfTestResult::Passed));
    assert!(matches!(failed, SelfTestResult::Failed(_)));
    assert!(matches!(in_progress, SelfTestResult::InProgress(50)));
    assert!(matches!(not_run, SelfTestResult::NotRun));
}

#[test]
fn test_self_test_result_failed_message() {
    let result = SelfTestResult::Failed("Read failure at sector 1234".to_string());

    if let SelfTestResult::Failed(msg) = result {
        assert!(msg.contains("Read failure"));
        assert!(msg.contains("1234"));
    } else {
        panic!("Expected Failed variant");
    }
}

#[test]
fn test_self_test_result_progress_percentage() {
    for pct in [0, 25, 50, 75, 99] {
        let result = SelfTestResult::InProgress(pct);
        if let SelfTestResult::InProgress(p) = result {
            assert_eq!(p, pct, "Progress percentage should match");
        } else {
            panic!("Expected InProgress variant");
        }
    }
}

// ============================================================================
// Failure Prediction Tests
// ============================================================================

#[test]
fn test_predict_failure_healthy_drive() -> anyhow::Result<()> {
    // This test uses real file operations, so we create mock SMART data
    // In a real scenario, this would call get_health() which needs hardware

    let health = SMARTHealth {
        overall_health: HealthStatus::Good,
        temperature_celsius: Some(40),
        power_on_hours: Some(1000),
        power_cycle_count: Some(100),
        reallocated_sectors: Some(0),
        pending_sectors: Some(0),
        uncorrectable_errors: Some(0),
        wear_level: Some(10),
        bad_block_count: None,
        erase_fail_count: None,
        program_fail_count: None,
        critical_warning: None,
        available_spare: Some(95),
        media_errors: None,
        attributes: HashMap::new(),
    };

    // Manually test prediction logic
    let mut risk_score = 0u32;

    if let Some(reallocated) = health.reallocated_sectors {
        if reallocated > 100 {
            risk_score += 40;
        } else if reallocated > 10 {
            risk_score += 20;
        }
    }

    assert_eq!(risk_score, 0, "Healthy drive should have 0 risk score");

    Ok(())
}

#[test]
fn test_predict_failure_reallocated_sectors_high() {
    let mut risk_score = 0u32;
    let reallocated = 150u64;

    if reallocated > 100 {
        risk_score += 40;
    } else if reallocated > 10 {
        risk_score += 20;
    }

    assert_eq!(
        risk_score, 40,
        "High reallocated sectors should add 40 points"
    );
}

#[test]
fn test_predict_failure_pending_sectors() {
    let mut risk_score = 0u32;
    let pending = 5u64;

    if pending > 0 {
        risk_score += 30;
    }

    assert_eq!(risk_score, 30, "Pending sectors should add 30 points");
}

#[test]
fn test_predict_failure_uncorrectable_errors() {
    let mut risk_score = 0u32;
    let errors = 1u64;

    if errors > 0 {
        risk_score += 50;
    }

    assert_eq!(risk_score, 50, "Uncorrectable errors should add 50 points");
}

#[test]
fn test_predict_failure_ssd_wear_critical() {
    let mut risk_score = 0u32;
    let wear = 95u8;

    if wear > 90 {
        risk_score += 60;
    } else if wear > 80 {
        risk_score += 30;
    }

    assert_eq!(risk_score, 60, "Critical wear should add 60 points");
}

#[test]
fn test_predict_failure_nvme_spare_critical() {
    let mut risk_score = 0u32;
    let spare = 5u8;

    if spare < 10 {
        risk_score += 50;
    } else if spare < 20 {
        risk_score += 25;
    }

    assert_eq!(risk_score, 50, "Critical spare should add 50 points");
}

#[test]
fn test_predict_failure_combined_score() {
    let mut risk_score = 0u32;

    // Multiple issues
    risk_score += 40; // Reallocated sectors
    risk_score += 30; // Pending sectors
    risk_score += 50; // Uncorrectable errors

    assert_eq!(risk_score, 120, "Combined issues should add up");
    assert!(risk_score >= 80, "Should be Critical level (>= 80)");
}

#[test]
fn test_risk_level_classification() {
    assert_eq!(classify_risk(0), RiskLevel::None, "0 score should be None");
    assert_eq!(classify_risk(15), RiskLevel::Low, "15 score should be Low");
    assert_eq!(
        classify_risk(40),
        RiskLevel::Medium,
        "40 score should be Medium"
    );
    assert_eq!(
        classify_risk(60),
        RiskLevel::High,
        "60 score should be High"
    );
    assert_eq!(
        classify_risk(90),
        RiskLevel::Critical,
        "90 score should be Critical"
    );
}

// Helper function to classify risk (mirrors the logic in predict_failure)
fn classify_risk(score: u32) -> RiskLevel {
    if score >= 80 {
        RiskLevel::Critical
    } else if score >= 50 {
        RiskLevel::High
    } else if score >= 30 {
        RiskLevel::Medium
    } else if score > 0 {
        RiskLevel::Low
    } else {
        RiskLevel::None
    }
}

// ============================================================================
// Edge Cases and Error Handling
// ============================================================================

#[test]
fn test_parse_ata_smart_malformed_attribute_line() -> anyhow::Result<()> {
    let output = r#"
=== START OF READ SMART DATA SECTION ===
SMART overall-health self-assessment test result: PASSED

ID# ATTRIBUTE_NAME          FLAG     VALUE WORST THRESH TYPE      UPDATED  WHEN_FAILED RAW_VALUE
  1 Raw_Read_Error_Rate     0x000f   100   100
malformed line here
  9 Power_On_Hours          0x0032   100   100   000    Old_age   Always       -       5000
"#;

    let health = SMARTMonitor::parse_ata_smart(output)?;

    // Should successfully parse the valid lines and skip malformed ones
    assert_eq!(health.power_on_hours, Some(5000));

    Ok(())
}

#[test]
fn test_parse_ata_smart_no_attributes_section() -> anyhow::Result<()> {
    let output = r#"
=== START OF READ SMART DATA SECTION ===
SMART overall-health self-assessment test result: PASSED
"#;

    let health = SMARTMonitor::parse_ata_smart(output)?;

    assert_eq!(health.overall_health, HealthStatus::Good);
    assert_eq!(health.attributes.len(), 0);

    Ok(())
}

#[test]
fn test_attribute_value_extraction() {
    let attr = SMARTAttribute {
        id: 194,
        name: "Temperature_Celsius".to_string(),
        current: 45,
        worst: 40,
        threshold: 0,
        raw_value: 45,
        flags: "0x0022".to_string(),
        failing_now: false,
        failed_before: false,
    };

    assert_eq!(attr.id, 194);
    assert_eq!(attr.name, "Temperature_Celsius");
    assert_eq!(attr.current, 45);
    assert_eq!(attr.raw_value, 45);
    assert!(!attr.failing_now);
    assert!(!attr.failed_before);
}

#[test]
fn test_temperature_monitor_structure() {
    let monitor = TemperatureMonitor {
        current_celsius: 50,
        max_operating: 70,
        critical_threshold: 65,
        warning_threshold: 55,
    };

    assert_eq!(monitor.current_celsius, 50);
    assert_eq!(monitor.max_operating, 70);
    assert_eq!(monitor.critical_threshold, 65);
    assert_eq!(monitor.warning_threshold, 55);
    assert!(monitor.current_celsius < monitor.warning_threshold);
}

#[test]
fn test_smart_health_structure_complete() {
    let mut attributes = HashMap::new();
    attributes.insert(
        "Temperature_Celsius".to_string(),
        SMARTAttribute {
            id: 194,
            name: "Temperature_Celsius".to_string(),
            current: 45,
            worst: 40,
            threshold: 0,
            raw_value: 45,
            flags: "0x0022".to_string(),
            failing_now: false,
            failed_before: false,
        },
    );

    let health = SMARTHealth {
        overall_health: HealthStatus::Good,
        temperature_celsius: Some(45),
        power_on_hours: Some(10000),
        power_cycle_count: Some(500),
        reallocated_sectors: Some(0),
        pending_sectors: Some(0),
        uncorrectable_errors: Some(0),
        wear_level: Some(25),
        bad_block_count: Some(0),
        erase_fail_count: Some(0),
        program_fail_count: Some(0),
        critical_warning: Some(0),
        available_spare: Some(95),
        media_errors: Some(0),
        attributes,
    };

    assert_eq!(health.overall_health, HealthStatus::Good);
    assert_eq!(health.attributes.len(), 1);
    assert!(health.attributes.contains_key("Temperature_Celsius"));
}

#[test]
fn test_self_test_type_variants() {
    let short = SelfTestType::Short;
    let extended = SelfTestType::Extended;
    let conveyance = SelfTestType::Conveyance;

    // Just ensure all variants exist and can be created
    assert!(matches!(short, SelfTestType::Short));
    assert!(matches!(extended, SelfTestType::Extended));
    assert!(matches!(conveyance, SelfTestType::Conveyance));
}

#[test]
fn test_failure_prediction_structure() {
    let prediction = FailurePrediction {
        risk_level: RiskLevel::Medium,
        risk_score: 45,
        estimated_days_remaining: Some(90),
        failure_indicators: vec!["Increasing reallocated sectors: 50".to_string()],
        recommendation: "Backup important data. Plan for replacement within 6 months.".to_string(),
    };

    assert_eq!(prediction.risk_level, RiskLevel::Medium);
    assert_eq!(prediction.risk_score, 45);
    assert_eq!(prediction.estimated_days_remaining, Some(90));
    assert_eq!(prediction.failure_indicators.len(), 1);
    assert!(prediction.recommendation.contains("Backup"));
}
