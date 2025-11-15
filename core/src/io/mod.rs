pub mod buffer_pool;
pub mod io_uring_engine;
pub mod metrics;
pub mod mmap_engine;
pub mod optimized_engine;
pub mod platform_specific;

#[cfg(test)]
mod tests;

// Re-exports
pub use buffer_pool::{AlignedBuffer, BufferPool};
pub use metrics::{IOMetrics, PerformanceStats};
pub use optimized_engine::{IOConfig, IOHandle, OptimizedIO};

use std::time::Duration;

/// I/O operation mode
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IOMode {
    /// Standard buffered I/O (slower, but compatible)
    Buffered,
    /// Direct I/O bypassing OS cache (faster)
    Direct,
    /// Memory-mapped I/O
    MemoryMapped,
}

/// I/O pattern for optimization
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IOPattern {
    Sequential,
    Random,
    Mixed,
}

/// Drive speed category for adaptive buffering
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DriveSpeed {
    Slow,     // < 100 MB/s (USB 2.0, old HDDs)
    Medium,   // 100-300 MB/s (SATA HDDs, USB 3.0)
    Fast,     // 300-600 MB/s (SATA SSDs)
    VeryFast, // > 600 MB/s (NVMe, high-end SSDs)
}

impl DriveSpeed {
    /// Determine optimal buffer size for this drive speed
    pub fn optimal_buffer_size(&self) -> usize {
        match self {
            DriveSpeed::Slow => 1024 * 1024,      // 1MB
            DriveSpeed::Medium => 4 * 1024 * 1024,    // 4MB
            DriveSpeed::Fast => 8 * 1024 * 1024,      // 8MB
            DriveSpeed::VeryFast => 16 * 1024 * 1024, // 16MB
        }
    }

    /// Determine optimal queue depth
    pub fn optimal_queue_depth(&self) -> usize {
        match self {
            DriveSpeed::Slow => 2,
            DriveSpeed::Medium => 4,
            DriveSpeed::Fast => 8,
            DriveSpeed::VeryFast => 32,
        }
    }

    /// Detect drive speed from observed throughput
    pub fn from_throughput(bytes_per_sec: u64) -> Self {
        let mb_per_sec = bytes_per_sec / (1024 * 1024);

        if mb_per_sec < 100 {
            DriveSpeed::Slow
        } else if mb_per_sec < 300 {
            DriveSpeed::Medium
        } else if mb_per_sec < 600 {
            DriveSpeed::Fast
        } else {
            DriveSpeed::VeryFast
        }
    }
}

/// Result type for I/O operations
pub type IOResult<T> = Result<T, IOError>;

/// I/O specific errors
#[derive(Debug, thiserror::Error)]
pub enum IOError {
    #[error("I/O operation failed: {0}")]
    OperationFailed(String),

    #[error("Alignment error: {0}")]
    AlignmentError(String),

    #[error("Buffer allocation failed: {0}")]
    AllocationFailed(String),

    #[error("Platform not supported: {0}")]
    PlatformNotSupported(String),

    #[error("Performance degradation detected: {0}")]
    PerformanceDegraded(String),

    #[error("Operation interrupted by user")]
    Interrupted,

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Anyhow error: {0}")]
    AnyhowError(#[from] anyhow::Error),
}

/// Temperature-based throttling decision
#[derive(Debug, Clone, Copy)]
pub enum ThrottleAction {
    None,
    Slow(f64), // Reduce speed by this factor (0.0-1.0)
    Pause(Duration),
}
