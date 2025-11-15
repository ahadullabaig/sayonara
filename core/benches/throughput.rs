/// Throughput benchmarks for Sayonara I/O engine
///
/// Measures write throughput across different drive types and configurations.

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use std::fs::{File, OpenOptions};
use std::io::Write;
use tempfile::NamedTempFile;

// Helper to create a temporary file of given size
fn create_temp_file(size_mb: u64) -> std::io::Result<NamedTempFile> {
    let mut file = NamedTempFile::new()?;
    let size_bytes = size_mb * 1024 * 1024;

    // Pre-allocate with zeros
    let chunk = vec![0u8; 1024 * 1024]; // 1MB chunks
    let mut written = 0u64;

    while written < size_bytes {
        let remaining = size_bytes - written;
        let write_size = remaining.min(1024 * 1024);
        file.write_all(&chunk[..write_size as usize])?;
        written += write_size;
    }

    file.flush()?;
    Ok(file)
}

// Benchmark sequential writes with different buffer sizes
fn bench_buffer_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("sequential_write_buffer_sizes");

    let file_size_mb = 10;
    let buffer_sizes = vec![
        ("4KB", 4 * 1024),
        ("16KB", 16 * 1024),
        ("64KB", 64 * 1024),
        ("256KB", 256 * 1024),
        ("1MB", 1024 * 1024),
        ("4MB", 4 * 1024 * 1024),
        ("8MB", 8 * 1024 * 1024),
    ];

    for (name, buffer_size) in buffer_sizes {
        group.throughput(Throughput::Bytes((file_size_mb * 1024 * 1024) as u64));
        group.bench_with_input(BenchmarkId::from_parameter(name), &buffer_size, |b, &size| {
            b.iter(|| {
                let mut file = create_temp_file(file_size_mb).unwrap();
                let buffer = vec![0xAB; size];
                let mut written = 0u64;
                let total_size = file_size_mb * 1024 * 1024;

                while written < total_size {
                    let remaining = total_size - written;
                    let write_size = remaining.min(size as u64);
                    file.write_all(&buffer[..write_size as usize]).unwrap();
                    written += write_size;
                }

                file.flush().unwrap();
                black_box(file);
            });
        });
    }

    group.finish();
}

// Benchmark simulated drive type throughput
fn bench_drive_types(c: &mut Criterion) {
    let mut group = c.benchmark_group("drive_type_throughput");

    let file_size_mb = 10;

    // Simulate different drive types with different optimal buffer sizes
    let drive_configs = vec![
        ("HDD_4MB_buffer", 4 * 1024 * 1024),
        ("SATA_SSD_8MB_buffer", 8 * 1024 * 1024),
        ("NVMe_16MB_buffer", 16 * 1024 * 1024),
    ];

    for (name, buffer_size) in drive_configs {
        group.throughput(Throughput::Bytes((file_size_mb * 1024 * 1024) as u64));
        group.bench_function(name, |b| {
            b.iter(|| {
                let mut file = create_temp_file(file_size_mb).unwrap();
                let buffer = vec![0xCD; buffer_size];
                let mut written = 0u64;
                let total_size = file_size_mb * 1024 * 1024;

                while written < total_size {
                    let remaining = total_size - written;
                    let write_size = remaining.min(buffer_size as u64);
                    file.write_all(&buffer[..write_size as usize]).unwrap();
                    written += write_size;
                }

                file.flush().unwrap();
                black_box(file);
            });
        });
    }

    group.finish();
}

// Benchmark pattern generation throughput
fn bench_pattern_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("pattern_generation");

    let buffer_size = 4 * 1024 * 1024; // 4MB

    group.throughput(Throughput::Bytes(buffer_size as u64));

    // Zero pattern (fastest)
    group.bench_function("zeros", |b| {
        b.iter(|| {
            let buffer = vec![0u8; buffer_size];
            black_box(buffer);
        });
    });

    // Fixed pattern
    group.bench_function("fixed_pattern", |b| {
        b.iter(|| {
            let buffer = vec![0xAB; buffer_size];
            black_box(buffer);
        });
    });

    // Random pattern (using simple RNG for benchmark)
    group.bench_function("random_pattern", |b| {
        b.iter(|| {
            use std::collections::hash_map::RandomState;
            use std::hash::{BuildHasher, Hash, Hasher};

            let mut buffer = vec![0u8; buffer_size];
            let state = RandomState::new();

            for chunk in buffer.chunks_mut(8) {
                let mut hasher = state.build_hasher();
                let len = chunk.len();
                len.hash(&mut hasher);
                let value = hasher.finish();
                let bytes = value.to_le_bytes();
                let copy_len = len.min(8);
                chunk[..copy_len].copy_from_slice(&bytes[..copy_len]);
            }

            black_box(buffer);
        });
    });

    group.finish();
}

// Benchmark write-sync cycle
fn bench_write_sync_cycle(c: &mut Criterion) {
    let mut group = c.benchmark_group("write_sync_cycle");

    let buffer = vec![0xEF; 1024 * 1024]; // 1MB

    group.throughput(Throughput::Bytes(1024 * 1024));

    // Write without sync
    group.bench_function("write_only", |b| {
        b.iter(|| {
            let mut file = NamedTempFile::new().unwrap();
            file.write_all(&buffer).unwrap();
            black_box(file);
        });
    });

    // Write with flush
    group.bench_function("write_flush", |b| {
        b.iter(|| {
            let mut file = NamedTempFile::new().unwrap();
            file.write_all(&buffer).unwrap();
            file.flush().unwrap();
            black_box(file);
        });
    });

    // Write with sync_all (most thorough)
    group.bench_function("write_sync_all", |b| {
        b.iter(|| {
            let mut file = NamedTempFile::new().unwrap();
            file.write_all(&buffer).unwrap();
            file.as_file().sync_all().unwrap();
            black_box(file);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_buffer_sizes,
    bench_drive_types,
    bench_pattern_generation,
    bench_write_sync_cycle,
);
criterion_main!(benches);
