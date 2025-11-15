/// Adaptive tuning benchmarks
///
/// Measures performance tuning and metrics tracking overhead.
use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use std::time::{Duration, Instant};

// Simplified metrics tracker for benchmarking
struct SimpleMetrics {
    start_time: Instant,
    bytes_processed: u64,
    operations: u64,
}

impl SimpleMetrics {
    fn new() -> Self {
        Self {
            start_time: Instant::now(),
            bytes_processed: 0,
            operations: 0,
        }
    }

    fn record(&mut self, bytes: u64) {
        self.bytes_processed += bytes;
        self.operations += 1;
    }

    fn throughput(&self) -> u64 {
        let elapsed = self.start_time.elapsed().as_secs_f64();
        if elapsed < 0.001 {
            return 0;
        }
        (self.bytes_processed as f64 / elapsed) as u64
    }
}

// Benchmark metrics recording overhead
fn bench_metrics_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("metrics_overhead");

    let num_operations = 10000;
    let bytes_per_op = 4096;

    group.throughput(Throughput::Bytes((num_operations * bytes_per_op) as u64));

    // No metrics (baseline)
    group.bench_function("no_metrics", |b| {
        b.iter(|| {
            let mut sum = 0u64;
            for _ in 0..num_operations {
                sum = sum.wrapping_add(bytes_per_op);
            }
            black_box(sum);
        });
    });

    // With simple metrics
    group.bench_function("with_simple_metrics", |b| {
        b.iter(|| {
            let mut metrics = SimpleMetrics::new();
            for _ in 0..num_operations {
                metrics.record(bytes_per_op);
            }
            black_box(metrics.throughput());
        });
    });

    group.finish();
}

// Benchmark throughput calculation methods
fn bench_throughput_calculation(c: &mut Criterion) {
    let mut group = c.benchmark_group("throughput_calculation");

    let bytes_processed = 1_000_000_000u64; // 1GB
    let elapsed_secs = 10.0;

    // Direct division
    group.bench_function("direct_division", |b| {
        b.iter(|| {
            let throughput = (bytes_processed as f64 / elapsed_secs) as u64;
            black_box(throughput);
        });
    });

    // With MB/s conversion
    group.bench_function("with_mb_conversion", |b| {
        b.iter(|| {
            let throughput = (bytes_processed as f64 / elapsed_secs) as u64;
            let mb_per_sec = throughput / (1024 * 1024);
            black_box(mb_per_sec);
        });
    });

    // With formatting
    group.bench_function("with_formatting", |b| {
        b.iter(|| {
            let throughput = (bytes_processed as f64 / elapsed_secs) as u64;
            let mb_per_sec = throughput / (1024 * 1024);
            let formatted = format!("{} MB/s", mb_per_sec);
            black_box(formatted);
        });
    });

    group.finish();
}

// Benchmark adaptive buffer size tuning
fn bench_adaptive_buffer_sizing(c: &mut Criterion) {
    let mut group = c.benchmark_group("adaptive_buffer_sizing");

    // Simulate different throughput levels
    let throughput_levels = vec![
        ("slow_hdd", 100 * 1024 * 1024u64),  // 100 MB/s
        ("fast_hdd", 200 * 1024 * 1024u64),  // 200 MB/s
        ("sata_ssd", 500 * 1024 * 1024u64),  // 500 MB/s
        ("nvme_ssd", 3000 * 1024 * 1024u64), // 3 GB/s
    ];

    for (name, throughput) in throughput_levels {
        group.bench_function(name, |b| {
            b.iter(|| {
                // Simulate adaptive buffer size selection
                let buffer_size = if throughput < 150 * 1024 * 1024 {
                    4 * 1024 * 1024 // 4MB for HDD
                } else if throughput < 1000 * 1024 * 1024 {
                    8 * 1024 * 1024 // 8MB for SATA SSD
                } else {
                    16 * 1024 * 1024 // 16MB for NVMe
                };
                black_box(buffer_size);
            });
        });
    }

    group.finish();
}

// Benchmark performance degradation detection
fn bench_degradation_detection(c: &mut Criterion) {
    let mut group = c.benchmark_group("degradation_detection");

    let baseline_throughput = 500 * 1024 * 1024u64; // 500 MB/s

    let test_cases = vec![
        ("normal", 480 * 1024 * 1024),     // 96% of baseline
        ("slight_deg", 350 * 1024 * 1024), // 70% of baseline
        ("degraded", 200 * 1024 * 1024),   // 40% of baseline
        ("severe_deg", 100 * 1024 * 1024), // 20% of baseline
    ];

    for (name, current_throughput) in test_cases {
        group.bench_function(name, |b| {
            b.iter(|| {
                // Detect if throughput dropped below 50% of baseline
                let is_degraded = current_throughput < baseline_throughput / 2;
                black_box(is_degraded);
            });
        });
    }

    group.finish();
}

// Benchmark queue depth adaptation
fn bench_queue_depth_adaptation(c: &mut Criterion) {
    let mut group = c.benchmark_group("queue_depth_adaptation");

    // Simulate latency-based queue depth tuning
    let latency_scenarios = vec![
        ("low_latency", 100),     // 100 microseconds
        ("medium_latency", 1000), // 1 millisecond
        ("high_latency", 10000),  // 10 milliseconds
    ];

    for (name, latency_us) in latency_scenarios {
        group.bench_function(name, |b| {
            b.iter(|| {
                // Adaptive queue depth based on latency
                let queue_depth = if latency_us < 500 {
                    32 // High queue depth for low latency (NVMe)
                } else if latency_us < 5000 {
                    8 // Medium queue depth for medium latency (SSD)
                } else {
                    2 // Low queue depth for high latency (HDD)
                };
                black_box(queue_depth);
            });
        });
    }

    group.finish();
}

// Benchmark temperature monitoring overhead
fn bench_temperature_monitoring(c: &mut Criterion) {
    let mut group = c.benchmark_group("temperature_monitoring");

    let check_interval_bytes = 100 * 1024 * 1024; // 100MB
    let bytes_per_op = 4 * 1024 * 1024; // 4MB

    // Without temperature checks
    group.bench_function("no_temp_check", |b| {
        b.iter(|| {
            let mut bytes_written = 0u64;
            for _ in 0..(check_interval_bytes / bytes_per_op) {
                bytes_written += bytes_per_op;
            }
            black_box(bytes_written);
        });
    });

    // With temperature check simulation
    group.bench_function("with_temp_check", |b| {
        b.iter(|| {
            let mut bytes_written = 0u64;
            let mut simulated_temp = 45u32; // Celsius

            for _ in 0..(check_interval_bytes / bytes_per_op) {
                bytes_written += bytes_per_op;

                // Simulate temperature check every interval
                if bytes_written % check_interval_bytes == 0 {
                    // Simulate reading temperature
                    simulated_temp += 1;

                    // Check threshold
                    if simulated_temp > 65 {
                        // Simulate throttling
                        std::thread::sleep(Duration::from_millis(1));
                    }
                }
            }
            black_box(bytes_written);
        });
    });

    group.finish();
}

// Benchmark efficiency calculation
fn bench_efficiency_calculation(c: &mut Criterion) {
    let mut group = c.benchmark_group("efficiency_calculation");

    let theoretical_max = 3000 * 1024 * 1024u64; // 3 GB/s
    let test_throughputs = vec![
        ("low", 500 * 1024 * 1024u64),
        ("medium", 1500 * 1024 * 1024u64),
        ("high", 2700 * 1024 * 1024u64),
    ];

    for (name, actual_throughput) in test_throughputs {
        group.bench_function(name, |b| {
            b.iter(|| {
                let efficiency = (actual_throughput as f64 / theoretical_max as f64) * 100.0;
                black_box(efficiency);
            });
        });
    }

    group.finish();
}

// Benchmark latency percentile calculation
fn bench_latency_percentiles(c: &mut Criterion) {
    let mut group = c.benchmark_group("latency_percentiles");

    // Create sample latencies (in microseconds)
    let mut latencies: Vec<u64> = Vec::new();
    for i in 0..1000 {
        // Simulate realistic latency distribution
        let base = 100 + (i % 100);
        let spike = if i % 50 == 0 { 500 } else { 0 };
        latencies.push(base + spike);
    }

    group.bench_function("p50", |b| {
        b.iter(|| {
            let mut sorted = latencies.clone();
            sorted.sort_unstable();
            let index = (50.0 / 100.0 * sorted.len() as f64) as usize;
            black_box(sorted[index.min(sorted.len() - 1)]);
        });
    });

    group.bench_function("p95", |b| {
        b.iter(|| {
            let mut sorted = latencies.clone();
            sorted.sort_unstable();
            let index = (95.0 / 100.0 * sorted.len() as f64) as usize;
            black_box(sorted[index.min(sorted.len() - 1)]);
        });
    });

    group.bench_function("p99", |b| {
        b.iter(|| {
            let mut sorted = latencies.clone();
            sorted.sort_unstable();
            let index = (99.0 / 100.0 * sorted.len() as f64) as usize;
            black_box(sorted[index.min(sorted.len() - 1)]);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_metrics_overhead,
    bench_throughput_calculation,
    bench_adaptive_buffer_sizing,
    bench_degradation_detection,
    bench_queue_depth_adaptation,
    bench_temperature_monitoring,
    bench_efficiency_calculation,
    bench_latency_percentiles,
);
criterion_main!(benches);
