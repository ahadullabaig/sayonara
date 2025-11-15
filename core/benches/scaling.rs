/// Scaling benchmarks for concurrent operations
///
/// Measures performance scaling from 1 to 10 concurrent drive operations.
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::io::Write;
use std::thread;
use tempfile::NamedTempFile;

// Helper to create and write to a file
fn write_file(size_mb: u64, buffer_size: usize) -> std::io::Result<()> {
    let mut file = NamedTempFile::new()?;
    let buffer = vec![0xAB; buffer_size];
    let total_size = size_mb * 1024 * 1024;
    let mut written = 0u64;

    while written < total_size {
        let remaining = total_size - written;
        let write_size = remaining.min(buffer_size as u64);
        file.write_all(&buffer[..write_size as usize])?;
        written += write_size;
    }

    file.flush()?;
    Ok(())
}

// Benchmark concurrent file writes
fn bench_concurrent_writes(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_writes");

    let file_size_mb = 5; // 5MB per file
    let buffer_size = 1024 * 1024; // 1MB buffer

    for num_threads in [1, 2, 4, 8] {
        let total_bytes = (file_size_mb * num_threads * 1024 * 1024) as u64;
        group.throughput(Throughput::Bytes(total_bytes));

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}threads", num_threads)),
            &num_threads,
            |b, &threads| {
                b.iter(|| {
                    let handles: Vec<_> = (0..threads)
                        .map(|_| {
                            thread::spawn(move || {
                                write_file(file_size_mb, buffer_size).unwrap();
                            })
                        })
                        .collect();

                    for handle in handles {
                        handle.join().unwrap();
                    }
                });
            },
        );
    }

    group.finish();
}

// Benchmark concurrent pattern generation
fn bench_concurrent_pattern_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_pattern_generation");

    let buffer_size = 4 * 1024 * 1024; // 4MB per thread

    for num_threads in [1, 2, 4, 8] {
        let total_bytes = (buffer_size * num_threads) as u64;
        group.throughput(Throughput::Bytes(total_bytes));

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}threads", num_threads)),
            &num_threads,
            |b, &threads| {
                b.iter(|| {
                    let handles: Vec<_> = (0..threads)
                        .map(|_| {
                            thread::spawn(move || {
                                let buffer = vec![0xCD; buffer_size];
                                black_box(buffer);
                            })
                        })
                        .collect();

                    for handle in handles {
                        handle.join().unwrap();
                    }
                });
            },
        );
    }

    group.finish();
}

// Benchmark thread spawn overhead
fn bench_thread_spawn_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("thread_spawn_overhead");

    for num_threads in [1, 2, 4, 8, 16] {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}threads", num_threads)),
            &num_threads,
            |b, &threads| {
                b.iter(|| {
                    let handles: Vec<_> = (0..threads)
                        .map(|_| {
                            thread::spawn(|| {
                                // Minimal work
                                black_box(42);
                            })
                        })
                        .collect();

                    for handle in handles {
                        handle.join().unwrap();
                    }
                });
            },
        );
    }

    group.finish();
}

// Benchmark parallel buffer initialization
fn bench_parallel_buffer_init(c: &mut Criterion) {
    let mut group = c.benchmark_group("parallel_buffer_init");

    let total_size = 64 * 1024 * 1024; // 64MB total

    for num_threads in [1, 2, 4, 8] {
        group.throughput(Throughput::Bytes(total_size as u64));

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}threads", num_threads)),
            &num_threads,
            |b, &threads| {
                b.iter(|| {
                    let chunk_size = total_size / threads;

                    let handles: Vec<_> = (0..threads)
                        .map(|_| {
                            thread::spawn(move || {
                                let buffer = vec![0xEF; chunk_size];
                                black_box(buffer);
                            })
                        })
                        .collect();

                    for handle in handles {
                        handle.join().unwrap();
                    }
                });
            },
        );
    }

    group.finish();
}

// Benchmark workload distribution efficiency
fn bench_workload_distribution(c: &mut Criterion) {
    let mut group = c.benchmark_group("workload_distribution");

    let total_work = 1000; // Number of small work units

    for num_threads in [1, 2, 4, 8] {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}threads", num_threads)),
            &num_threads,
            |b, &threads| {
                b.iter(|| {
                    let work_per_thread = total_work / threads;

                    let handles: Vec<_> = (0..threads)
                        .map(|_| {
                            thread::spawn(move || {
                                let mut sum = 0u64;
                                for i in 0..work_per_thread {
                                    sum = sum.wrapping_add(i as u64);
                                }
                                black_box(sum);
                            })
                        })
                        .collect();

                    for handle in handles {
                        handle.join().unwrap();
                    }
                });
            },
        );
    }

    group.finish();
}

// Benchmark lock-free vs locked operations
fn bench_synchronization_overhead(c: &mut Criterion) {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::{Arc, Mutex};

    let mut group = c.benchmark_group("synchronization_overhead");

    let num_threads = 4;
    let increments_per_thread = 10000;

    // Atomic increment (lock-free)
    group.bench_function("atomic_increment", |b| {
        b.iter(|| {
            let counter = Arc::new(AtomicU64::new(0));

            let handles: Vec<_> = (0..num_threads)
                .map(|_| {
                    let counter = Arc::clone(&counter);
                    thread::spawn(move || {
                        for _ in 0..increments_per_thread {
                            counter.fetch_add(1, Ordering::Relaxed);
                        }
                    })
                })
                .collect();

            for handle in handles {
                handle.join().unwrap();
            }

            black_box(counter.load(Ordering::Relaxed));
        });
    });

    // Mutex-protected increment
    group.bench_function("mutex_increment", |b| {
        b.iter(|| {
            let counter = Arc::new(Mutex::new(0u64));

            let handles: Vec<_> = (0..num_threads)
                .map(|_| {
                    let counter = Arc::clone(&counter);
                    thread::spawn(move || {
                        for _ in 0..increments_per_thread {
                            let mut num = counter.lock().unwrap();
                            *num += 1;
                        }
                    })
                })
                .collect();

            for handle in handles {
                handle.join().unwrap();
            }

            black_box(*counter.lock().unwrap());
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_concurrent_writes,
    bench_concurrent_pattern_generation,
    bench_thread_spawn_overhead,
    bench_parallel_buffer_init,
    bench_workload_distribution,
    bench_synchronization_overhead,
);
criterion_main!(benches);
