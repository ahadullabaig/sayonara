// High-Performance Optimized I/O Engine

use super::*;
use super::buffer_pool::{BufferPool, PooledBuffer, PAGE_SIZE};
use super::metrics::{IOMetrics, PerformanceTuner};
use super::platform_specific::{PlatformIO, get_platform_io};
use std::fs::File;
use std::sync::Arc;
use std::time::Instant;
use crate::drives::operations::smart::SMARTMonitor;

/// I/O Configuration
#[derive(Debug, Clone)]
pub struct IOConfig {
    /// Use Direct I/O (O_DIRECT) - bypasses OS cache
    pub use_direct_io: bool,

    /// Initial buffer size (will be tuned adaptively)
    pub initial_buffer_size: usize,

    /// Maximum buffer size
    pub max_buffer_size: usize,

    /// Queue depth for async operations
    pub queue_depth: usize,

    /// Maximum number of buffers to allocate
    pub max_buffers: usize,

    /// Temperature threshold for throttling (Celsius)
    pub temperature_threshold: u32,

    /// Check temperature every N bytes
    pub temperature_check_interval: u64,

    /// Enable adaptive performance tuning
    pub adaptive_tuning: bool,

    /// Target efficiency (percentage of drive's max speed)
    pub target_efficiency: f64,
}

impl Default for IOConfig {
    fn default() -> Self {
        Self {
            use_direct_io: true,
            initial_buffer_size: 4 * 1024 * 1024,  // 4MB
            max_buffer_size: 16 * 1024 * 1024,     // 16MB
            queue_depth: 4,
            max_buffers: 32,
            temperature_threshold: 65,
            temperature_check_interval: 100 * 1024 * 1024,  // 100MB
            adaptive_tuning: true,
            target_efficiency: 95.0,
        }
    }
}

impl IOConfig {
    /// Create config optimized for drive speed
    pub fn for_drive_speed(speed: DriveSpeed) -> Self {
        let mut config = Self::default();
        config.initial_buffer_size = speed.optimal_buffer_size();
        config.queue_depth = speed.optimal_queue_depth();
        config
    }

    /// Create config for NVMe drives
    pub fn nvme_optimized() -> Self {
        Self {
            use_direct_io: true,
            initial_buffer_size: 16 * 1024 * 1024,  // 16MB
            max_buffer_size: 32 * 1024 * 1024,      // 32MB
            queue_depth: 32,
            max_buffers: 64,
            temperature_threshold: 75,  // NVMe can run hotter
            temperature_check_interval: 500 * 1024 * 1024,  // 500MB
            adaptive_tuning: true,
            target_efficiency: 95.0,
        }
    }

    /// Create config for SATA SSD
    pub fn sata_ssd_optimized() -> Self {
        Self {
            use_direct_io: true,
            initial_buffer_size: 8 * 1024 * 1024,   // 8MB
            max_buffer_size: 16 * 1024 * 1024,      // 16MB
            queue_depth: 8,
            max_buffers: 32,
            temperature_threshold: 65,
            temperature_check_interval: 200 * 1024 * 1024,  // 200MB
            adaptive_tuning: true,
            target_efficiency: 95.0,
        }
    }

    /// Create config for HDD
    pub fn hdd_optimized() -> Self {
        Self {
            use_direct_io: true,
            initial_buffer_size: 4 * 1024 * 1024,   // 4MB
            max_buffer_size: 8 * 1024 * 1024,       // 8MB
            queue_depth: 2,
            max_buffers: 16,
            temperature_threshold: 55,  // HDDs run cooler
            temperature_check_interval: 50 * 1024 * 1024,   // 50MB
            adaptive_tuning: true,
            target_efficiency: 90.0,  // HDDs have more overhead
        }
    }

    /// Create config optimized for verification reads (high performance)
    pub fn verification_optimized() -> Self {
        Self {
            use_direct_io: true,
            initial_buffer_size: 8 * 1024 * 1024,   // 8MB buffers for fast reads
            max_buffer_size: 16 * 1024 * 1024,      // 16MB
            queue_depth: 16,                        // Higher queue depth for reads
            max_buffers: 32,
            temperature_threshold: 70,              // Can tolerate higher temps for reads
            temperature_check_interval: 500 * 1024 * 1024,  // 500MB
            adaptive_tuning: true,
            target_efficiency: 95.0,
        }
    }

    /// Create config for small random reads (detection, sampling)
    pub fn small_read_optimized() -> Self {
        Self {
            use_direct_io: false,  // Don't use Direct I/O for small reads
            initial_buffer_size: 64 * 1024,   // 64KB - small buffers
            max_buffer_size: 256 * 1024,      // 256KB
            queue_depth: 4,
            max_buffers: 16,
            temperature_threshold: 70,
            temperature_check_interval: u64::MAX,  // No temp checks for small ops
            adaptive_tuning: false,  // Fixed config for small reads
            target_efficiency: 80.0,
        }
    }
}

/// Optimized I/O Handle
pub struct IOHandle {
    file: File,
    buffer_pool: Arc<BufferPool>,
    metrics: Arc<IOMetrics>,
    tuner: Option<Arc<PerformanceTuner>>,
    platform_io: Box<dyn PlatformIO>,
    config: IOConfig,
    pub(crate) device_path: String,
    bytes_since_temp_check: Arc<std::sync::Mutex<u64>>,
    temperature_monitoring_disabled: Arc<std::sync::atomic::AtomicBool>,
}

impl IOHandle {
    /// Write data at the specified offset
    pub fn write_at(&mut self, data: &[u8], offset: u64) -> IOResult<usize> {
        let start = Instant::now();

        let written = self.platform_io.write_optimized(&self.file, data, offset)?;

        let latency = start.elapsed();
        self.metrics.record_operation(written as u64, latency);

        // Temperature check
        self.check_temperature_if_needed(written as u64)?;

        Ok(written)
    }

    /// Write entire buffer using optimal I/O
    pub fn write_buffer(&mut self, buffer: &PooledBuffer, offset: u64) -> IOResult<usize> {
        self.write_at(buffer.as_slice(), offset)
    }

    /// Read data at the specified offset
    pub fn read_at(&mut self, buffer: &mut [u8], offset: u64) -> IOResult<usize> {
        let start = Instant::now();

        let read = self.platform_io.read_optimized(&self.file, buffer, offset)?;

        let latency = start.elapsed();
        self.metrics.record_operation(read as u64, latency);

        Ok(read)
    }

    /// Read into a pooled buffer
    pub fn read_buffer(&mut self, buffer: &mut PooledBuffer, offset: u64) -> IOResult<usize> {
        self.read_at(buffer.as_mut_slice(), offset)
    }

    /// Sync all data to disk
    pub fn sync(&self) -> IOResult<()> {
        self.platform_io.sync_data(&self.file)
    }

    /// Get current I/O metrics
    pub fn metrics(&self) -> Arc<IOMetrics> {
        self.metrics.clone()
    }

    /// Acquire a buffer from the pool
    pub fn acquire_buffer(&self) -> IOResult<PooledBuffer> {
        self.buffer_pool.acquire()
    }

    /// Check temperature and throttle if needed
    fn check_temperature_if_needed(&mut self, bytes_written: u64) -> IOResult<()> {
        // Skip if temperature monitoring is disabled
        if self.temperature_monitoring_disabled.load(std::sync::atomic::Ordering::Relaxed) {
            return Ok(());
        }

        let mut bytes_since_check = self.bytes_since_temp_check.lock().unwrap();
        *bytes_since_check += bytes_written;

        if *bytes_since_check >= self.config.temperature_check_interval {
            *bytes_since_check = 0;
            drop(bytes_since_check);

            match SMARTMonitor::monitor_temperature(&self.device_path) {
                Ok(temp_monitor) => {
                    if temp_monitor.current_celsius > self.config.temperature_threshold {
                        let throttle = self.calculate_throttle(temp_monitor.current_celsius);
                        self.apply_throttle(throttle)?;
                    }
                }
                Err(_) => {
                    // Temperature monitoring failed - disable it and warn once
                    eprintln!("⚠️  WARNING: Could not read temperature sensor");
                    eprintln!("   Temperature monitoring will be disabled.");
                    self.temperature_monitoring_disabled.store(true, std::sync::atomic::Ordering::Relaxed);
                }
            }
        }

        Ok(())
    }

    /// Calculate throttling action based on temperature
    pub(crate) fn calculate_throttle(&self, temp: u32) -> ThrottleAction {
        let threshold = self.config.temperature_threshold;

        if temp < threshold {
            ThrottleAction::None
        } else if temp < threshold + 5 {
            // Slow down by 25%
            ThrottleAction::Slow(0.75)
        } else if temp < threshold + 10 {
            // Slow down by 50%
            ThrottleAction::Slow(0.50)
        } else {
            // Pause for cooling
            let pause_secs = ((temp - threshold) / 5) as u64;
            ThrottleAction::Pause(std::time::Duration::from_secs(pause_secs.min(30)))
        }
    }

    /// Apply throttling action
    fn apply_throttle(&self, action: ThrottleAction) -> IOResult<()> {
        match action {
            ThrottleAction::None => Ok(()),
            ThrottleAction::Slow(factor) => {
                println!("🌡️  Temperature throttling: Reducing speed to {:.0}%", factor * 100.0);
                // Implement by adding delays between writes
                std::thread::sleep(std::time::Duration::from_millis(
                    ((1.0 - factor) * 100.0) as u64
                ));
                Ok(())
            }
            ThrottleAction::Pause(duration) => {
                println!("🌡️  Temperature too high! Pausing for {:?} to cool down", duration);
                std::thread::sleep(duration);
                Ok(())
            }
        }
    }
}

/// Optimized I/O Engine
pub struct OptimizedIO;

impl OptimizedIO {
    /// Open a device/file with optimized I/O
    pub fn open(device_path: &str, config: IOConfig) -> IOResult<IOHandle> {
        let platform_io = get_platform_io();

        // Only print for large operations (not detection/sampling)
        if config.initial_buffer_size >= 1024 * 1024 {
            println!("🚀 Opening device with optimized I/O");
            println!("   Platform: {}", platform_io.platform_name());
            println!("   Direct I/O: {}", config.use_direct_io);
            println!("   Buffer Size: {} MB", config.initial_buffer_size / (1024 * 1024));
            println!("   Queue Depth: {}", config.queue_depth);
        }

        // Open file with platform-specific optimizations
        let file = platform_io.open_optimized(device_path, config.use_direct_io)?;

        // Create buffer pool
        let alignment = if config.use_direct_io { PAGE_SIZE } else { 8 };
        let buffer_pool = Arc::new(BufferPool::new(
            config.initial_buffer_size,
            alignment,
            config.max_buffers,
        ));

        // Pre-allocate some buffers
        buffer_pool.preallocate(config.queue_depth)?;

        // Create metrics
        let metrics = Arc::new(IOMetrics::new());

        // Create performance tuner if adaptive tuning is enabled
        let tuner = if config.adaptive_tuning {
            Some(Arc::new(PerformanceTuner::new()))
        } else {
            None
        };

        Ok(IOHandle {
            file,
            buffer_pool,
            metrics,
            tuner,
            platform_io,
            config,
            device_path: device_path.to_string(),
            bytes_since_temp_check: Arc::new(std::sync::Mutex::new(0)),
            temperature_monitoring_disabled: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        })
    }

    /// Perform a full sequential write with optimizations
    pub fn sequential_write<F>(
        handle: &mut IOHandle,
        total_size: u64,
        mut fill_buffer: F,
    ) -> IOResult<()>
    where
        F: FnMut(&mut PooledBuffer) -> IOResult<()>,
    {
        let mut offset = 0u64;
        let buffer_size = handle.buffer_pool.stats().buffer_size as u64;

        while offset < total_size {
            // Check for interrupt signal
            if crate::is_interrupted() {
                return Err(IOError::Interrupted);
            }

            let write_size = (total_size - offset).min(buffer_size);

            // Acquire buffer from pool
            let mut buffer = handle.acquire_buffer()?;

            // Fill buffer with data
            fill_buffer(&mut buffer)?;

            // Write only the needed portion to device
            let buffer_slice = &buffer.as_slice()[..write_size as usize];
            let written = handle.write_at(buffer_slice, offset)?;

            if written as u64 != write_size {
                return Err(IOError::OperationFailed(
                    format!("Partial write: {} of {} bytes", written, write_size)
                ));
            }

            offset += written as u64;

            // Adaptive tuning if enabled
            if let Some(ref tuner) = handle.tuner {
                let stats = handle.metrics.stats();
                if stats.elapsed.as_secs() > 0 && stats.throughput_bps > 0 {
                    // Tuner can adjust buffer size and queue depth
                    let _ = tuner.record_and_tune(
                        written as u64,
                        stats.avg_latency
                    );
                }
            }
        }

        // Final sync
        handle.sync()?;

        Ok(())
    }

    /// Perform a full sequential read with optimizations
    pub fn sequential_read<F>(
        handle: &mut IOHandle,
        total_size: u64,
        mut process_buffer: F,
    ) -> IOResult<()>
    where
        F: FnMut(&PooledBuffer, usize) -> IOResult<()>,
    {
        let mut offset = 0u64;

        while offset < total_size {
            // Acquire buffer from pool
            let mut buffer = handle.acquire_buffer()?;

            // Read from device
            let bytes_read = handle.read_buffer(&mut buffer, offset)?;

            if bytes_read == 0 {
                return Err(IOError::OperationFailed(
                    format!("Unexpected EOF at offset {}", offset)
                ));
            }

            // Process the read data
            process_buffer(&buffer, bytes_read)?;

            offset += bytes_read as u64;

            // Adaptive tuning if enabled
            if let Some(ref tuner) = handle.tuner {
                let stats = handle.metrics.stats();
                if stats.elapsed.as_secs() > 0 && stats.throughput_bps > 0 {
                    let _ = tuner.record_and_tune(
                        bytes_read as u64,
                        stats.avg_latency
                    );
                }
            }
        }

        Ok(())
    }

    /// Read a specific range into a single buffer
    pub fn read_range(
        handle: &mut IOHandle,
        offset: u64,
        size: usize,
    ) -> IOResult<Vec<u8>> {
        let mut data = vec![0u8; size];
        let bytes_read = handle.read_at(&mut data, offset)?;
        data.truncate(bytes_read);
        Ok(data)
    }

    /// Print final performance report
    pub fn print_performance_report(handle: &IOHandle, drive_max_speed_bps: Option<u64>) {
        let stats = handle.metrics.stats();

        println!("\n📊 I/O Performance Report");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        stats.print();

        if let Some(max_speed) = drive_max_speed_bps {
            let efficiency = stats.efficiency(max_speed);
            println!("  ⚙️  Efficiency: {:.1}% of drive max", efficiency);

            if efficiency >= 95.0 {
                println!("  ✅ EXCELLENT: Achieved 95%+ efficiency target!");
            } else if efficiency >= 85.0 {
                println!("  ✅ GOOD: Above 85% efficiency");
            } else {
                println!("  ⚠️  SUBOPTIMAL: Below 85% efficiency");
            }
        }

        // Buffer pool stats
        let pool_stats = handle.buffer_pool.stats();
        println!("\n📦 Buffer Pool Statistics:");
        println!("  Allocated Buffers: {}", pool_stats.allocated);
        println!("  Available Buffers: {}", pool_stats.available);
        println!("  Total Memory: {:.2} MB",
                 pool_stats.total_memory as f64 / (1024.0 * 1024.0));

        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_io_config_defaults() {
        let config = IOConfig::default();
        assert!(config.use_direct_io);
        assert_eq!(config.initial_buffer_size, 4 * 1024 * 1024);
    }

    #[test]
    fn test_drive_speed_configs() {
        let nvme_config = IOConfig::nvme_optimized();
        assert!(nvme_config.queue_depth > 8);

        let hdd_config = IOConfig::hdd_optimized();
        assert!(hdd_config.queue_depth < nvme_config.queue_depth);
    }

    #[test]
    fn test_optimized_io_creation() {
        let temp = NamedTempFile::new().unwrap();
        let path = temp.path().to_str().unwrap();

        // Use buffered I/O for test (Direct I/O requires block device)
        let mut config = IOConfig::default();
        config.use_direct_io = false;

        let handle = OptimizedIO::open(path, config);
        assert!(handle.is_ok());
    }
}
