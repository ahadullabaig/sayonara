/// Bad sector handling - skip bad sectors gracefully and log them
///
/// This module handles bad sectors that cannot be written, skipping them
/// while maintaining a record for verification and reporting.
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Default maximum bad sectors before aborting (1% of drive)
const DEFAULT_MAX_BAD_SECTORS: usize = 10000;

/// Bad sector handler with logging
pub struct BadSectorHandler {
    /// Set of bad sector offsets
    bad_sectors: Arc<Mutex<HashSet<u64>>>,

    /// Maximum allowed bad sectors before abort
    max_bad_sectors: usize,

    /// Log file path (optional)
    log_file: Option<PathBuf>,

    /// Device path for logging
    device_path: String,
}

impl BadSectorHandler {
    /// Create new bad sector handler
    pub fn new(device_path: impl Into<String>) -> Self {
        Self {
            bad_sectors: Arc::new(Mutex::new(HashSet::new())),
            max_bad_sectors: DEFAULT_MAX_BAD_SECTORS,
            log_file: None,
            device_path: device_path.into(),
        }
    }

    /// Set maximum bad sectors before abort
    pub fn with_max_bad_sectors(mut self, max: usize) -> Self {
        self.max_bad_sectors = max;
        self
    }

    /// Set log file path
    pub fn with_log_file(mut self, path: impl Into<PathBuf>) -> Self {
        self.log_file = Some(path.into());
        self
    }

    /// Default log file path for device
    pub fn default_log_file(device_name: &str) -> PathBuf {
        let sanitized = device_name.replace("/", "_").replace(".", "_");
        PathBuf::from(format!(
            "/var/log/sayonara-wipe/bad_sectors_{}.log",
            sanitized
        ))
    }

    /// Record a bad sector
    pub fn record_bad_sector(&self, sector_offset: u64, reason: impl Into<String>) -> Result<()> {
        let reason = reason.into();

        // Add to set
        let mut sectors = self.bad_sectors.lock().unwrap();
        sectors.insert(sector_offset);

        // Check if exceeded limit
        if sectors.len() > self.max_bad_sectors {
            return Err(anyhow::anyhow!(
                "Exceeded maximum bad sectors ({} > {}). Drive may be failing.",
                sectors.len(),
                self.max_bad_sectors
            ));
        }

        drop(sectors);

        // Log to file if configured
        if let Some(ref log_path) = self.log_file {
            self.append_to_log(log_path, sector_offset, &reason)?;
        }

        // Log via tracing
        tracing::warn!(
            device = %self.device_path,
            sector = sector_offset,
            reason = %reason,
            total_bad = self.bad_sector_count(),
            "Bad sector recorded"
        );

        Ok(())
    }

    /// Append entry to log file
    fn append_to_log(&self, log_path: &Path, sector: u64, reason: &str) -> Result<()> {
        // Ensure log directory exists
        if let Some(parent) = log_path.parent() {
            std::fs::create_dir_all(parent).context("Failed to create log directory")?;
        }

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)
            .context("Failed to open bad sector log")?;

        writeln!(
            file,
            "{} | Device: {} | Sector: {} | Reason: {}",
            chrono::Utc::now().to_rfc3339(),
            self.device_path,
            sector,
            reason
        )
        .context("Failed to write to bad sector log")?;

        Ok(())
    }

    /// Check if sector is known to be bad
    pub fn is_bad_sector(&self, sector_offset: u64) -> bool {
        let sectors = self.bad_sectors.lock().unwrap();
        sectors.contains(&sector_offset)
    }

    /// Get count of bad sectors
    pub fn bad_sector_count(&self) -> usize {
        let sectors = self.bad_sectors.lock().unwrap();
        sectors.len()
    }

    /// Get list of bad sectors
    pub fn get_bad_sectors(&self) -> Vec<u64> {
        let sectors = self.bad_sectors.lock().unwrap();
        sectors.iter().copied().collect()
    }

    /// Check if should abort due to too many bad sectors
    pub fn should_abort(&self) -> bool {
        self.bad_sector_count() > self.max_bad_sectors
    }

    /// Clear all recorded bad sectors
    pub fn clear(&self) {
        let mut sectors = self.bad_sectors.lock().unwrap();
        sectors.clear();
    }

    /// Generate bad sector report
    pub fn generate_report(&self) -> BadSectorReport {
        let sectors = self.bad_sectors.lock().unwrap();
        let mut sector_list: Vec<_> = sectors.iter().copied().collect();
        sector_list.sort_unstable();

        BadSectorReport {
            device_path: self.device_path.clone(),
            total_bad_sectors: sector_list.len(),
            max_bad_sectors: self.max_bad_sectors,
            percentage: (sector_list.len() as f64 / self.max_bad_sectors as f64) * 100.0,
            bad_sector_offsets: sector_list,
            log_file: self.log_file.clone(),
        }
    }
}

/// Write result from bad sector handling
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WriteResult {
    /// Write succeeded
    Success,

    /// Sector skipped due to being bad
    Skipped { sector: u64, reason: String },
}

/// Bad sector report for documentation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BadSectorReport {
    /// Device path
    pub device_path: String,

    /// Total number of bad sectors found
    pub total_bad_sectors: usize,

    /// Maximum allowed bad sectors
    pub max_bad_sectors: usize,

    /// Percentage of max reached
    pub percentage: f64,

    /// List of bad sector offsets
    pub bad_sector_offsets: Vec<u64>,

    /// Log file location
    pub log_file: Option<PathBuf>,
}

impl BadSectorReport {
    /// Format report as human-readable string
    pub fn format(&self) -> String {
        let mut output = String::new();
        output.push_str(&format!("Bad Sector Report for {}\n", self.device_path));
        output.push_str(&format!("{}\n", "=".repeat(60)));
        output.push_str(&format!("Total bad sectors: {}\n", self.total_bad_sectors));
        output.push_str(&format!("Maximum allowed: {}\n", self.max_bad_sectors));
        output.push_str(&format!("Percentage: {:.2}%\n", self.percentage));

        if let Some(ref log_file) = self.log_file {
            output.push_str(&format!("Log file: {}\n", log_file.display()));
        }

        if !self.bad_sector_offsets.is_empty() {
            output.push_str("\nBad sector offsets:\n");
            for (i, offset) in self.bad_sector_offsets.iter().enumerate() {
                output.push_str(&format!("  {}: {}\n", i + 1, offset));
                if i >= 99 {
                    output.push_str(&format!(
                        "  ... and {} more\n",
                        self.total_bad_sectors - 100
                    ));
                    break;
                }
            }
        }

        output
    }

    /// Check if device is likely failing
    pub fn is_device_failing(&self) -> bool {
        self.percentage > 50.0 || self.total_bad_sectors > 1000
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_bad_sector_handler_creation() {
        let handler = BadSectorHandler::new("/dev/sda");
        assert_eq!(handler.bad_sector_count(), 0);
        assert_eq!(handler.max_bad_sectors, DEFAULT_MAX_BAD_SECTORS);
    }

    #[test]
    fn test_record_bad_sector() {
        let handler = BadSectorHandler::new("/dev/sda");
        handler.record_bad_sector(1024, "I/O error").unwrap();

        assert_eq!(handler.bad_sector_count(), 1);
        assert!(handler.is_bad_sector(1024));
        assert!(!handler.is_bad_sector(2048));
    }

    #[test]
    fn test_multiple_bad_sectors() {
        let handler = BadSectorHandler::new("/dev/sda");

        for i in 0..10 {
            handler
                .record_bad_sector(i * 512, format!("Error {}", i))
                .unwrap();
        }

        assert_eq!(handler.bad_sector_count(), 10);

        let sectors = handler.get_bad_sectors();
        assert_eq!(sectors.len(), 10);
    }

    #[test]
    fn test_max_bad_sectors_limit() {
        let handler = BadSectorHandler::new("/dev/sda").with_max_bad_sectors(5);

        // Record 5 sectors - should succeed
        for i in 0..5 {
            handler.record_bad_sector(i * 512, "error").unwrap();
        }

        // 6th sector should fail
        let result = handler.record_bad_sector(6 * 512, "error");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Exceeded maximum"));
    }

    #[test]
    fn test_should_abort() {
        let handler = BadSectorHandler::new("/dev/sda").with_max_bad_sectors(3);

        assert!(!handler.should_abort());

        handler.record_bad_sector(0, "error").unwrap();
        handler.record_bad_sector(512, "error").unwrap();
        handler.record_bad_sector(1024, "error").unwrap();

        assert!(!handler.should_abort());

        // One more should trigger abort
        let _ = handler.record_bad_sector(1536, "error");
        assert!(handler.should_abort());
    }

    #[test]
    fn test_bad_sector_log_file() {
        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("bad_sectors.log");

        let handler = BadSectorHandler::new("/dev/sda").with_log_file(log_path.clone());

        handler.record_bad_sector(1024, "I/O timeout").unwrap();
        handler.record_bad_sector(2048, "Write error").unwrap();

        // Verify log file exists and has content
        assert!(log_path.exists());
        let content = std::fs::read_to_string(&log_path).unwrap();
        assert!(content.contains("/dev/sda"));
        assert!(content.contains("1024"));
        assert!(content.contains("2048"));
        assert!(content.contains("I/O timeout"));
    }

    #[test]
    fn test_clear_bad_sectors() {
        let handler = BadSectorHandler::new("/dev/sda");

        handler.record_bad_sector(1024, "error").unwrap();
        handler.record_bad_sector(2048, "error").unwrap();
        assert_eq!(handler.bad_sector_count(), 2);

        handler.clear();
        assert_eq!(handler.bad_sector_count(), 0);
    }

    #[test]
    fn test_bad_sector_report() {
        let handler = BadSectorHandler::new("/dev/sda").with_max_bad_sectors(100);

        handler.record_bad_sector(1024, "error").unwrap();
        handler.record_bad_sector(2048, "error").unwrap();

        let report = handler.generate_report();
        assert_eq!(report.total_bad_sectors, 2);
        assert_eq!(report.max_bad_sectors, 100);
        assert_eq!(report.percentage, 2.0);
        assert_eq!(report.bad_sector_offsets, vec![1024, 2048]);
    }

    #[test]
    fn test_report_format() {
        let handler = BadSectorHandler::new("/dev/sda");
        handler.record_bad_sector(1024, "error").unwrap();

        let report = handler.generate_report();
        let formatted = report.format();

        assert!(formatted.contains("/dev/sda"));
        assert!(formatted.contains("Total bad sectors: 1"));
        assert!(formatted.contains("1024"));
    }

    #[test]
    fn test_is_device_failing() {
        let handler = BadSectorHandler::new("/dev/sda").with_max_bad_sectors(10);

        // Add 6 bad sectors (60%)
        for i in 0..6 {
            handler.record_bad_sector(i * 512, "error").unwrap();
        }

        let report = handler.generate_report();
        assert!(report.is_device_failing());
    }

    #[test]
    fn test_default_log_file_path() {
        let path = BadSectorHandler::default_log_file("/dev/sda");
        assert!(path.to_string_lossy().contains("bad_sectors__dev_sda.log"));

        let path = BadSectorHandler::default_log_file("/dev/nvme0n1");
        assert!(path
            .to_string_lossy()
            .contains("bad_sectors__dev_nvme0n1.log"));
    }

    #[test]
    fn test_duplicate_bad_sectors() {
        let handler = BadSectorHandler::new("/dev/sda");

        // Record same sector twice
        handler.record_bad_sector(1024, "error 1").unwrap();
        handler.record_bad_sector(1024, "error 2").unwrap();

        // Should only count once
        assert_eq!(handler.bad_sector_count(), 1);
    }
}
