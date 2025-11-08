use crate::{DriveError, DriveResult};
pub(crate) use crate::HealthStatus;
use std::process::Command;
use std::collections::HashMap;
use std::io;
use std::io::Write;

#[derive(Debug, Clone)]
pub struct SMARTHealth {
    pub overall_health: HealthStatus,
    pub temperature_celsius: Option<u32>,
    pub power_on_hours: Option<u64>,
    pub power_cycle_count: Option<u64>,
    pub reallocated_sectors: Option<u64>,
    pub pending_sectors: Option<u64>,
    pub uncorrectable_errors: Option<u64>,
    pub wear_level: Option<u8>,  // For SSDs (percentage used)
    pub bad_block_count: Option<u64>,
    pub erase_fail_count: Option<u64>,
    pub program_fail_count: Option<u64>,
    pub critical_warning: Option<u8>,  // NVMe specific
    pub available_spare: Option<u8>,   // NVMe percentage
    pub media_errors: Option<u64>,     // NVMe
    pub attributes: HashMap<String, SMARTAttribute>,
}

#[derive(Debug, Clone)]
pub struct SMARTAttribute {
    pub id: u8,
    pub name: String,
    pub current: u8,
    pub worst: u8,
    pub threshold: u8,
    pub raw_value: u64,
    pub flags: String,
    pub failing_now: bool,
    pub failed_before: bool,
}

#[derive(Debug, Clone)]
pub struct TemperatureMonitor {
    pub current_celsius: u32,
    pub max_operating: u32,
    pub critical_threshold: u32,
    pub warning_threshold: u32,
}

pub struct SMARTMonitor;

impl SMARTMonitor {
    /// Parse temperature with robust unit detection and sanity checks
    pub fn parse_temperature_robust(value: u64, context: &str) -> Option<u32> {
        // Sanity check ranges for different units
        const CELSIUS_MAX: u64 = 100;  // Drives don't survive above 100°C
        const FAHRENHEIT_MIN: u64 = 32;
        const FAHRENHEIT_MAX: u64 = 212;
        const KELVIN_MIN: u64 = 273;
        const KELVIN_MAX: u64 = 373;

        let temp_celsius = if value >= KELVIN_MIN && value <= KELVIN_MAX {
            // Likely Kelvin (e.g., 313K = 40°C)
            println!("  🌡️  Detected Kelvin temperature: {}K", value);
            value - 273
        } else if value >= FAHRENHEIT_MIN && value <= FAHRENHEIT_MAX && value > CELSIUS_MAX {
            // Likely Fahrenheit (e.g., 104°F = 40°C)
            println!("  🌡️  Detected Fahrenheit temperature: {}°F", value);
            ((value - 32) * 5) / 9
        } else if value <= CELSIUS_MAX {
            // Likely Celsius
            value
        } else {
            // Invalid reading
            eprintln!("  ⚠️  Invalid temperature reading: {} (context: {})", value, context);
            eprintln!("     This reading is out of range for all known temperature units.");
            eprintln!("     Possible causes: bad SMART data, firmware bug, or sensor failure.");
            return None;
        };

        // Final sanity check
        if temp_celsius > CELSIUS_MAX {
            eprintln!("  ❌ Temperature {}°C exceeds physical limits! Ignoring.", temp_celsius);
            return None;
        }

        Some(temp_celsius as u32)
    }

    /// Get comprehensive SMART health information
    pub fn get_health(device_path: &str) -> DriveResult<SMARTHealth> {
        // Determine drive type and use appropriate method
        if device_path.contains("nvme") {
            Self::get_nvme_health(device_path)
        } else {
            Self::get_ata_health(device_path)
        }
    }

    /// Get ATA/SATA drive SMART health
    fn get_ata_health(device_path: &str) -> DriveResult<SMARTHealth> {
        let output = Command::new("smartctl")
            .args(["-A", "-H", "-i", device_path])
            .output()
            .map_err(|e| DriveError::SMARTReadFailed(format!("smartctl failed: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("Permission denied") {
                return Err(DriveError::SMARTReadFailed("Insufficient permissions".to_string()));
            }
        }

        let output_str = String::from_utf8_lossy(&output.stdout);
        Self::parse_ata_smart(&output_str)
    }

    /// Get NVMe drive SMART health - FIXED VERSION
    fn get_nvme_health(device_path: &str) -> DriveResult<SMARTHealth> {
        // Try nvme-cli first
        let output = Command::new("nvme")
            .args(["smart-log", device_path])
            .output();

        if let Ok(output) = output {
            if output.status.success() {
                let output_str = String::from_utf8_lossy(&output.stdout);
                return Self::parse_nvme_smart(&output_str, device_path);
            }
        }

        // Fall back to smartctl for NVMe
        let output = Command::new("smartctl")
            .args(["-A", "-H", device_path])
            .output()
            .map_err(|e| DriveError::SMARTReadFailed(format!("Failed to read NVMe SMART: {}", e)))?;

        let output_str = String::from_utf8_lossy(&output.stdout);
        Self::parse_nvme_smart_smartctl(&output_str)
    }

    /// Parse ATA SMART output
    pub(crate) fn parse_ata_smart(output: &str) -> DriveResult<SMARTHealth> {
        let mut health = SMARTHealth {
            overall_health: HealthStatus::Unknown,
            temperature_celsius: None,
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
            available_spare: None,
            media_errors: None,
            attributes: HashMap::new(),
        };

        // Parse overall health
        if output.contains("PASSED") {
            health.overall_health = HealthStatus::Good;
        } else if output.contains("FAILED") {
            health.overall_health = HealthStatus::Failed;
        }

        // Parse SMART attributes
        let mut in_attributes = false;
        for line in output.lines() {
            if line.contains("ID# ATTRIBUTE_NAME") {
                in_attributes = true;
                continue;
            }

            if !in_attributes {
                continue;
            }

            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 10 {
                continue;
            }

            if let Ok(id) = parts[0].parse::<u8>() {
                let name = parts[1].to_string();
                let current = parts[3].parse::<u8>().unwrap_or(0);
                let worst = parts[4].parse::<u8>().unwrap_or(0);
                let threshold = parts[5].parse::<u8>().unwrap_or(0);
                let raw_value = Self::parse_raw_value(parts[9]);

                let attribute = SMARTAttribute {
                    id,
                    name: name.clone(),
                    current,
                    worst,
                    threshold,
                    raw_value,
                    flags: parts[2].to_string(),
                    failing_now: parts[8] == "FAILING_NOW",
                    failed_before: parts[8] == "In_the_past",
                };

                // Extract specific values
                match name.as_str() {
                    "Temperature_Celsius" | "Airflow_Temperature_Cel" => {
                        health.temperature_celsius = Some(raw_value as u32);
                    }
                    "Power_On_Hours" => {
                        health.power_on_hours = Some(raw_value);
                    }
                    "Power_Cycle_Count" => {
                        health.power_cycle_count = Some(raw_value);
                    }
                    "Reallocated_Sector_Ct" => {
                        health.reallocated_sectors = Some(raw_value);
                    }
                    "Current_Pending_Sector" => {
                        health.pending_sectors = Some(raw_value);
                    }
                    "Offline_Uncorrectable" | "Uncorrectable_Error_Cnt" => {
                        health.uncorrectable_errors = Some(raw_value);
                    }
                    "Wear_Leveling_Count" | "SSD_Life_Left" => {
                        health.wear_level = Some(100 - current);
                    }
                    "Bad_Block_Count" | "Runtime_Bad_Block" => {
                        health.bad_block_count = Some(raw_value);
                    }
                    "Erase_Fail_Count" => {
                        health.erase_fail_count = Some(raw_value);
                    }
                    "Program_Fail_Count" | "Program_Fail_Cnt_Total" => {
                        health.program_fail_count = Some(raw_value);
                    }
                    _ => {}
                }

                health.attributes.insert(name, attribute);
            }
        }

        // Determine overall health based on critical attributes
        health.overall_health = Self::determine_health_status(&health);

        Ok(health)
    }

    /// Parse NVMe SMART output from nvme-cli - FIXED VERSION
    fn parse_nvme_smart(output: &str, device_path: &str) -> DriveResult<SMARTHealth> {
        let mut health = SMARTHealth {
            overall_health: HealthStatus::Unknown,
            temperature_celsius: None,
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
            available_spare: None,
            media_errors: None,
            attributes: HashMap::new(),
        };

        for line in output.lines() {
            let line_lower = line.to_lowercase();

            if line_lower.contains("critical_warning") {
                if let Some(value) = Self::extract_hex_value(&line_lower) {
                    health.critical_warning = Some(value as u8);
                }
            } else if line_lower.contains("temperature") && !line_lower.contains("sensor") {
                if let Some(value) = Self::extract_number(&line_lower) {
                    // FIXED: Use robust temperature parsing
                    health.temperature_celsius = Self::parse_temperature_robust(value, line);
                }
            } else if line_lower.contains("power_on_hours") || line_lower.contains("power on hours") {
                if let Some(value) = Self::extract_number(&line_lower) {
                    health.power_on_hours = Some(value);
                }
            } else if line_lower.contains("power_cycles") || line_lower.contains("power cycles") {
                if let Some(value) = Self::extract_number(&line_lower) {
                    health.power_cycle_count = Some(value);
                }
            } else if line_lower.contains("media_errors") || line_lower.contains("media and data integrity") {
                if let Some(value) = Self::extract_number(&line_lower) {
                    health.media_errors = Some(value);
                }
            } else if line_lower.contains("percentage_used") || line_lower.contains("percentage used") {
                if let Some(value) = Self::extract_number(&line_lower) {
                    health.wear_level = Some(value as u8);
                }
            } else if line_lower.contains("available_spare") && !line_lower.contains("threshold") {
                if let Some(value) = Self::extract_percentage(&line_lower) {
                    health.available_spare = Some(value as u8);
                }
            }
        }

        // Get additional health status
        let health_output = Command::new("nvme")
            .args(["id-ctrl", device_path])
            .output();

        if let Ok(output) = health_output {
            let output_str = String::from_utf8_lossy(&output.stdout);
            for line in output_str.lines() {
                if line.contains("Critical Warning") {
                    health.overall_health = if line.contains("0x00") {
                        HealthStatus::Good
                    } else {
                        HealthStatus::Warning
                    };
                }
            }
        }

        health.overall_health = Self::determine_health_status(&health);
        Ok(health)
    }

    /// Parse NVMe SMART from smartctl output
    pub(crate) fn parse_nvme_smart_smartctl(output: &str) -> DriveResult<SMARTHealth> {
        let mut health = SMARTHealth {
            overall_health: HealthStatus::Unknown,
            temperature_celsius: None,
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
            available_spare: None,
            media_errors: None,
            attributes: HashMap::new(),
        };

        // Parse smartctl NVMe output
        for line in output.lines() {
            if line.contains("SMART overall-health") {
                if line.contains("PASSED") {
                    health.overall_health = HealthStatus::Good;
                } else if line.contains("FAILED") {
                    health.overall_health = HealthStatus::Failed;
                }
            } else if line.contains("Temperature:") {
                if let Some(temp) = Self::extract_temperature(line) {
                    health.temperature_celsius = Some(temp);
                }
            } else if line.contains("Power On Hours:") {
                if let Some(hours) = Self::extract_number_from_line(line) {
                    health.power_on_hours = Some(hours);
                }
            } else if line.contains("Power Cycles:") {
                if let Some(cycles) = Self::extract_number_from_line(line) {
                    health.power_cycle_count = Some(cycles);
                }
            } else if line.contains("Media and Data Integrity Errors:") {
                if let Some(errors) = Self::extract_number_from_line(line) {
                    health.media_errors = Some(errors);
                }
            } else if line.contains("Percentage Used:") {
                if let Some(used) = Self::extract_percentage_from_line(line) {
                    health.wear_level = Some(used as u8);
                }
            } else if line.contains("Available Spare:") {
                if let Some(spare) = Self::extract_percentage_from_line(line) {
                    health.available_spare = Some(spare as u8);
                }
            }
        }

        health.overall_health = Self::determine_health_status(&health);
        Ok(health)
    }

    /// Determine overall health status based on attributes
    pub(crate) fn determine_health_status(health: &SMARTHealth) -> HealthStatus {
        // Critical checks
        if let Some(reallocated) = health.reallocated_sectors {
            if reallocated > 100 {
                return HealthStatus::Critical;
            } else if reallocated > 10 {
                return HealthStatus::Warning;
            }
        }

        if let Some(pending) = health.pending_sectors {
            if pending > 0 {
                return HealthStatus::Warning;
            }
        }

        if let Some(uncorrectable) = health.uncorrectable_errors {
            if uncorrectable > 0 {
                return HealthStatus::Critical;
            }
        }

        // SSD specific checks
        if let Some(wear) = health.wear_level {
            if wear > 90 {
                return HealthStatus::Critical;
            } else if wear > 80 {
                return HealthStatus::Warning;
            }
        }

        // NVMe specific checks
        if let Some(spare) = health.available_spare {
            if spare < 10 {
                return HealthStatus::Critical;
            } else if spare < 20 {
                return HealthStatus::Warning;
            }
        }

        if let Some(warning) = health.critical_warning {
            if warning > 0 {
                return HealthStatus::Warning;
            }
        }

        // Temperature check
        if let Some(temp) = health.temperature_celsius {
            if temp > 70 {
                return HealthStatus::Critical;
            } else if temp > 60 {
                return HealthStatus::Warning;
            }
        }

        HealthStatus::Good
    }

    /// Monitor temperature during operations - FIXED VERSION
    pub fn monitor_temperature(device_path: &str) -> DriveResult<TemperatureMonitor> {
        let health = Self::get_health(device_path)?;

        let current = match health.temperature_celsius {
            Some(temp) => {
                // Additional sanity check
                if temp > 100 {
                    eprintln!("⚠️  WARNING: Temperature reading {}°C is physically impossible!", temp);
                    eprintln!("   Possible SMART data corruption or sensor failure.");
                    eprintln!("   Using safe fallback temperature of 50°C for safety checks.");
                    50  // Safe fallback
                } else if temp > 85 {
                    eprintln!("⚠️  CRITICAL: Temperature {}°C is dangerously high!", temp);
                    temp
                } else {
                    temp
                }
            }
            None => {
                return Err(DriveError::SMARTReadFailed(
                    "Temperature sensor unavailable".to_string()
                ));
            }
        };

        // Set thresholds based on drive type
        let (warning, critical, max) = if device_path.contains("nvme") {
            (65, 75, 85)  // NVMe typically rated for higher temps
        } else {
            (55, 65, 70)   // SATA/SAS more conservative
        };

        Ok(TemperatureMonitor {
            current_celsius: current,
            max_operating: max,
            critical_threshold: critical,
            warning_threshold: warning,
        })
    }

    /// Check if it's safe to operate - ENHANCED VERSION
    pub fn check_safe_to_operate(device_path: &str) -> DriveResult<bool> {
        let health = match Self::get_health(device_path) {
            Ok(h) => h,
            Err(e) => {
                eprintln!("⚠️  Could not read drive health: {}", e);
                eprintln!("   Assuming safe to proceed (use --force to bypass)");
                return Ok(true);
            }
        };

        // Don't operate on failing drives
        if health.overall_health == HealthStatus::Failed {
            eprintln!("❌ Drive health status: FAILED");
            return Ok(false);
        }

        // Check temperature with better error handling
        if let Some(temp) = health.temperature_celsius {
            if temp > 100 {
                eprintln!("❌ Temperature reading {}°C is impossible - sensor may be broken", temp);
                eprintln!("   Recommend checking drive with manufacturer tools");
                print!("Continue without temperature monitoring? [y/N]: ");
                io::stdout().flush()?;

                let mut input = String::new();
                io::stdin().read_line(&mut input)?;

                if input.trim().to_lowercase() != "y" {
                    return Ok(false);
                }
            } else if temp > 70 {
                eprintln!("⚠️  WARNING: Drive temperature is {}°C (high)", temp);
                return Ok(false);
            }
        } else {
            eprintln!("ℹ️  Note: Temperature sensor unavailable");
        }

        // Check critical attributes
        if let Some(reallocated) = health.reallocated_sectors {
            if reallocated > 1000 {
                eprintln!("⚠️  WARNING: High reallocated sector count: {}", reallocated);
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Wait for drive to cool down if needed - FIXED VERSION with timeout
    pub fn wait_for_safe_temperature(device_path: &str, max_wait_seconds: u64) -> DriveResult<()> {
        use std::thread;
        use std::time::{Duration, Instant};

        println!("\n🌡️  Temperature Safety Check");

        let start = Instant::now();
        let max_duration = Duration::from_secs(max_wait_seconds);
        let mut consecutive_failures = 0;
        const MAX_FAILURES: u32 = 3;

        loop {
            match Self::monitor_temperature(device_path) {
                Ok(temp_mon) => {
                    consecutive_failures = 0;  // Reset failure counter

                    if temp_mon.current_celsius <= temp_mon.warning_threshold {
                        println!("✅ Drive temperature is safe: {}°C (threshold: {}°C)",
                                 temp_mon.current_celsius, temp_mon.warning_threshold);
                        return Ok(());
                    }

                    if temp_mon.current_celsius >= temp_mon.critical_threshold {
                        eprintln!("🔥 CRITICAL TEMPERATURE: {}°C!", temp_mon.current_celsius);
                    }

                    if start.elapsed() > max_duration {
                        eprintln!("\n❌ Timeout: Drive did not cool down within {} seconds", max_wait_seconds);
                        eprintln!("   Current: {}°C, Target: {}°C",
                                  temp_mon.current_celsius, temp_mon.warning_threshold);

                        print!("Continue anyway? [y/N]: ");
                        io::stdout().flush()?;

                        let mut input = String::new();
                        io::stdin().read_line(&mut input)?;

                        if input.trim().to_lowercase() == "y" {
                            eprintln!("⚠️  WARNING: Proceeding with high temperature!");
                            return Ok(());
                        }

                        return Err(DriveError::TemperatureExceeded(
                            format!("Drive temperature {}°C exceeds safe threshold {}°C",
                                    temp_mon.current_celsius, temp_mon.warning_threshold)
                        ));
                    }

                    println!("🌡️  Temperature: {}°C (waiting to cool below {}°C) - {}s elapsed",
                             temp_mon.current_celsius,
                             temp_mon.warning_threshold,
                             start.elapsed().as_secs());

                    thread::sleep(Duration::from_secs(30));
                }
                Err(e) => {
                    consecutive_failures += 1;
                    eprintln!("⚠️  Failed to read temperature (attempt {}/{}): {}",
                              consecutive_failures, MAX_FAILURES, e);

                    if consecutive_failures >= MAX_FAILURES {
                        eprintln!("\n❌ Temperature monitoring failed {} times consecutively", MAX_FAILURES);
                        eprintln!("   Possible causes:");
                        eprintln!("   - SMART not supported or disabled");
                        eprintln!("   - Drive firmware bug");
                        eprintln!("   - Sensor hardware failure");

                        print!("\nSkip temperature monitoring and continue? [y/N]: ");
                        io::stdout().flush()?;

                        let mut input = String::new();
                        io::stdin().read_line(&mut input)?;

                        if input.trim().to_lowercase() == "y" {
                            eprintln!("⚠️  WARNING: Temperature monitoring disabled!");
                            return Ok(());
                        }

                        return Err(DriveError::SMARTReadFailed(
                            "Temperature monitoring unavailable".to_string()
                        ));
                    }

                    thread::sleep(Duration::from_secs(10));
                }
            }
        }
    }

    /// Parse raw value from SMART attribute
    pub(crate) fn parse_raw_value(raw_str: &str) -> u64 {
        // Handle different raw value formats
        if let Ok(val) = raw_str.parse::<u64>() {
            return val;
        }

        // Handle hex values
        if raw_str.starts_with("0x") {
            if let Ok(val) = u64::from_str_radix(&raw_str[2..], 16) {
                return val;
            }
        }

        // Handle composite values (e.g., "100 (Min/Max 24/45)")
        if let Some(space_pos) = raw_str.find(' ') {
            if let Ok(val) = raw_str[..space_pos].parse::<u64>() {
                return val;
            }
        }

        0
    }

    /// Extract number from a line
    pub(crate) fn extract_number(line: &str) -> Option<u64> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        for part in parts.iter().rev() {
            if let Ok(num) = part.replace(",", "").parse::<u64>() {
                return Some(num);
            }
        }
        None
    }

    /// Extract hex value
    pub(crate) fn extract_hex_value(line: &str) -> Option<u64> {
        if let Some(hex_start) = line.find("0x") {
            let hex_str = &line[hex_start + 2..];
            let end = hex_str.find(|c: char| !c.is_ascii_hexdigit()).unwrap_or(hex_str.len());
            u64::from_str_radix(&hex_str[..end], 16).ok()
        } else {
            None
        }
    }

    /// Extract percentage value
    pub(crate) fn extract_percentage(line: &str) -> Option<u64> {
        if let Some(percent_pos) = line.find('%') {
            let before_percent = &line[..percent_pos];
            let number_start = before_percent.rfind(|c: char| !c.is_numeric()).map(|i| i + 1).unwrap_or(0);
            before_percent[number_start..].parse::<u64>().ok()
        } else {
            None
        }
    }

    /// Extract temperature from line
    pub(crate) fn extract_temperature(line: &str) -> Option<u32> {
        // Look for patterns like "45 Celsius" or "45 C"
        let parts: Vec<&str> = line.split_whitespace().collect();
        for i in 0..parts.len() - 1 {
            if let Ok(temp) = parts[i].parse::<u32>() {
                let next = parts[i + 1].to_lowercase();
                if next.contains("celsius") || next == "c" {
                    return Some(temp);
                }
            }
        }
        None
    }

    /// Extract number from a line with colon separator
    pub(crate) fn extract_number_from_line(line: &str) -> Option<u64> {
        if let Some(colon_pos) = line.find(':') {
            let after_colon = &line[colon_pos + 1..].trim();
            let number_end = after_colon.find(|c: char| !c.is_numeric() && c != ',').unwrap_or(after_colon.len());
            after_colon[..number_end].replace(",", "").parse::<u64>().ok()
        } else {
            None
        }
    }

    /// Extract percentage from line with colon
    pub(crate) fn extract_percentage_from_line(line: &str) -> Option<u64> {
        if let Some(colon_pos) = line.find(':') {
            let after_colon = &line[colon_pos + 1..].trim();
            if let Some(percent_pos) = after_colon.find('%') {
                let num_str = &after_colon[..percent_pos].trim();
                return num_str.parse::<u64>().ok();
            }
        }
        None
    }

    /// Get drive lifetime writes (TBW)
    pub fn get_lifetime_writes(device_path: &str) -> DriveResult<Option<u64>> {
        let output = Command::new("smartctl")
            .args(["-A", device_path])
            .output()
            .map_err(|e| DriveError::SMARTReadFailed(format!("Failed to read SMART: {}", e)))?;

        let output_str = String::from_utf8_lossy(&output.stdout);

        // Look for various TBW indicators
        for line in output_str.lines() {
            if line.contains("Total_LBAs_Written") ||
                line.contains("Lifetime_Writes") ||
                line.contains("Host_Writes") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 10 {
                    if let Ok(sectors) = parts[9].parse::<u64>() {
                        // Convert sectors to bytes (assuming 512 byte sectors)
                        return Ok(Some(sectors * 512));
                    }
                }
            }
        }

        // For NVMe, try different approach
        if device_path.contains("nvme") {
            let output = Command::new("nvme")
                .args(["smart-log", device_path])
                .output();

            if let Ok(output) = output {
                let output_str = String::from_utf8_lossy(&output.stdout);
                for line in output_str.lines() {
                    if line.contains("data_units_written") {
                        if let Some(units) = Self::extract_number(&line) {
                            // NVMe reports in 512KB units
                            return Ok(Some(units * 512 * 1024));
                        }
                    }
                }
            }
        }

        Ok(None)
    }

    /// Perform SMART self-test
    pub fn run_self_test(device_path: &str, test_type: SelfTestType) -> DriveResult<()> {
        let test_arg = match test_type {
            SelfTestType::Short => "short",
            SelfTestType::Extended => "long",
            SelfTestType::Conveyance => "conveyance",
        };

        println!("Starting {} self-test on {}...", test_arg, device_path);

        let output = Command::new("smartctl")
            .args(["-t", test_arg, device_path])
            .output()
            .map_err(|e| DriveError::SMARTReadFailed(format!("Failed to start self-test: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(DriveError::SMARTReadFailed(format!("Self-test failed: {}", stderr)));
        }

        println!("Self-test started. Use 'smartctl -l selftest {}' to check progress", device_path);
        Ok(())
    }

    /// Check self-test results
    pub fn check_self_test_results(device_path: &str) -> DriveResult<SelfTestResult> {
        let output = Command::new("smartctl")
            .args(["-l", "selftest", device_path])
            .output()
            .map_err(|e| DriveError::SMARTReadFailed(format!("Failed to read test results: {}", e)))?;

        let output_str = String::from_utf8_lossy(&output.stdout);

        // Parse most recent test result
        for line in output_str.lines() {
            if line.contains("Completed without error") {
                return Ok(SelfTestResult::Passed);
            } else if line.contains("Completed: read failure") {
                return Ok(SelfTestResult::Failed("Read failure detected".to_string()));
            } else if line.contains("In progress") {
                if let Some(percent) = Self::extract_percentage(&line) {
                    return Ok(SelfTestResult::InProgress(percent as u8));
                }
                return Ok(SelfTestResult::InProgress(0));
            }
        }

        Ok(SelfTestResult::NotRun)
    }

    /// Predict drive failure probability
    pub fn predict_failure(device_path: &str) -> DriveResult<FailurePrediction> {
        let health = Self::get_health(device_path)?;

        let mut risk_score = 0u32;
        let mut reasons = Vec::new();

        // Check reallocated sectors
        if let Some(reallocated) = health.reallocated_sectors {
            if reallocated > 100 {
                risk_score += 40;
                reasons.push(format!("High reallocated sector count: {}", reallocated));
            } else if reallocated > 10 {
                risk_score += 20;
                reasons.push(format!("Increasing reallocated sectors: {}", reallocated));
            }
        }

        // Check pending sectors
        if let Some(pending) = health.pending_sectors {
            if pending > 0 {
                risk_score += 30;
                reasons.push(format!("Pending sectors detected: {}", pending));
            }
        }

        // Check uncorrectable errors
        if let Some(errors) = health.uncorrectable_errors {
            if errors > 0 {
                risk_score += 50;
                reasons.push(format!("Uncorrectable errors: {}", errors));
            }
        }

        // SSD wear check
        if let Some(wear) = health.wear_level {
            if wear > 90 {
                risk_score += 60;
                reasons.push(format!("SSD wear level critical: {}%", wear));
            } else if wear > 80 {
                risk_score += 30;
                reasons.push(format!("SSD wear level high: {}%", wear));
            }
        }

        // NVMe spare check
        if let Some(spare) = health.available_spare {
            if spare < 10 {
                risk_score += 50;
                reasons.push(format!("Available spare critical: {}%", spare));
            } else if spare < 20 {
                risk_score += 25;
                reasons.push(format!("Available spare low: {}%", spare));
            }
        }

        let risk_level = if risk_score >= 80 {
            RiskLevel::Critical
        } else if risk_score >= 50 {
            RiskLevel::High
        } else if risk_score >= 30 {
            RiskLevel::Medium
        } else if risk_score > 0 {
            RiskLevel::Low
        } else {
            RiskLevel::None
        };

        Ok(FailurePrediction {
            risk_level: risk_level.clone(),
            risk_score: risk_score.min(100) as u8,
            estimated_days_remaining: Self::estimate_remaining_days(risk_score),
            failure_indicators: reasons,
            recommendation: Self::get_recommendation(risk_level),
        })
    }

    fn estimate_remaining_days(risk_score: u32) -> Option<u32> {
        match risk_score {
            0..=20 => None,  // No estimate for healthy drives
            21..=40 => Some(365),
            41..=60 => Some(90),
            61..=80 => Some(30),
            81..=100 => Some(7),
            _ => None,
        }
    }

    fn get_recommendation(risk_level: RiskLevel) -> String {
        match risk_level {
            RiskLevel::None => "Drive is healthy. Continue normal operations.".to_string(),
            RiskLevel::Low => "Monitor drive health regularly. Consider backup planning.".to_string(),
            RiskLevel::Medium => "Backup important data. Plan for replacement within 6 months.".to_string(),
            RiskLevel::High => "Backup immediately. Replace drive as soon as possible.".to_string(),
            RiskLevel::Critical => "URGENT: Drive failure imminent. Do not use for critical data.".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum SelfTestType {
    Short,
    Extended,
    Conveyance,
}

#[derive(Debug, Clone)]
pub enum SelfTestResult {
    Passed,
    Failed(String),
    InProgress(u8),  // Percentage complete
    NotRun,
}

#[derive(Debug, Clone)]
pub struct FailurePrediction {
    pub risk_level: RiskLevel,
    pub risk_score: u8,  // 0-100
    pub estimated_days_remaining: Option<u32>,
    pub failure_indicators: Vec<String>,
    pub recommendation: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RiskLevel {
    None,
    Low,
    Medium,
    High,
    Critical,
}
