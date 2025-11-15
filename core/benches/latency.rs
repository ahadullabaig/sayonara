/// Latency benchmarks for Sayonara I/O operations
///
/// Measures operation latency (p50, p95, p99) for various I/O operations.

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use std::fs::{File, OpenOptions};
use std::io::{Write, Read, Seek, SeekFrom};
use std::time::Duration;
use tempfile::NamedTempFile;

// Create a pre-populated temp file for read latency tests
fn create_populated_file(size_mb: u64) -> std::io::Result<NamedTempFile> {
    let mut file = NamedTempFile::new()?;
    let size_bytes = size_mb * 1024 * 1024;
    let chunk = vec![0xCD; 1024 * 1024]; // 1MB chunks

    let mut written = 0u64;
    while written < size_bytes {
        let remaining = size_bytes - written;
        let write_size = remaining.min(1024 * 1024);
        file.write_all(&chunk[..write_size as usize])?;
        written += write_size;
    }

    file.flush()?;
    file.seek(SeekFrom::Start(0))?;
    Ok(file)
}

// Benchmark file open latency
fn bench_file_open_latency(c: &mut Criterion) {
    let mut group = c.benchmark_group("file_open_latency");
    group.measurement_time(Duration::from_secs(10));

    // Create a temp file to open repeatedly
    let temp_file = NamedTempFile::new().unwrap();
    let path = temp_file.path().to_path_buf();

    group.bench_function("open_read_only", |b| {
        b.iter(|| {
            let file = File::open(&path).unwrap();
            black_box(file);
        });
    });

    group.bench_function("open_read_write", |b| {
        b.iter(|| {
            let file = OpenOptions::new()
                .read(true)
                .write(true)
                .open(&path)
                .unwrap();
            black_box(file);
        });
    });

    #[cfg(target_os = "linux")]
    group.bench_function("open_with_direct_io", |b| {
        use std::os::unix::fs::OpenOptionsExt;

        b.iter(|| {
            let file = OpenOptions::new()
                .read(true)
                .write(true)
                .custom_flags(libc::O_DIRECT)
                .open(&path)
                .unwrap_or_else(|_| File::open(&path).unwrap()); // Fallback if O_DIRECT fails
            black_box(file);
        });
    });

    group.finish();
}

// Benchmark single write operation latency
fn bench_write_latency(c: &mut Criterion) {
    let mut group = c.benchmark_group("write_latency");
    group.measurement_time(Duration::from_secs(10));

    let sizes = vec![
        ("4KB", 4 * 1024),
        ("64KB", 64 * 1024),
        ("1MB", 1024 * 1024),
        ("4MB", 4 * 1024 * 1024),
    ];

    for (name, size) in sizes {
        group.bench_with_input(BenchmarkId::from_parameter(name), &size, |b, &size| {
            let buffer = vec![0xAB; size];

            b.iter(|| {
                let mut file = NamedTempFile::new().unwrap();
                file.write_all(&buffer).unwrap();
                black_box(file);
            });
        });
    }

    group.finish();
}

// Benchmark read operation latency
fn bench_read_latency(c: &mut Criterion) {
    let mut group = c.benchmark_group("read_latency");
    group.measurement_time(Duration::from_secs(10));

    let file = create_populated_file(10).unwrap(); // 10MB file
    let path = file.path().to_path_buf();

    let sizes = vec![
        ("4KB", 4 * 1024),
        ("64KB", 64 * 1024),
        ("1MB", 1024 * 1024),
        ("4MB", 4 * 1024 * 1024),
    ];

    for (name, size) in sizes {
        group.bench_with_input(BenchmarkId::from_parameter(name), &size, |b, &size| {
            b.iter(|| {
                let mut file = File::open(&path).unwrap();
                let mut buffer = vec![0u8; size];
                file.read_exact(&mut buffer).unwrap();
                black_box(buffer);
            });
        });
    }

    group.finish();
}

// Benchmark seek operation latency
fn bench_seek_latency(c: &mut Criterion) {
    let mut group = c.benchmark_group("seek_latency");
    group.measurement_time(Duration::from_secs(10));

    let file_size = 100; // 100MB
    let file = create_populated_file(file_size).unwrap();
    let path = file.path().to_path_buf();

    group.bench_function("seek_sequential", |b| {
        b.iter(|| {
            let mut file = File::open(&path).unwrap();
            for offset_mb in 0..10 {
                let offset = offset_mb * 1024 * 1024;
                file.seek(SeekFrom::Start(offset)).unwrap();
            }
            black_box(file);
        });
    });

    group.bench_function("seek_random", |b| {
        use std::collections::hash_map::RandomState;
        use std::hash::{BuildHasher, Hasher};

        let state = RandomState::new();
        let mut hasher = state.build_hasher();

        b.iter(|| {
            let mut file = File::open(&path).unwrap();
            for i in 0..10 {
                hasher.write_usize(i);
                let offset = (hasher.finish() % (file_size * 1024 * 1024)) as u64;
                file.seek(SeekFrom::Start(offset)).unwrap();
            }
            black_box(file);
        });
    });

    group.finish();
}

// Benchmark flush/sync latency
fn bench_sync_latency(c: &mut Criterion) {
    let mut group = c.benchmark_group("sync_latency");
    group.measurement_time(Duration::from_secs(10));

    let buffer = vec![0xCD; 1024 * 1024]; // 1MB

    group.bench_function("flush_only", |b| {
        b.iter(|| {
            let mut file = NamedTempFile::new().unwrap();
            file.write_all(&buffer).unwrap();
            file.flush().unwrap();
            black_box(file);
        });
    });

    group.bench_function("sync_data", |b| {
        b.iter(|| {
            let mut file = NamedTempFile::new().unwrap();
            file.write_all(&buffer).unwrap();
            file.as_file().sync_data().unwrap();
            black_box(file);
        });
    });

    group.bench_function("sync_all", |b| {
        b.iter(|| {
            let mut file = NamedTempFile::new().unwrap();
            file.write_all(&buffer).unwrap();
            file.as_file().sync_all().unwrap();
            black_box(file);
        });
    });

    group.finish();
}

// Benchmark metadata operations
fn bench_metadata_latency(c: &mut Criterion) {
    let mut group = c.benchmark_group("metadata_latency");
    group.measurement_time(Duration::from_secs(10));

    let file = create_populated_file(10).unwrap();
    let path = file.path().to_path_buf();

    group.bench_function("get_metadata", |b| {
        b.iter(|| {
            let metadata = std::fs::metadata(&path).unwrap();
            black_box(metadata);
        });
    });

    group.bench_function("get_file_size", |b| {
        b.iter(|| {
            let file = File::open(&path).unwrap();
            let size = file.metadata().unwrap().len();
            black_box(size);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_file_open_latency,
    bench_write_latency,
    bench_read_latency,
    bench_seek_latency,
    bench_sync_latency,
    bench_metadata_latency,
);
criterion_main!(benches);
