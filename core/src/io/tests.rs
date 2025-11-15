#[cfg(test)]
mod tests {
    use crate::io::*;
    use tempfile::NamedTempFile;
    use std::time::Instant;
    use crate::io::metrics::PerformanceTuner;
    use serial_test::serial;

    type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

    #[test]
    #[serial]
    fn test_sequential_write_performance() -> Result<()> {
        // Reset interrupt flag in case other tests set it
        crate::reset_interrupted();

        let temp = NamedTempFile::new()?;
        let path = temp.path().to_str().unwrap();

        // Use buffered I/O for testing (Direct I/O requires block device)
        let mut config = IOConfig::default();
        config.use_direct_io = false;
        config.initial_buffer_size = 1 * 1024 * 1024;  // 1MB for test

        let mut handle = OptimizedIO::open(path, config)?;

        // Write 10MB of data
        let test_size = 10 * 1024 * 1024u64;
        let pattern = vec![0xAA, 0xBB, 0xCC, 0xDD];

        let start = Instant::now();

        OptimizedIO::sequential_write(&mut handle, test_size, |buffer| {
            buffer.fill(&pattern);
            Ok(())
        })?;

        let elapsed = start.elapsed();
        let throughput = test_size as f64 / elapsed.as_secs_f64();

        println!("Test throughput: {:.2} MB/s", throughput / (1024.0 * 1024.0));

        // Check metrics
        let stats = handle.metrics().stats();
        assert_eq!(stats.bytes_processed, test_size);
        assert!(stats.throughput_bps > 0);

        Ok(())
    }

    #[test]
    fn test_buffer_pool_reuse() -> Result<()> {
        let pool = BufferPool::direct_io_pool(4096, 5);
        pool.preallocate(3)?;

        let initial_stats = pool.stats();
        assert_eq!(initial_stats.available, 3);

        // Acquire and release buffers
        {
            let _b1 = pool.acquire()?;
            let _b2 = pool.acquire()?;

            let stats = pool.stats();
            assert_eq!(stats.available, 1);
        }

        // Buffers should be returned
        let final_stats = pool.stats();
        assert_eq!(final_stats.available, 3);

        Ok(())
    }

    #[test]
    fn test_metrics_accuracy() -> Result<()> {
        let metrics = IOMetrics::new();

        // Record some operations
        for i in 1..=100 {
            metrics.record_operation(
                1024,
                std::time::Duration::from_micros(i * 10)
            );
        }

        let stats = metrics.stats();
        assert_eq!(stats.bytes_processed, 100 * 1024);
        assert_eq!(stats.operations_count, 100);
        assert!(stats.avg_latency.as_micros() > 0);

        Ok(())
    }

    #[test]
    fn test_drive_speed_detection() {
        // Slow drive
        let slow = DriveSpeed::from_throughput(50 * 1024 * 1024);
        assert_eq!(slow, DriveSpeed::Slow);
        assert_eq!(slow.optimal_buffer_size(), 1 * 1024 * 1024);

        // Fast NVMe
        let fast = DriveSpeed::from_throughput(800 * 1024 * 1024);
        assert_eq!(fast, DriveSpeed::VeryFast);
        assert_eq!(fast.optimal_buffer_size(), 16 * 1024 * 1024);
    }

    #[test]
    fn test_aligned_buffer_zeroing() -> Result<()> {
        let mut buffer = AlignedBuffer::page_aligned(4096)?;

        // Fill with pattern
        buffer.fill(&[0xFF]);
        assert_eq!(buffer.as_slice()[0], 0xFF);

        // Zero it
        buffer.zero();
        assert_eq!(buffer.as_slice()[0], 0x00);
        assert_eq!(buffer.as_slice()[4095], 0x00);

        Ok(())
    }

    #[test]
    fn test_throttle_calculation() {
        let config = IOConfig::default();
        let handle = create_test_handle(&config);

        // Below threshold
        let action = handle.calculate_throttle(60);
        assert!(matches!(action, ThrottleAction::None));

        // Just above threshold
        let action = handle.calculate_throttle(67);
        if let ThrottleAction::Slow(factor) = action {
            assert!(factor < 1.0);
        } else {
            panic!("Expected Slow action");
        }

        // Way above threshold
        let action = handle.calculate_throttle(80);
        assert!(matches!(action, ThrottleAction::Pause(_)));
    }

    fn create_test_handle(config: &IOConfig) -> IOHandle {
        let temp = NamedTempFile::new().unwrap();
        let path = temp.path().to_str().unwrap();

        let mut test_config = config.clone();
        test_config.use_direct_io = false;

        OptimizedIO::open(path, test_config).unwrap()
    }

    #[test]
    fn test_latency_percentiles() {
        let metrics = IOMetrics::new();

        // Record operations with varying latencies
        for i in 1..=1000 {
            metrics.record_operation(
                1024,
                std::time::Duration::from_micros(i)
            );
        }

        let p50 = metrics.latency_percentile(50.0);
        let p95 = metrics.latency_percentile(95.0);
        let p99 = metrics.latency_percentile(99.0);

        // Verify ordering
        assert!(p50 < p95);
        assert!(p95 < p99);

        // Verify reasonable values
        assert!(p50.as_micros() >= 400 && p50.as_micros() <= 600);
        assert!(p95.as_micros() >= 900 && p95.as_micros() <= 1000);
    }

    #[test]
    fn test_performance_tuner() {
        let tuner = PerformanceTuner::new();

        let initial_buffer = tuner.buffer_size();
        let initial_queue = tuner.queue_depth();

        // Simulate good performance
        for _ in 0..100 {
            tuner.record_and_tune(1024 * 1024, std::time::Duration::from_millis(5));
        }

        // Parameters may have been tuned
        let final_buffer = tuner.buffer_size();
        let final_queue = tuner.queue_depth();

        println!("Buffer: {} -> {}", initial_buffer, final_buffer);
        println!("Queue: {} -> {}", initial_queue, final_queue);

        // At minimum, should not have decreased
        assert!(final_buffer >= initial_buffer || final_queue >= initial_queue);
    }

    #[test]
    fn test_efficiency_calculation() {
        let stats = PerformanceStats {
            elapsed: std::time::Duration::from_secs(10),
            bytes_processed: 100 * 1024 * 1024,
            operations_count: 100,
            errors: 0,
            throughput_bps: 10 * 1024 * 1024,  // 10 MB/s
            iops: 10,
            avg_latency: std::time::Duration::from_millis(100),
            p50_latency: std::time::Duration::from_millis(90),
            p95_latency: std::time::Duration::from_millis(150),
            p99_latency: std::time::Duration::from_millis(200),
        };

        // If theoretical max is 100 MB/s, we're at 10%
        let efficiency = stats.efficiency(100 * 1024 * 1024);
        assert!((efficiency - 10.0).abs() < 0.1);
    }

    // ==================== INTEGRATION TESTS ====================
    // I/O performance integration test has been moved to:
    // tests/hardware_integration.rs::test_io_performance_with_mock
    // This test uses mock drives and can run without physical hardware
}
