/// Retry strategies with exponential backoff, jitter, and circuit breaker pattern
///
/// This module provides sophisticated retry logic for handling transient failures.
/// Includes exponential backoff with jitter to prevent thundering herd,
/// and circuit breaker pattern to fail fast when service is persistently down.
use super::classification::{ClassifiedError, ErrorClass};
use anyhow::Result;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Retry strategy trait
pub trait RetryStrategy: Send + Sync {
    /// Determine if retry should be attempted
    fn should_retry(&self, attempt: u32, error: &ClassifiedError) -> bool;

    /// Calculate delay before next retry
    fn next_delay(&self, attempt: u32) -> Duration;

    /// Maximum number of retry attempts
    fn max_attempts(&self) -> u32;
}

/// Exponential backoff retry strategy with jitter
///
/// Implements exponential backoff: delay = base * 2^attempt
/// Adds random jitter to prevent thundering herd problem
#[derive(Debug, Clone)]
pub struct ExponentialBackoff {
    /// Base delay for first retry (default: 100ms)
    base_delay: Duration,

    /// Maximum delay cap (default: 30 seconds)
    max_delay: Duration,

    /// Maximum retry attempts
    max_attempts: u32,

    /// Jitter factor (0.0 - 1.0) - adds randomness to delay
    jitter_factor: f64,
}

impl ExponentialBackoff {
    /// Create new exponential backoff strategy
    pub fn new(base_delay: Duration, max_delay: Duration, max_attempts: u32) -> Self {
        Self {
            base_delay,
            max_delay,
            max_attempts,
            jitter_factor: 0.3, // 30% jitter by default
        }
    }

    /// Create with custom jitter factor
    pub fn with_jitter(mut self, jitter_factor: f64) -> Self {
        self.jitter_factor = jitter_factor.clamp(0.0, 1.0);
        self
    }

    /// Preset for transient errors (fast retries)
    pub fn transient() -> Self {
        Self::new(Duration::from_millis(100), Duration::from_secs(30), 10)
    }

    /// Preset for recoverable errors (moderate retries)
    pub fn recoverable() -> Self {
        Self::new(Duration::from_millis(500), Duration::from_secs(60), 5)
    }

    /// Preset for environmental errors (slow, patient retries)
    pub fn environmental() -> Self {
        Self::new(Duration::from_secs(5), Duration::from_secs(300), 20)
    }

    /// Calculate exponential delay with jitter
    fn calculate_delay(&self, attempt: u32) -> Duration {
        // Calculate exponential delay: base * 2^attempt
        let exponential_ms = self.base_delay.as_millis() * (2_u128.pow(attempt));
        let capped_ms = exponential_ms.min(self.max_delay.as_millis());

        // Add jitter: delay ± (delay * jitter_factor)
        let jitter_range = capped_ms as f64 * self.jitter_factor;
        let jitter = (rand::random::<f64>() - 0.5) * 2.0 * jitter_range;
        let final_ms = (capped_ms as f64 + jitter).max(0.0);

        Duration::from_millis(final_ms as u64)
    }
}

impl RetryStrategy for ExponentialBackoff {
    fn should_retry(&self, attempt: u32, error: &ClassifiedError) -> bool {
        attempt < self.max_attempts && error.class.allows_retry()
    }

    fn next_delay(&self, attempt: u32) -> Duration {
        self.calculate_delay(attempt)
    }

    fn max_attempts(&self) -> u32 {
        self.max_attempts
    }
}

/// No retry strategy (for fatal errors)
#[derive(Debug, Clone, Copy)]
pub struct NoRetry;

impl RetryStrategy for NoRetry {
    fn should_retry(&self, _attempt: u32, _error: &ClassifiedError) -> bool {
        false
    }

    fn next_delay(&self, _attempt: u32) -> Duration {
        Duration::from_secs(0)
    }

    fn max_attempts(&self) -> u32 {
        0
    }
}

/// Circuit breaker pattern - prevents cascading failures
///
/// States:
/// - Closed: Normal operation, requests pass through
/// - Open: Too many failures, fail fast without trying
/// - HalfOpen: Testing if service recovered
#[derive(Debug)]
pub struct CircuitBreaker {
    /// Number of failures before opening circuit
    failure_threshold: u32,

    /// Number of successes needed to close circuit
    success_threshold: u32,

    /// Time to wait before transitioning to half-open
    timeout: Duration,

    /// Internal state (protected by mutex)
    state: Arc<Mutex<CircuitState>>,
}

#[derive(Debug, Clone)]
struct CircuitState {
    /// Current circuit state
    status: CircuitStatus,

    /// Consecutive failure count
    failure_count: u32,

    /// Consecutive success count (in half-open state)
    success_count: u32,

    /// When circuit was opened
    opened_at: Option<Instant>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CircuitStatus {
    /// Normal operation
    Closed,

    /// Too many failures, fail fast
    Open,

    /// Testing if service recovered
    HalfOpen,
}

impl CircuitBreaker {
    /// Create new circuit breaker
    pub fn new(failure_threshold: u32, success_threshold: u32, timeout: Duration) -> Self {
        Self {
            failure_threshold,
            success_threshold,
            timeout,
            state: Arc::new(Mutex::new(CircuitState {
                status: CircuitStatus::Closed,
                failure_count: 0,
                success_count: 0,
                opened_at: None,
            })),
        }
    }

    /// Default circuit breaker for device operations
    pub fn default_device() -> Self {
        Self::new(
            5,                       // Open after 5 failures
            3,                       // Close after 3 successes
            Duration::from_secs(30), // Wait 30s before testing
        )
    }

    /// Execute operation with circuit breaker protection
    pub fn call<F, T>(&self, operation: F) -> Result<T>
    where
        F: FnOnce() -> Result<T>,
    {
        // Check current state
        {
            let mut state = self.state.lock().unwrap();

            match state.status {
                CircuitStatus::Open => {
                    // Check if timeout elapsed
                    if let Some(opened_at) = state.opened_at {
                        if opened_at.elapsed() >= self.timeout {
                            // Transition to half-open
                            state.status = CircuitStatus::HalfOpen;
                            state.success_count = 0;
                            tracing::info!("Circuit breaker transitioning to HalfOpen");
                        } else {
                            // Still open, fail fast
                            return Err(anyhow::anyhow!(
                                "Circuit breaker is OPEN - failing fast (opened {} ago)",
                                humantime::format_duration(opened_at.elapsed())
                            ));
                        }
                    }
                }
                CircuitStatus::Closed | CircuitStatus::HalfOpen => {
                    // Allow operation to proceed
                }
            }
        }

        // Execute operation
        match operation() {
            Ok(result) => {
                self.record_success();
                Ok(result)
            }
            Err(error) => {
                self.record_failure();
                Err(error)
            }
        }
    }

    /// Record successful operation
    fn record_success(&self) {
        let mut state = self.state.lock().unwrap();

        match state.status {
            CircuitStatus::Closed => {
                // Reset failure count on success
                state.failure_count = 0;
            }
            CircuitStatus::HalfOpen => {
                state.success_count += 1;
                if state.success_count >= self.success_threshold {
                    // Close circuit
                    state.status = CircuitStatus::Closed;
                    state.failure_count = 0;
                    state.success_count = 0;
                    state.opened_at = None;
                    tracing::info!(
                        "Circuit breaker CLOSED after {} successes",
                        self.success_threshold
                    );
                }
            }
            CircuitStatus::Open => {
                // Shouldn't happen, but reset if it does
                tracing::warn!("Success recorded while circuit was OPEN - resetting");
                state.status = CircuitStatus::Closed;
                state.failure_count = 0;
                state.opened_at = None;
            }
        }
    }

    /// Record failed operation
    fn record_failure(&self) {
        let mut state = self.state.lock().unwrap();

        match state.status {
            CircuitStatus::Closed => {
                state.failure_count += 1;
                if state.failure_count >= self.failure_threshold {
                    // Open circuit
                    state.status = CircuitStatus::Open;
                    state.opened_at = Some(Instant::now());
                    tracing::warn!(
                        "Circuit breaker OPENED after {} failures",
                        self.failure_threshold
                    );
                }
            }
            CircuitStatus::HalfOpen => {
                // Failed in half-open, reopen immediately
                state.status = CircuitStatus::Open;
                state.opened_at = Some(Instant::now());
                state.success_count = 0;
                tracing::warn!("Circuit breaker REOPENED after failure in HalfOpen state");
            }
            CircuitStatus::Open => {
                // Already open, nothing to do
            }
        }
    }

    /// Get current circuit status
    pub fn status(&self) -> String {
        let state = self.state.lock().unwrap();
        format!("{:?}", state.status)
    }

    /// Check if circuit is open
    pub fn is_open(&self) -> bool {
        let state = self.state.lock().unwrap();
        matches!(state.status, CircuitStatus::Open)
    }

    /// Reset circuit breaker to closed state
    pub fn reset(&self) {
        let mut state = self.state.lock().unwrap();
        state.status = CircuitStatus::Closed;
        state.failure_count = 0;
        state.success_count = 0;
        state.opened_at = None;
        tracing::info!("Circuit breaker manually reset to CLOSED");
    }
}

/// Retry configuration per error class
pub struct RetryConfig {
    strategies: std::collections::HashMap<ErrorClass, Box<dyn RetryStrategy>>,
}

impl RetryConfig {
    /// Create default retry configuration
    pub fn new() -> Self {
        let mut strategies: std::collections::HashMap<ErrorClass, Box<dyn RetryStrategy>> =
            std::collections::HashMap::new();

        strategies.insert(
            ErrorClass::Transient,
            Box::new(ExponentialBackoff::transient()),
        );

        strategies.insert(
            ErrorClass::Recoverable,
            Box::new(ExponentialBackoff::recoverable()),
        );

        strategies.insert(
            ErrorClass::Environmental,
            Box::new(ExponentialBackoff::environmental()),
        );

        strategies.insert(ErrorClass::Fatal, Box::new(NoRetry));

        strategies.insert(ErrorClass::UserInterrupted, Box::new(NoRetry));

        Self { strategies }
    }

    /// Get retry strategy for error class
    pub fn get_strategy(&self, class: ErrorClass) -> &dyn RetryStrategy {
        self.strategies
            .get(&class)
            .map(|s| s.as_ref())
            .unwrap_or(&NoRetry as &dyn RetryStrategy)
    }

    /// Set custom strategy for error class
    pub fn set_strategy(&mut self, class: ErrorClass, strategy: Box<dyn RetryStrategy>) {
        self.strategies.insert(class, strategy);
    }
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::classification::ErrorContext;
    use crate::DriveError;

    #[test]
    fn test_exponential_backoff_delays() {
        let backoff =
            ExponentialBackoff::new(Duration::from_millis(100), Duration::from_secs(10), 5)
                .with_jitter(0.0); // No jitter for predictable testing

        let delay0 = backoff.next_delay(0);
        let delay1 = backoff.next_delay(1);
        let delay2 = backoff.next_delay(2);

        assert_eq!(delay0.as_millis(), 100);
        assert_eq!(delay1.as_millis(), 200);
        assert_eq!(delay2.as_millis(), 400);
    }

    #[test]
    fn test_exponential_backoff_max_delay() {
        let backoff =
            ExponentialBackoff::new(Duration::from_millis(100), Duration::from_secs(1), 10)
                .with_jitter(0.0);

        // Should cap at max_delay (1000ms)
        let delay10 = backoff.next_delay(10);
        assert_eq!(delay10.as_millis(), 1000);
    }

    #[test]
    fn test_exponential_backoff_jitter() {
        let backoff =
            ExponentialBackoff::new(Duration::from_millis(100), Duration::from_secs(10), 5)
                .with_jitter(0.5);

        // Generate multiple delays and verify they differ (jitter working)
        let delays: Vec<_> = (0..10).map(|_| backoff.next_delay(1).as_millis()).collect();

        // Should not all be identical due to jitter
        let all_same = delays.windows(2).all(|w| w[0] == w[1]);
        assert!(!all_same, "Jitter should produce varied delays");
    }

    #[test]
    fn test_retry_strategy_presets() {
        let transient = ExponentialBackoff::transient();
        assert_eq!(transient.max_attempts(), 10);

        let recoverable = ExponentialBackoff::recoverable();
        assert_eq!(recoverable.max_attempts(), 5);

        let environmental = ExponentialBackoff::environmental();
        assert_eq!(environmental.max_attempts(), 20);
    }

    #[test]
    fn test_circuit_breaker_closed_state() {
        let cb = CircuitBreaker::new(3, 2, Duration::from_secs(1));

        // First operation should succeed
        let result = cb.call(|| Ok::<_, anyhow::Error>(42));
        assert!(result.is_ok());
        assert!(!cb.is_open());
    }

    #[test]
    fn test_circuit_breaker_opens_after_failures() {
        let cb = CircuitBreaker::new(3, 2, Duration::from_secs(1));

        // Fail 3 times
        for _ in 0..3 {
            let _ = cb.call(|| Err::<(), _>(anyhow::anyhow!("failure")));
        }

        // Circuit should be open now
        assert!(cb.is_open());

        // Next call should fail fast
        let result = cb.call(|| Ok::<_, anyhow::Error>(42));
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Circuit breaker is OPEN"));
    }

    #[test]
    fn test_circuit_breaker_half_open_transition() {
        let cb = CircuitBreaker::new(2, 2, Duration::from_millis(100));

        // Open the circuit
        for _ in 0..2 {
            let _ = cb.call(|| Err::<(), _>(anyhow::anyhow!("failure")));
        }
        assert!(cb.is_open());

        // Wait for timeout
        std::thread::sleep(Duration::from_millis(150));

        // Next call should transition to half-open and succeed
        let result = cb.call(|| Ok::<_, anyhow::Error>(42));
        assert!(result.is_ok());
    }

    #[test]
    fn test_circuit_breaker_closes_after_successes() {
        let cb = CircuitBreaker::new(2, 2, Duration::from_millis(100));

        // Open the circuit
        for _ in 0..2 {
            let _ = cb.call(|| Err::<(), _>(anyhow::anyhow!("failure")));
        }

        // Wait for timeout
        std::thread::sleep(Duration::from_millis(150));

        // Succeed twice to close
        for _ in 0..2 {
            let _ = cb.call(|| Ok::<_, anyhow::Error>(()));
        }

        assert!(!cb.is_open());
    }

    #[test]
    fn test_circuit_breaker_reset() {
        let cb = CircuitBreaker::new(2, 2, Duration::from_secs(1));

        // Open the circuit
        for _ in 0..2 {
            let _ = cb.call(|| Err::<(), _>(anyhow::anyhow!("failure")));
        }
        assert!(cb.is_open());

        // Reset
        cb.reset();
        assert!(!cb.is_open());

        // Should accept operations now
        let result = cb.call(|| Ok::<_, anyhow::Error>(42));
        assert!(result.is_ok());
    }

    #[test]
    fn test_no_retry_strategy() {
        let strategy = NoRetry;
        let context = ErrorContext::new("test", "/dev/sda");
        let error = crate::error::classification::ErrorClassifier::new().classify(
            DriveError::NotFound("Device not found".to_string()),
            context,
        );

        assert!(!strategy.should_retry(0, &error));
        assert_eq!(strategy.max_attempts(), 0);
        assert_eq!(strategy.next_delay(0), Duration::from_secs(0));
    }

    #[test]
    fn test_retry_config_default() {
        let config = RetryConfig::new();

        // Transient errors get retry strategy
        let transient_strategy = config.get_strategy(ErrorClass::Transient);
        assert_eq!(transient_strategy.max_attempts(), 10);

        // Fatal errors get no retry
        let fatal_strategy = config.get_strategy(ErrorClass::Fatal);
        assert_eq!(fatal_strategy.max_attempts(), 0);
    }

    #[test]
    fn test_retry_strategy_should_retry() {
        let backoff =
            ExponentialBackoff::new(Duration::from_millis(100), Duration::from_secs(1), 3);

        let context = ErrorContext::new("test", "/dev/sda");
        let error = crate::error::classification::ErrorClassifier::new()
            .classify(DriveError::Timeout("timeout".to_string()), context);

        assert!(backoff.should_retry(0, &error));
        assert!(backoff.should_retry(1, &error));
        assert!(backoff.should_retry(2, &error));
        assert!(!backoff.should_retry(3, &error));
    }
}
