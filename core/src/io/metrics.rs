// Performance metrics tracking for I/O operations

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Real-time I/O metrics
#[derive(Debug, Clone)]
pub struct IOMetrics {
    start_time: Instant,
    bytes_processed: Arc<Mutex<u64>>,
    operations_count: Arc<Mutex<u64>>,
    latencies: Arc<Mutex<Vec<Duration>>>,
    errors: Arc<Mutex<u64>>,
    last_update: Arc<Mutex<Instant>>,
}

impl IOMetrics {
    /// Create new metrics tracker
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            bytes_processed: Arc::new(Mutex::new(0)),
            operations_count: Arc::new(Mutex::new(0)),
            latencies: Arc::new(Mutex::new(Vec::new())),
            errors: Arc::new(Mutex::new(0)),
            last_update: Arc::new(Mutex::new(Instant::now())),
        }
    }

    /// Record a successful operation
    pub fn record_operation(&self, bytes: u64, latency: Duration) {
        *self.bytes_processed.lock().unwrap() += bytes;
        *self.operations_count.lock().unwrap() += 1;

        // Keep last 1000 latencies for percentile calculation
        let mut latencies = self.latencies.lock().unwrap();
        latencies.push(latency);
        if latencies.len() > 1000 {
            latencies.remove(0);
        }

        *self.last_update.lock().unwrap() = Instant::now();
    }

    /// Record an error
    pub fn record_error(&self) {
        *self.errors.lock().unwrap() += 1;
    }

    /// Get current throughput in bytes/sec
    pub fn throughput(&self) -> u64 {
        let elapsed = self.start_time.elapsed().as_secs_f64();
        if elapsed < 0.001 {
            return 0;
        }

        let bytes = *self.bytes_processed.lock().unwrap();
        (bytes as f64 / elapsed) as u64
    }

    /// Get current IOPS (I/O operations per second)
    pub fn iops(&self) -> u64 {
        let elapsed = self.start_time.elapsed().as_secs_f64();
        if elapsed < 0.001 {
            return 0;
        }

        let ops = *self.operations_count.lock().unwrap();
        (ops as f64 / elapsed) as u64
    }

    /// Get average latency
    pub fn average_latency(&self) -> Duration {
        let latencies = self.latencies.lock().unwrap();
        if latencies.is_empty() {
            return Duration::from_secs(0);
        }

        let sum: Duration = latencies.iter().sum();
        sum / latencies.len() as u32
    }

    /// Get latency percentile (p50, p95, p99)
    pub fn latency_percentile(&self, percentile: f64) -> Duration {
        let mut latencies = self.latencies.lock().unwrap().clone();
        if latencies.is_empty() {
            return Duration::from_secs(0);
        }

        latencies.sort();
        let index = ((percentile / 100.0) * latencies.len() as f64) as usize;
        let index = index.min(latencies.len() - 1);
        latencies[index]
    }

    /// Get performance statistics
    pub fn stats(&self) -> PerformanceStats {
        let bytes = *self.bytes_processed.lock().unwrap();
        let ops = *self.operations_count.lock().unwrap();
        let errors = *self.errors.lock().unwrap();
        let elapsed = self.start_time.elapsed();

        PerformanceStats {
            elapsed,
            bytes_processed: bytes,
            operations_count: ops,
            errors,
            throughput_bps: self.throughput(),
            iops: self.iops(),
            avg_latency: self.average_latency(),
            p50_latency: self.latency_percentile(50.0),
            p95_latency: self.latency_percentile(95.0),
            p99_latency: self.latency_percentile(99.0),
        }
    }

    /// Check if performance is degrading
    pub fn is_degraded(&self, baseline_throughput: u64) -> bool {
        let current = self.throughput();

        // If no operations yet, can't determine degradation
        let ops = *self.operations_count.lock().unwrap();
        if ops == 0 {
            return false;
        }

        // Consider degraded if throughput drops below 50% of baseline
        if baseline_throughput > 0 {
            current < baseline_throughput / 2
        } else {
            false
        }
    }

    /// Reset metrics
    pub fn reset(&mut self) {
        *self.bytes_processed.lock().unwrap() = 0;
        *self.operations_count.lock().unwrap() = 0;
        self.latencies.lock().unwrap().clear();
        *self.errors.lock().unwrap() = 0;
        self.start_time = Instant::now();
        *self.last_update.lock().unwrap() = Instant::now();
    }
}

impl Default for IOMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Performance statistics snapshot
#[derive(Debug, Clone)]
pub struct PerformanceStats {
    pub elapsed: Duration,
    pub bytes_processed: u64,
    pub operations_count: u64,
    pub errors: u64,
    pub throughput_bps: u64,
    pub iops: u64,
    pub avg_latency: Duration,
    pub p50_latency: Duration,
    pub p95_latency: Duration,
    pub p99_latency: Duration,
}

impl PerformanceStats {
    /// Format throughput in human-readable form
    pub fn throughput_human(&self) -> String {
        let mb_per_sec = self.throughput_bps as f64 / (1024.0 * 1024.0);
        if mb_per_sec >= 1000.0 {
            format!("{:.2} GB/s", mb_per_sec / 1024.0)
        } else {
            format!("{:.2} MB/s", mb_per_sec)
        }
    }

    /// Format IOPS in human-readable form
    pub fn iops_human(&self) -> String {
        if self.iops >= 1_000_000 {
            format!("{:.2}M IOPS", self.iops as f64 / 1_000_000.0)
        } else if self.iops >= 1_000 {
            format!("{:.2}K IOPS", self.iops as f64 / 1_000.0)
        } else {
            format!("{} IOPS", self.iops)
        }
    }

    /// Calculate efficiency (percentage of theoretical max)
    pub fn efficiency(&self, theoretical_max_bps: u64) -> f64 {
        if theoretical_max_bps == 0 {
            return 0.0;
        }
        (self.throughput_bps as f64 / theoretical_max_bps as f64) * 100.0
    }

    /// Pretty print statistics
    pub fn print(&self) {
        println!("I/O Performance Statistics:");
        println!("  ⏱️  Elapsed: {:.2}s", self.elapsed.as_secs_f64());
        println!(
            "  📊 Bytes Processed: {} ({:.2} GB)",
            self.bytes_processed,
            self.bytes_processed as f64 / (1024.0 * 1024.0 * 1024.0)
        );
        println!("  🔄 Operations: {}", self.operations_count);
        println!("  ❌ Errors: {}", self.errors);
        println!("  ⚡ Throughput: {}", self.throughput_human());
        println!("  🎯 IOPS: {}", self.iops_human());
        println!("  ⏲️  Latency:");
        println!(
            "     Average: {:.2}ms",
            self.avg_latency.as_secs_f64() * 1000.0
        );
        println!("     P50: {:.2}ms", self.p50_latency.as_secs_f64() * 1000.0);
        println!("     P95: {:.2}ms", self.p95_latency.as_secs_f64() * 1000.0);
        println!("     P99: {:.2}ms", self.p99_latency.as_secs_f64() * 1000.0);
    }
}

/// Automatic performance tuner
pub struct PerformanceTuner {
    metrics: IOMetrics,
    baseline_throughput: Arc<Mutex<Option<u64>>>,
    buffer_size: Arc<Mutex<usize>>,
    queue_depth: Arc<Mutex<usize>>,
}

impl PerformanceTuner {
    pub fn new() -> Self {
        Self {
            metrics: IOMetrics::new(),
            baseline_throughput: Arc::new(Mutex::new(None)),
            buffer_size: Arc::new(Mutex::new(4 * 1024 * 1024)), // Start with 4MB
            queue_depth: Arc::new(Mutex::new(4)),
        }
    }

    /// Record operation and check for tuning opportunities
    pub fn record_and_tune(&self, bytes: u64, latency: Duration) -> (usize, usize) {
        self.metrics.record_operation(bytes, latency);

        // Update baseline after warmup period (10 seconds)
        if self.metrics.start_time.elapsed() > Duration::from_secs(10) {
            let mut baseline = self.baseline_throughput.lock().unwrap();
            if baseline.is_none() {
                *baseline = Some(self.metrics.throughput());
            }
        }

        // Check if we should tune parameters
        let current_stats = self.metrics.stats();
        self.tune_if_needed(&current_stats);

        // Return current parameters
        (
            *self.buffer_size.lock().unwrap(),
            *self.queue_depth.lock().unwrap(),
        )
    }

    /// Adaptive tuning based on performance
    fn tune_if_needed(&self, stats: &PerformanceStats) {
        // Only tune after warmup
        if stats.elapsed < Duration::from_secs(10) {
            return;
        }

        let baseline = self.baseline_throughput.lock().unwrap();
        if let Some(baseline_throughput) = *baseline {
            let current = stats.throughput_bps;

            // If throughput is degrading, try increasing buffer size
            if current < baseline_throughput * 8 / 10 {
                // < 80% of baseline
                let mut buffer_size = self.buffer_size.lock().unwrap();
                if *buffer_size < 16 * 1024 * 1024 {
                    // Max 16MB
                    *buffer_size *= 2;
                    println!(
                        "📈 Tuning: Increased buffer size to {} MB",
                        *buffer_size / (1024 * 1024)
                    );
                }
            }

            // If latency is too high, reduce queue depth
            if stats.avg_latency > Duration::from_millis(100) {
                let mut queue_depth = self.queue_depth.lock().unwrap();
                if *queue_depth > 2 {
                    *queue_depth = (*queue_depth * 3) / 4; // Reduce by 25%
                    println!("📉 Tuning: Reduced queue depth to {}", *queue_depth);
                }
            }

            // If IOPS is low and latency is good, increase queue depth
            if stats.iops < 1000 && stats.avg_latency < Duration::from_millis(10) {
                let mut queue_depth = self.queue_depth.lock().unwrap();
                if *queue_depth < 32 {
                    *queue_depth += 2;
                    println!("📈 Tuning: Increased queue depth to {}", *queue_depth);
                }
            }
        }
    }

    /// Get current metrics
    pub fn metrics(&self) -> &IOMetrics {
        &self.metrics
    }

    /// Get current buffer size
    pub fn buffer_size(&self) -> usize {
        *self.buffer_size.lock().unwrap()
    }

    /// Get current queue depth
    pub fn queue_depth(&self) -> usize {
        *self.queue_depth.lock().unwrap()
    }
}

impl Default for PerformanceTuner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_recording() {
        let metrics = IOMetrics::new();

        metrics.record_operation(1024 * 1024, Duration::from_millis(10));
        metrics.record_operation(1024 * 1024, Duration::from_millis(15));

        let stats = metrics.stats();
        assert_eq!(stats.bytes_processed, 2 * 1024 * 1024);
        assert_eq!(stats.operations_count, 2);
    }

    #[test]
    fn test_throughput_calculation() {
        let metrics = IOMetrics::new();

        std::thread::sleep(Duration::from_millis(100));
        metrics.record_operation(100 * 1024 * 1024, Duration::from_millis(1));

        let throughput = metrics.throughput();
        assert!(throughput > 0);
    }

    #[test]
    fn test_latency_percentiles() {
        let metrics = IOMetrics::new();

        for i in 1..=100 {
            metrics.record_operation(1024, Duration::from_millis(i));
        }

        let p50 = metrics.latency_percentile(50.0);
        let p95 = metrics.latency_percentile(95.0);

        assert!(p50 < p95);
        assert!(p50.as_millis() >= 40 && p50.as_millis() <= 60);
    }

    #[test]
    fn test_performance_degradation() {
        let metrics = IOMetrics::new();

        assert!(!metrics.is_degraded(1000));

        // Sleep a bit to ensure elapsed time passes
        std::thread::sleep(Duration::from_millis(100));

        // Record slow operation: 100 bytes in 1 second = very slow
        metrics.record_operation(100, Duration::from_secs(1));

        // Sleep again to ensure throughput calculation has meaningful elapsed time
        std::thread::sleep(Duration::from_millis(100));

        // Throughput should now be low enough to be considered degraded
        assert!(metrics.is_degraded(1000));
    }
}
