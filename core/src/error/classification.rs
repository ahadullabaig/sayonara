/// Error classification system for recovery strategy selection
///
/// This module categorizes errors to determine the appropriate recovery strategy.
/// Each error is classified into one of five categories that dictate retry behavior,
/// recovery mechanisms, and user feedback.
use crate::DriveError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// Classification of errors for recovery strategy selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ErrorClass {
    /// Transient errors - retry immediately (network glitch, device busy)
    /// Examples: temporary I/O errors, device busy, temporary timeout
    Transient,

    /// Recoverable errors - retry with different approach (wrong method, bad parameters)
    /// Examples: drive frozen (try unfreeze), authentication failed (try different method)
    Recoverable,

    /// Fatal errors - cannot recover, abort operation
    /// Examples: device not found, permission denied, unsupported operation
    Fatal,

    /// Environmental errors - wait for conditions to improve
    /// Examples: temperature too high, low power, insufficient resources
    Environmental,

    /// User interrupted - graceful shutdown requested
    UserInterrupted,
}

impl ErrorClass {
    /// Get human-readable description of error class
    pub fn description(&self) -> &'static str {
        match self {
            ErrorClass::Transient => "Temporary error that may resolve on retry",
            ErrorClass::Recoverable => "Error requiring alternative approach",
            ErrorClass::Fatal => "Unrecoverable error requiring abort",
            ErrorClass::Environmental => "Environmental condition needs improvement",
            ErrorClass::UserInterrupted => "Operation cancelled by user",
        }
    }

    /// Check if this error class allows retries
    pub fn allows_retry(&self) -> bool {
        matches!(
            self,
            ErrorClass::Transient | ErrorClass::Recoverable | ErrorClass::Environmental
        )
    }

    /// Get default maximum retry count for this error class
    pub fn default_max_retries(&self) -> u32 {
        match self {
            ErrorClass::Transient => 10,      // Fast retries for transient issues
            ErrorClass::Recoverable => 5,     // Moderate retries for recoverable issues
            ErrorClass::Environmental => 20,  // Many retries for environmental issues
            ErrorClass::Fatal => 0,           // No retries for fatal errors
            ErrorClass::UserInterrupted => 0, // No retries for user interrupts
        }
    }
}

impl fmt::Display for ErrorClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorClass::Transient => write!(f, "Transient"),
            ErrorClass::Recoverable => write!(f, "Recoverable"),
            ErrorClass::Fatal => write!(f, "Fatal"),
            ErrorClass::Environmental => write!(f, "Environmental"),
            ErrorClass::UserInterrupted => write!(f, "UserInterrupted"),
        }
    }
}

/// Context information about where and when an error occurred
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorContext {
    /// Operation name (e.g., "wipe_pass_5", "verify_sector", "trim_operation")
    pub operation: String,

    /// Device path (e.g., "/dev/sda", "/dev/nvme0n1")
    pub device_path: String,

    /// Byte offset where error occurred (if applicable)
    pub offset: Option<u64>,

    /// Timestamp when error occurred
    pub timestamp: DateTime<Utc>,

    /// Additional metadata (algorithm, pass number, etc.)
    pub metadata: HashMap<String, String>,
}

impl ErrorContext {
    /// Create a new error context
    pub fn new(operation: impl Into<String>, device_path: impl Into<String>) -> Self {
        Self {
            operation: operation.into(),
            device_path: device_path.into(),
            offset: None,
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        }
    }

    /// Create context for a specific pass operation
    pub fn for_pass(device_path: impl Into<String>, algorithm: &str, pass: usize) -> Self {
        let mut ctx = Self::new(format!("{}_pass_{}", algorithm, pass), device_path);
        ctx.metadata
            .insert("algorithm".to_string(), algorithm.to_string());
        ctx.metadata.insert("pass".to_string(), pass.to_string());
        ctx
    }

    /// Create context for a verification operation
    pub fn for_verification(device_path: impl Into<String>, offset: u64) -> Self {
        let mut ctx = Self::new("verification", device_path);
        ctx.offset = Some(offset);
        ctx
    }

    /// Add metadata to the context
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Set byte offset
    pub fn with_offset(mut self, offset: u64) -> Self {
        self.offset = Some(offset);
        self
    }
}

/// Classified error with recovery information
#[derive(Debug, Clone)]
pub struct ClassifiedError {
    /// Original error from the operation
    pub original: DriveError,

    /// Classification for recovery strategy
    pub class: ErrorClass,

    /// Context about where error occurred
    pub context: ErrorContext,

    /// Current retry attempt count
    pub retry_count: u32,

    /// Maximum allowed retries for this error
    pub max_retries: u32,

    /// Suggested recovery actions
    pub recovery_suggestions: Vec<String>,
}

impl ClassifiedError {
    /// Check if this error can be retried
    pub fn can_retry(&self) -> bool {
        self.retry_count < self.max_retries && self.class.allows_retry()
    }

    /// Increment retry count
    pub fn increment_retry(&mut self) {
        self.retry_count += 1;
    }

    /// Get remaining retry attempts
    pub fn remaining_retries(&self) -> u32 {
        self.max_retries.saturating_sub(self.retry_count)
    }

    /// Check if this is the last retry attempt
    pub fn is_last_retry(&self) -> bool {
        self.retry_count + 1 >= self.max_retries
    }
}

impl fmt::Display for ClassifiedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} error in {} on {}: {} (attempt {}/{})",
            self.class,
            self.context.operation,
            self.context.device_path,
            self.original,
            self.retry_count + 1,
            self.max_retries
        )
    }
}

/// Error classifier - determines error class and recovery strategy
pub struct ErrorClassifier {
    /// Custom retry limits per error class
    retry_limits: HashMap<ErrorClass, u32>,
}

impl ErrorClassifier {
    /// Create a new error classifier with default retry limits
    pub fn new() -> Self {
        let mut retry_limits = HashMap::new();
        retry_limits.insert(
            ErrorClass::Transient,
            ErrorClass::Transient.default_max_retries(),
        );
        retry_limits.insert(
            ErrorClass::Recoverable,
            ErrorClass::Recoverable.default_max_retries(),
        );
        retry_limits.insert(
            ErrorClass::Environmental,
            ErrorClass::Environmental.default_max_retries(),
        );
        retry_limits.insert(ErrorClass::Fatal, 0);
        retry_limits.insert(ErrorClass::UserInterrupted, 0);

        Self { retry_limits }
    }

    /// Set custom retry limit for an error class
    pub fn set_retry_limit(&mut self, class: ErrorClass, limit: u32) {
        self.retry_limits.insert(class, limit);
    }

    /// Classify a DriveError to determine recovery strategy
    pub fn classify(&self, error: DriveError, context: ErrorContext) -> ClassifiedError {
        let class = self.classify_error(&error, &context);
        let max_retries = self.retry_limits.get(&class).copied().unwrap_or(0);
        let recovery_suggestions = self.generate_recovery_suggestions(&error, class);

        ClassifiedError {
            original: error,
            class,
            context,
            retry_count: 0,
            max_retries,
            recovery_suggestions,
        }
    }

    /// Determine the error class based on the error type
    fn classify_error(&self, error: &DriveError, context: &ErrorContext) -> ErrorClass {
        match error {
            // User interruption - highest priority
            DriveError::Interrupted => ErrorClass::UserInterrupted,

            // Environmental errors - need time to resolve
            DriveError::TemperatureExceeded(_) => ErrorClass::Environmental,

            // Recoverable errors - need different approach
            DriveError::DriveFrozen(_) => ErrorClass::Recoverable,
            DriveError::UnlockFailed(_) => ErrorClass::Recoverable,
            DriveError::TRIMFailed(_) => ErrorClass::Recoverable,
            DriveError::CryptoEraseFailed(_) => ErrorClass::Recoverable,

            // Timeout - might be transient or hardware issue
            DriveError::Timeout(_msg) => {
                // If multiple timeouts on same operation, might be fatal
                if context
                    .metadata
                    .get("retry_count")
                    .and_then(|s| s.parse::<u32>().ok())
                    .unwrap_or(0)
                    > 5
                {
                    ErrorClass::Fatal
                } else {
                    ErrorClass::Transient
                }
            }

            // I/O errors - analyze the underlying error
            DriveError::IoError(io_err) => self.classify_io_error(io_err),

            // Hardware command failures - might be recoverable
            DriveError::HardwareCommandFailed(msg) => {
                if msg.contains("not supported") || msg.contains("invalid command") {
                    ErrorClass::Fatal
                } else {
                    ErrorClass::Transient
                }
            }

            // SMART read failures - usually transient
            DriveError::SMARTReadFailed(_) => ErrorClass::Transient,

            // Fatal errors - cannot recover
            DriveError::NotFound(_) => ErrorClass::Fatal,
            DriveError::PermissionDenied(_) => ErrorClass::Fatal,
            DriveError::Unsupported(_) => ErrorClass::Fatal,
        }
    }

    /// Classify I/O errors more granularly
    fn classify_io_error(&self, io_err: &std::io::Error) -> ErrorClass {
        use std::io::ErrorKind;

        match io_err.kind() {
            // Transient errors that may resolve on retry
            ErrorKind::Interrupted => ErrorClass::Transient,
            ErrorKind::WouldBlock => ErrorClass::Transient,
            ErrorKind::TimedOut => ErrorClass::Transient,
            ErrorKind::BrokenPipe => ErrorClass::Transient,
            ErrorKind::ConnectionReset => ErrorClass::Transient,

            // Fatal errors that won't resolve
            ErrorKind::NotFound => ErrorClass::Fatal,
            ErrorKind::PermissionDenied => ErrorClass::Fatal,
            ErrorKind::Unsupported => ErrorClass::Fatal,
            ErrorKind::InvalidInput => ErrorClass::Fatal,

            // Potentially recoverable with different approach
            ErrorKind::UnexpectedEof => ErrorClass::Recoverable,
            ErrorKind::WriteZero => ErrorClass::Recoverable,
            ErrorKind::Other => ErrorClass::Transient, // Conservative default

            // Default to transient for unknown errors
            _ => ErrorClass::Transient,
        }
    }

    /// Generate recovery suggestions based on error type and class
    fn generate_recovery_suggestions(&self, error: &DriveError, class: ErrorClass) -> Vec<String> {
        let mut suggestions = Vec::new();

        match (error, class) {
            (DriveError::DriveFrozen(_), _) => {
                suggestions.push("Try drive freeze mitigation strategies".to_string());
                suggestions.push("Attempt SATA link reset".to_string());
                suggestions.push("Consider PCIe hot-reset".to_string());
            }

            (DriveError::TemperatureExceeded(_), _) => {
                suggestions.push("Pause operation to allow cooling".to_string());
                suggestions.push("Check system ventilation".to_string());
                suggestions.push("Reduce I/O queue depth".to_string());
            }

            (DriveError::Timeout(_), ErrorClass::Transient) => {
                suggestions.push("Retry with exponential backoff".to_string());
                suggestions.push("Check device connection".to_string());
            }

            (DriveError::HardwareCommandFailed(_), ErrorClass::Transient) => {
                suggestions.push("Retry hardware command".to_string());
                suggestions.push("Check device health (SMART)".to_string());
                suggestions.push("Try alternative command sequence".to_string());
            }

            (DriveError::PermissionDenied(_), _) => {
                suggestions.push("Run with elevated privileges (sudo)".to_string());
                suggestions.push("Check device permissions".to_string());
            }

            (DriveError::UnlockFailed(_), _) => {
                suggestions.push("Verify encryption key/password".to_string());
                suggestions.push("Try alternative unlock methods".to_string());
            }

            (DriveError::TRIMFailed(_), _) => {
                suggestions.push("Verify TRIM support on device".to_string());
                suggestions.push("Try alternative discard methods".to_string());
                suggestions.push("Continue without TRIM if not critical".to_string());
            }

            _ => {
                suggestions.push(format!("Retry with {} strategy", class));
            }
        }

        suggestions
    }

    /// Determine if error should be retried based on classification
    pub fn should_retry(&self, error: &ClassifiedError) -> bool {
        error.can_retry()
    }
}

impl Default for ErrorClassifier {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_class_allows_retry() {
        assert!(ErrorClass::Transient.allows_retry());
        assert!(ErrorClass::Recoverable.allows_retry());
        assert!(ErrorClass::Environmental.allows_retry());
        assert!(!ErrorClass::Fatal.allows_retry());
        assert!(!ErrorClass::UserInterrupted.allows_retry());
    }

    #[test]
    fn test_error_class_default_retries() {
        assert_eq!(ErrorClass::Transient.default_max_retries(), 10);
        assert_eq!(ErrorClass::Recoverable.default_max_retries(), 5);
        assert_eq!(ErrorClass::Environmental.default_max_retries(), 20);
        assert_eq!(ErrorClass::Fatal.default_max_retries(), 0);
        assert_eq!(ErrorClass::UserInterrupted.default_max_retries(), 0);
    }

    #[test]
    fn test_classify_interrupted() {
        let classifier = ErrorClassifier::new();
        let context = ErrorContext::new("test_op", "/dev/sda");
        let error = DriveError::Interrupted;

        let classified = classifier.classify(error, context);
        assert_eq!(classified.class, ErrorClass::UserInterrupted);
        assert!(!classified.can_retry());
    }

    #[test]
    fn test_classify_temperature_exceeded() {
        let classifier = ErrorClassifier::new();
        let context = ErrorContext::new("test_op", "/dev/sda");
        let error = DriveError::TemperatureExceeded("65C".to_string());

        let classified = classifier.classify(error, context);
        assert_eq!(classified.class, ErrorClass::Environmental);
        assert!(classified.can_retry());
        assert_eq!(classified.max_retries, 20);
    }

    #[test]
    fn test_classify_drive_frozen() {
        let classifier = ErrorClassifier::new();
        let context = ErrorContext::new("test_op", "/dev/sda");
        let error = DriveError::DriveFrozen("Security frozen".to_string());

        let classified = classifier.classify(error, context);
        assert_eq!(classified.class, ErrorClass::Recoverable);
        assert!(classified.can_retry());
    }

    #[test]
    fn test_classify_permission_denied() {
        let classifier = ErrorClassifier::new();
        let context = ErrorContext::new("test_op", "/dev/sda");
        let error = DriveError::PermissionDenied("Root required".to_string());

        let classified = classifier.classify(error, context);
        assert_eq!(classified.class, ErrorClass::Fatal);
        assert!(!classified.can_retry());
    }

    #[test]
    fn test_classified_error_retry_logic() {
        let classifier = ErrorClassifier::new();
        let context = ErrorContext::new("test_op", "/dev/sda");
        let error = DriveError::Timeout("Command timeout".to_string());

        let mut classified = classifier.classify(error, context);
        assert_eq!(classified.retry_count, 0);
        assert!(classified.can_retry());
        assert_eq!(classified.remaining_retries(), 10);

        classified.increment_retry();
        assert_eq!(classified.retry_count, 1);
        assert_eq!(classified.remaining_retries(), 9);
    }

    #[test]
    fn test_error_context_builder() {
        let ctx = ErrorContext::for_pass("/dev/sda", "Gutmann", 5)
            .with_offset(1024)
            .with_metadata("test_key", "test_value");

        assert_eq!(ctx.operation, "Gutmann_pass_5");
        assert_eq!(ctx.device_path, "/dev/sda");
        assert_eq!(ctx.offset, Some(1024));
        assert_eq!(ctx.metadata.get("algorithm"), Some(&"Gutmann".to_string()));
        assert_eq!(ctx.metadata.get("pass"), Some(&"5".to_string()));
        assert_eq!(
            ctx.metadata.get("test_key"),
            Some(&"test_value".to_string())
        );
    }

    #[test]
    fn test_recovery_suggestions() {
        let classifier = ErrorClassifier::new();
        let context = ErrorContext::new("test_op", "/dev/sda");
        let error = DriveError::DriveFrozen("Frozen".to_string());

        let classified = classifier.classify(error, context);
        assert!(!classified.recovery_suggestions.is_empty());
        assert!(classified
            .recovery_suggestions
            .iter()
            .any(|s| s.contains("freeze mitigation")));
    }

    #[test]
    fn test_custom_retry_limits() {
        let mut classifier = ErrorClassifier::new();
        classifier.set_retry_limit(ErrorClass::Transient, 3);

        let context = ErrorContext::new("test_op", "/dev/sda");
        let error = DriveError::Timeout("Timeout".to_string());

        let classified = classifier.classify(error, context);
        assert_eq!(classified.max_retries, 3);
    }

    #[test]
    fn test_io_error_classification() {
        let classifier = ErrorClassifier::new();
        let context = ErrorContext::new("test_op", "/dev/sda");

        // Transient I/O errors
        let error = DriveError::IoError(std::io::Error::new(
            std::io::ErrorKind::Interrupted,
            "interrupted",
        ));
        let classified = classifier.classify(error, context.clone());
        assert_eq!(classified.class, ErrorClass::Transient);

        // Fatal I/O errors
        let error = DriveError::IoError(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "not found",
        ));
        let classified = classifier.classify(error, context);
        assert_eq!(classified.class, ErrorClass::Fatal);
    }
}
