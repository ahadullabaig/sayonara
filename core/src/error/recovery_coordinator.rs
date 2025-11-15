/// Recovery coordinator - orchestrates all error recovery mechanisms
///
/// This module provides the main recovery orchestration layer that:
/// - Classifies errors and determines recovery strategy
/// - Manages checkpoints for resume capability
/// - Executes retry logic with appropriate backoff
/// - Applies recovery mechanisms (bad sector handling, self-healing, degraded mode)
/// - Provides circuit breaker protection
use super::checkpoint::{Checkpoint, CheckpointManager};
use super::classification::{ClassifiedError, ErrorClass, ErrorClassifier, ErrorContext};
use super::mechanisms::{
    AlternativeIO, BadSectorHandler, DegradedMode, DegradedModeManager, HealMethod, SelfHealer,
};
use super::retry::{CircuitBreaker, RetryConfig};
use crate::{DriveError, DriveResult, WipeConfig};
use anyhow::Result;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// Progress information for checkpointing
#[derive(Debug, Clone)]
pub struct Progress {
    /// Current pass number
    pub current_pass: usize,

    /// Bytes written so far
    pub bytes_written: u64,

    /// Algorithm-specific state (JSON-serializable)
    pub state: serde_json::Value,
}

/// Recovery action to take
#[derive(Debug, Clone)]
pub enum RecoveryAction {
    /// Retry operation after delay
    Retry { after: Duration },

    /// Skip this operation
    Skip { reason: String },

    /// Abort entire operation
    Abort { error: DriveError },

    /// Apply healing method
    Heal { method: HealMethod },

    /// Enter degraded mode
    Degrade { mode: DegradedMode },

    /// Try alternative I/O method
    AlternativeIO,
}

/// Resume state loaded from checkpoint
#[derive(Debug, Clone)]
pub struct ResumeState {
    /// Checkpoint that was loaded
    pub checkpoint: Checkpoint,

    /// Pass to resume from
    pub current_pass: usize,

    /// Bytes already written
    pub bytes_written: u64,

    /// Algorithm-specific state
    pub state: serde_json::Value,
}

/// Recovery coordinator - main orchestration
pub struct RecoveryCoordinator {
    /// Error classifier
    classifier: ErrorClassifier,

    /// Checkpoint manager
    checkpoint_manager: Arc<Mutex<CheckpointManager>>,

    /// Retry configuration
    retry_config: RetryConfig,

    /// Circuit breaker
    circuit_breaker: CircuitBreaker,

    /// Bad sector handler
    bad_sector_handler: Option<BadSectorHandler>,

    /// Self-healer
    self_healer: SelfHealer,

    /// Degraded mode manager
    degraded_mode: Arc<Mutex<DegradedModeManager>>,

    /// Alternative I/O manager
    alternative_io: Arc<Mutex<AlternativeIO>>,

    /// Device path
    device_path: String,

    /// Operation ID for this session
    operation_id: String,
}

impl RecoveryCoordinator {
    /// Create new recovery coordinator
    pub fn new(device_path: impl Into<String>, _config: &WipeConfig) -> Result<Self> {
        let device_path = device_path.into();
        let operation_id = uuid::Uuid::new_v4().to_string();

        let checkpoint_manager = Arc::new(Mutex::new(CheckpointManager::new(None)?));

        // Set up bad sector handler if needed
        let bad_sector_handler = Some(
            BadSectorHandler::new(&device_path)
                .with_log_file(BadSectorHandler::default_log_file(&device_path)),
        );

        Ok(Self {
            classifier: ErrorClassifier::new(),
            checkpoint_manager,
            retry_config: RetryConfig::new(),
            circuit_breaker: CircuitBreaker::default_device(),
            bad_sector_handler,
            self_healer: SelfHealer::new(),
            degraded_mode: Arc::new(Mutex::new(DegradedModeManager::new())),
            alternative_io: Arc::new(Mutex::new(AlternativeIO::new())),
            device_path,
            operation_id,
        })
    }

    /// Execute operation with recovery
    ///
    /// This wraps any fallible operation with full error recovery:
    /// - Retry logic
    /// - Circuit breaker
    /// - Error classification
    /// - Recovery mechanisms
    pub fn execute_with_recovery<F, T>(
        &self,
        operation_name: &str,
        context: ErrorContext,
        mut operation: F,
    ) -> DriveResult<T>
    where
        F: FnMut() -> DriveResult<T>,
    {
        let strategy = self.retry_config.get_strategy(ErrorClass::Transient);

        let mut attempt = 0;
        let mut last_classified_error = None;

        loop {
            // Check circuit breaker
            if self.circuit_breaker.is_open() {
                tracing::error!(
                    operation = operation_name,
                    device = %self.device_path,
                    "Circuit breaker is OPEN, failing fast"
                );
                return Err(last_classified_error
                    .map(|e: ClassifiedError| e.original)
                    .unwrap_or_else(|| {
                        DriveError::HardwareCommandFailed("Circuit breaker open".to_string())
                    }));
            }

            // Execute operation within circuit breaker
            match self
                .circuit_breaker
                .call(|| operation().map_err(|e| anyhow::anyhow!("{}", e)))
            {
                Ok(result) => {
                    if attempt > 0 {
                        tracing::info!(
                            operation = operation_name,
                            device = %self.device_path,
                            attempt = attempt + 1,
                            "Operation succeeded after retry"
                        );
                    }
                    return Ok(result);
                }
                Err(error) => {
                    // Convert anyhow::Error to DriveError
                    let drive_error = match error.downcast::<DriveError>() {
                        Ok(de) => de,
                        Err(e) => DriveError::HardwareCommandFailed(e.to_string()),
                    };

                    // Classify error
                    let mut classified = self.classifier.classify(drive_error, context.clone());
                    classified.retry_count = attempt;

                    tracing::warn!(
                        operation = operation_name,
                        device = %self.device_path,
                        error = %classified,
                        class = ?classified.class,
                        "Operation failed"
                    );

                    // Determine recovery action
                    let action = self.determine_recovery_action(&classified);

                    match action {
                        RecoveryAction::Retry { after } => {
                            if !strategy.should_retry(attempt, &classified) {
                                tracing::error!(
                                    operation = operation_name,
                                    attempt = attempt + 1,
                                    max_attempts = strategy.max_attempts(),
                                    "Max retry attempts reached"
                                );
                                return Err(classified.original);
                            }

                            tracing::info!(
                                operation = operation_name,
                                attempt = attempt + 1,
                                delay_ms = after.as_millis(),
                                remaining = classified.remaining_retries(),
                                "Retrying after delay"
                            );

                            thread::sleep(after);
                            attempt += 1;
                            last_classified_error = Some(classified);
                            continue;
                        }

                        RecoveryAction::Skip { reason } => {
                            tracing::warn!(
                                operation = operation_name,
                                reason = %reason,
                                "Skipping operation"
                            );
                            return Err(classified.original);
                        }

                        RecoveryAction::Abort { error } => {
                            tracing::error!(
                                operation = operation_name,
                                error = %error,
                                "Aborting operation"
                            );
                            return Err(error);
                        }

                        RecoveryAction::Heal { method } => {
                            tracing::info!(
                                method = ?method,
                                "Attempting self-healing before retry"
                            );

                            if let Err(e) = self.self_healer.heal(&self.device_path, method) {
                                tracing::error!(error = %e, "Self-healing failed");
                            } else {
                                tracing::info!("Self-healing succeeded, retrying operation");
                                attempt += 1;
                                continue;
                            }

                            return Err(classified.original);
                        }

                        RecoveryAction::Degrade { mode } => {
                            tracing::warn!(mode = ?mode, "Entering degraded mode");
                            let mut degraded = self.degraded_mode.lock().unwrap();
                            degraded.enable(mode);
                            return Err(classified.original);
                        }

                        RecoveryAction::AlternativeIO => {
                            tracing::info!("Attempting alternative I/O method");
                            // This is handled at call site
                            return Err(classified.original);
                        }
                    }
                }
            }
        }
    }

    /// Determine recovery action based on error
    fn determine_recovery_action(&self, error: &ClassifiedError) -> RecoveryAction {
        match error.class {
            ErrorClass::Transient => {
                let strategy = self.retry_config.get_strategy(error.class);
                let delay = strategy.next_delay(error.retry_count);
                RecoveryAction::Retry { after: delay }
            }

            ErrorClass::Recoverable => {
                // Try healing for recoverable errors
                if error.retry_count == 0 {
                    match &error.original {
                        DriveError::DriveFrozen(_) => RecoveryAction::Heal {
                            method: HealMethod::ResetDevice,
                        },
                        _ => {
                            let strategy = self.retry_config.get_strategy(error.class);
                            let delay = strategy.next_delay(error.retry_count);
                            RecoveryAction::Retry { after: delay }
                        }
                    }
                } else {
                    RecoveryAction::Abort {
                        error: error.original.clone(),
                    }
                }
            }

            ErrorClass::Environmental => {
                let strategy = self.retry_config.get_strategy(error.class);
                let delay = strategy.next_delay(error.retry_count);
                RecoveryAction::Retry { after: delay }
            }

            ErrorClass::Fatal => RecoveryAction::Abort {
                error: error.original.clone(),
            },

            ErrorClass::UserInterrupted => RecoveryAction::Abort {
                error: error.original.clone(),
            },
        }
    }

    /// Save checkpoint if needed
    pub fn maybe_checkpoint(
        &mut self,
        algorithm: &str,
        total_passes: usize,
        total_size: u64,
        progress: &Progress,
    ) -> Result<()> {
        let mut manager = self.checkpoint_manager.lock().unwrap();

        if manager.should_save(progress.bytes_written) {
            let mut checkpoint = Checkpoint::new(
                &self.device_path,
                algorithm,
                &self.operation_id,
                total_passes,
                total_size,
            );

            checkpoint.update_progress(progress.current_pass, progress.bytes_written);
            checkpoint.state = progress.state.clone();

            manager.save(&checkpoint)?;

            tracing::debug!(
                device = %self.device_path,
                pass = progress.current_pass,
                bytes = progress.bytes_written,
                "Checkpoint saved"
            );
        }

        Ok(())
    }

    /// Load and resume from checkpoint
    pub fn resume_from_checkpoint(&self, algorithm: &str) -> Result<Option<ResumeState>> {
        let manager = self.checkpoint_manager.lock().unwrap();

        if let Some(checkpoint) = manager.load(&self.device_path, algorithm)? {
            tracing::info!(
                device = %self.device_path,
                algorithm = algorithm,
                pass = checkpoint.current_pass,
                bytes = checkpoint.bytes_written,
                "Resuming from checkpoint"
            );

            Ok(Some(ResumeState {
                current_pass: checkpoint.current_pass,
                bytes_written: checkpoint.bytes_written,
                state: checkpoint.state.clone(),
                checkpoint,
            }))
        } else {
            Ok(None)
        }
    }

    /// Delete checkpoint after successful completion
    pub fn delete_checkpoint(&self) -> Result<()> {
        let mut manager = self.checkpoint_manager.lock().unwrap();
        let deleted = manager.delete_by_device(&self.device_path, "*")?;

        if deleted > 0 {
            tracing::info!(
                device = %self.device_path,
                count = deleted,
                "Checkpoints deleted after successful completion"
            );
        }

        Ok(())
    }

    /// Get bad sector handler
    pub fn bad_sector_handler(&self) -> Option<&BadSectorHandler> {
        self.bad_sector_handler.as_ref()
    }

    /// Get bad sector handler (mutable)
    pub fn bad_sector_handler_mut(&mut self) -> Option<&mut BadSectorHandler> {
        self.bad_sector_handler.as_mut()
    }

    /// Get degraded mode manager
    pub fn degraded_mode(&self) -> Arc<Mutex<DegradedModeManager>> {
        Arc::clone(&self.degraded_mode)
    }

    /// Get alternative I/O manager
    pub fn alternative_io(&self) -> Arc<Mutex<AlternativeIO>> {
        Arc::clone(&self.alternative_io)
    }

    /// Reset circuit breaker
    pub fn reset_circuit_breaker(&self) {
        self.circuit_breaker.reset();
    }

    /// Get operation ID
    pub fn operation_id(&self) -> &str {
        &self.operation_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use tempfile::TempDir;

    fn create_test_coordinator() -> (RecoveryCoordinator, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test_checkpoints.db");

        let device_path = "/dev/sda".to_string();
        let operation_id = uuid::Uuid::new_v4().to_string();
        let checkpoint_manager = Arc::new(Mutex::new(
            CheckpointManager::new(Some(db_path.to_str().unwrap())).unwrap(),
        ));

        let coordinator = RecoveryCoordinator {
            classifier: ErrorClassifier::new(),
            checkpoint_manager,
            retry_config: RetryConfig::new(),
            circuit_breaker: CircuitBreaker::default_device(),
            bad_sector_handler: Some(BadSectorHandler::new(&device_path)),
            self_healer: SelfHealer::new(),
            degraded_mode: Arc::new(Mutex::new(DegradedModeManager::new())),
            alternative_io: Arc::new(Mutex::new(AlternativeIO::new())),
            device_path,
            operation_id,
        };

        (coordinator, temp_dir)
    }

    #[test]
    fn test_recovery_coordinator_creation() {
        let (coordinator, _temp) = create_test_coordinator();
        assert_eq!(coordinator.device_path, "/dev/sda");
    }

    #[test]
    fn test_execute_with_recovery_success() {
        let (coordinator, _temp) = create_test_coordinator();
        let context = ErrorContext::new("test_op", "/dev/sda");

        let result = coordinator
            .execute_with_recovery("test_operation", context, || Ok::<_, DriveError>(42));

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_execute_with_recovery_retry() {
        let (coordinator, _temp) = create_test_coordinator();
        let context = ErrorContext::new("test_op", "/dev/sda");

        let attempt_count = Arc::new(AtomicU32::new(0));
        let attempt_count_clone = Arc::clone(&attempt_count);

        let result = coordinator.execute_with_recovery("test_operation", context, || {
            let count = attempt_count_clone.fetch_add(1, Ordering::SeqCst);
            if count < 2 {
                Err(DriveError::Timeout("timeout".to_string()))
            } else {
                Ok(42)
            }
        });

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert!(attempt_count.load(Ordering::SeqCst) >= 3);
    }

    #[test]
    fn test_execute_with_recovery_fatal_error() {
        let (coordinator, _temp) = create_test_coordinator();
        let context = ErrorContext::new("test_op", "/dev/sda");

        let result = coordinator.execute_with_recovery("test_operation", context, || {
            Err::<(), _>(DriveError::NotFound("Device not found".to_string()))
        });

        assert!(result.is_err());
        // Fatal errors should not be retried
    }

    #[test]
    fn test_progress_tracking() {
        let progress = Progress {
            current_pass: 5,
            bytes_written: 1024 * 1024 * 1024,
            state: serde_json::json!({"test": "value"}),
        };

        assert_eq!(progress.current_pass, 5);
        assert_eq!(progress.bytes_written, 1024 * 1024 * 1024);
    }

    #[test]
    fn test_recovery_action_types() {
        let action = RecoveryAction::Retry {
            after: Duration::from_secs(1),
        };
        matches!(action, RecoveryAction::Retry { .. });

        let action = RecoveryAction::Skip {
            reason: "test".to_string(),
        };
        matches!(action, RecoveryAction::Skip { .. });
    }
}
