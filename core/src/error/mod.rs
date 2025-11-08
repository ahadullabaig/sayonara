/// Error recovery system for Sayonara Wipe
///
/// This module provides a comprehensive error recovery framework including:
/// - Error classification and handling
/// - SQLite-based checkpoint/resume capability
/// - Retry strategies with exponential backoff and jitter
/// - Circuit breaker pattern
/// - Bad sector handling
/// - Self-healing mechanisms (driver reload, device reset, etc.)
/// - Degraded mode operations
/// - Alternative I/O fallback methods
///
/// # Architecture
///
/// The error recovery system is built in layers:
///
/// ```text
/// ┌─────────────────────────────────────────┐
/// │     Recovery Coordinator (Orchestration) │
/// └────────────────┬────────────────────────┘
///                  │
///      ┌───────────┴───────────┐
///      ↓                       ↓
/// ┌────────────┐        ┌──────────────┐
/// │Classification│      │  Checkpoint   │
/// │   & Retry    │      │   Manager     │
/// └─────┬────────┘      └──────────────┘
///       │
///       ↓
/// ┌─────────────────────────────────────┐
/// │     Recovery Mechanisms             │
/// │  - Bad Sector Handler               │
/// │  - Self-Healer                      │
/// │  - Degraded Mode                    │
/// │  - Alternative I/O                  │
/// └─────────────────────────────────────┘
/// ```
///
/// # Usage Example
///
/// ```rust,ignore
/// use sayonara_wipe::error::{RecoveryCoordinator, Progress, ErrorContext};
/// use sayonara_wipe::WipeConfig;
///
/// let config = WipeConfig::default();
/// let mut coordinator = RecoveryCoordinator::new("/dev/sda", &config)?;
///
/// // Try to resume from checkpoint
/// if let Some(resume) = coordinator.resume_from_checkpoint("Gutmann")? {
///     println!("Resuming from pass {}", resume.current_pass);
/// }
///
/// // Execute operation with recovery
/// for pass in 0..35 {
///     let context = ErrorContext::for_pass("/dev/sda", "Gutmann", pass);
///
///     coordinator.execute_with_recovery(
///         "wipe_pass",
///         context,
///         || {
///             // Your wipe logic here
///             perform_wipe_pass(pass)?;
///             Ok(())
///         }
///     )?;
///
///     // Save checkpoint
///     coordinator.maybe_checkpoint(
///         "Gutmann",
///         35,
///         total_size,
///         &Progress {
///             current_pass: pass,
///             bytes_written: (pass as u64 + 1) * total_size,
///             state: serde_json::json!({"pass": pass}),
///         }
///     )?;
/// }
///
/// // Clean up checkpoint on success
/// coordinator.delete_checkpoint()?;
/// # Ok::<(), anyhow::Error>(())
/// ```

pub mod checkpoint;
pub mod classification;
pub mod mechanisms;
pub mod recovery_coordinator;
pub mod retry;

// Re-export main types for convenience
pub use checkpoint::{Checkpoint, CheckpointManager, CheckpointStats};
pub use classification::{ClassifiedError, ErrorClass, ErrorClassifier, ErrorContext};
pub use mechanisms::{
    AlternativeIO, BadSectorHandler, BadSectorReport, DegradedMode, DegradedModeManager,
    HealMethod, IOMethod, SelfHealer, WriteResult,
};
pub use recovery_coordinator::{Progress, RecoveryAction, RecoveryCoordinator, ResumeState};
pub use retry::{CircuitBreaker, ExponentialBackoff, RetryConfig, RetryStrategy};
