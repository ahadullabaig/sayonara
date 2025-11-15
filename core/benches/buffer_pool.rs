/// Buffer pool benchmarks
///
/// Measures buffer allocation, reuse, and efficiency.

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use std::alloc::{alloc, dealloc, Layout};

// Simulate aligned buffer allocation
fn allocate_aligned_buffer(size: usize, alignment: usize) -> (*mut u8, Layout) {
    let layout = Layout::from_size_align(size, alignment).unwrap();
    unsafe {
        let ptr = alloc(layout);
        assert!(!ptr.is_null(), "Allocation failed");
        (ptr, layout)
    }
}

fn deallocate_aligned_buffer(ptr: *mut u8, layout: Layout) {
    unsafe {
        dealloc(ptr, layout);
    }
}

// Benchmark buffer allocation strategies
fn bench_buffer_allocation(c: &mut Criterion) {
    let mut group = c.benchmark_group("buffer_allocation");

    let sizes = vec![
        ("4KB", 4 * 1024),
        ("64KB", 64 * 1024),
        ("1MB", 1024 * 1024),
        ("4MB", 4 * 1024 * 1024),
        ("16MB", 16 * 1024 * 1024),
    ];

    let alignment = 4096; // Page-aligned

    for (name, size) in sizes {
        group.throughput(Throughput::Bytes(size as u64));

        group.bench_with_input(
            BenchmarkId::new("allocate_deallocate", name),
            &size,
            |b, &size| {
                b.iter(|| {
                    let (ptr, layout) = allocate_aligned_buffer(size, alignment);
                    deallocate_aligned_buffer(ptr, layout);
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("allocate_use_deallocate", name),
            &size,
            |b, &size| {
                b.iter(|| {
                    let (ptr, layout) = allocate_aligned_buffer(size, alignment);

                    // Simulate usage by writing to buffer
                    unsafe {
                        std::ptr::write_bytes(ptr, 0xAB, size);
                    }

                    deallocate_aligned_buffer(ptr, layout);
                });
            },
        );
    }

    group.finish();
}

// Benchmark buffer reuse pattern
fn bench_buffer_reuse(c: &mut Criterion) {
    let mut group = c.benchmark_group("buffer_reuse");

    let buffer_size = 4 * 1024 * 1024; // 4MB
    let alignment = 4096;
    let num_operations = 100;

    group.throughput(Throughput::Bytes((buffer_size * num_operations) as u64));

    // No reuse - allocate and deallocate each time
    group.bench_function("no_reuse", |b| {
        b.iter(|| {
            for _ in 0..num_operations {
                let (ptr, layout) = allocate_aligned_buffer(buffer_size, alignment);
                unsafe {
                    std::ptr::write_bytes(ptr, 0xCD, buffer_size);
                }
                deallocate_aligned_buffer(ptr, layout);
            }
        });
    });

    // With reuse - allocate once, use many times
    group.bench_function("with_reuse", |b| {
        b.iter(|| {
            let (ptr, layout) = allocate_aligned_buffer(buffer_size, alignment);

            for _ in 0..num_operations {
                unsafe {
                    std::ptr::write_bytes(ptr, 0xCD, buffer_size);
                }
            }

            deallocate_aligned_buffer(ptr, layout);
        });
    });

    group.finish();
}

// Benchmark buffer pool with recycling
fn bench_buffer_pool_recycling(c: &mut Criterion) {
    use std::collections::VecDeque;

    let mut group = c.benchmark_group("buffer_pool_recycling");

    let buffer_size = 1024 * 1024; // 1MB
    let alignment = 4096;
    let pool_size = 8;
    let num_operations = 50;

    group.throughput(Throughput::Bytes((buffer_size * num_operations) as u64));

    // Naive: no pooling
    group.bench_function("no_pooling", |b| {
        b.iter(|| {
            for _ in 0..num_operations {
                let (ptr, layout) = allocate_aligned_buffer(buffer_size, alignment);
                unsafe { std::ptr::write_bytes(ptr, 0xEF, buffer_size); }
                deallocate_aligned_buffer(ptr, layout);
            }
        });
    });

    // With pooling
    group.bench_function("with_pooling", |b| {
        b.iter(|| {
            // Pre-allocate pool
            let mut pool: VecDeque<(*mut u8, Layout)> = VecDeque::new();
            for _ in 0..pool_size {
                pool.push_back(allocate_aligned_buffer(buffer_size, alignment));
            }

            // Use and recycle
            for _ in 0..num_operations {
                let (ptr, layout) = pool.pop_front().unwrap_or_else(|| {
                    allocate_aligned_buffer(buffer_size, alignment)
                });

                unsafe { std::ptr::write_bytes(ptr, 0xEF, buffer_size); }

                pool.push_back((ptr, layout));
            }

            // Cleanup
            while let Some((ptr, layout)) = pool.pop_front() {
                deallocate_aligned_buffer(ptr, layout);
            }
        });
    });

    group.finish();
}

// Benchmark alignment overhead
fn bench_alignment_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("alignment_overhead");

    let size = 1024 * 1024; // 1MB

    let alignments = vec![
        ("512B", 512),
        ("4KB", 4096),
        ("2MB", 2 * 1024 * 1024),
    ];

    for (name, alignment) in alignments {
        group.bench_with_input(
            BenchmarkId::from_parameter(name),
            &alignment,
            |b, &alignment| {
                b.iter(|| {
                    let (ptr, layout) = allocate_aligned_buffer(size, alignment);
                    unsafe {
                        std::ptr::write_bytes(ptr, 0xAB, size);
                    }
                    deallocate_aligned_buffer(ptr, layout);
                });
            },
        );
    }

    group.finish();
}

// Benchmark buffer initialization strategies
fn bench_buffer_initialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("buffer_initialization");

    let size = 4 * 1024 * 1024; // 4MB

    group.throughput(Throughput::Bytes(size as u64));

    // Uninitialized (unsafe but fastest)
    group.bench_function("uninitialized", |b| {
        b.iter(|| {
            let buffer: Vec<u8> = Vec::with_capacity(size);
            black_box(buffer);
        });
    });

    // Zero-initialized
    group.bench_function("zeroed", |b| {
        b.iter(|| {
            let buffer = vec![0u8; size];
            black_box(buffer);
        });
    });

    // Pattern-initialized
    group.bench_function("pattern", |b| {
        b.iter(|| {
            let buffer = vec![0xAB; size];
            black_box(buffer);
        });
    });

    // Lazy initialization (allocate then fill)
    group.bench_function("lazy_fill", |b| {
        b.iter(|| {
            let mut buffer = Vec::with_capacity(size);
            buffer.resize(size, 0xCD);
            black_box(buffer);
        });
    });

    group.finish();
}

// Benchmark memory bandwidth
fn bench_memory_bandwidth(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_bandwidth");

    let size = 16 * 1024 * 1024; // 16MB

    group.throughput(Throughput::Bytes(size as u64));

    // Sequential write
    group.bench_function("sequential_write", |b| {
        b.iter(|| {
            let mut buffer = vec![0u8; size];
            for i in 0..size {
                buffer[i] = 0xAB;
            }
            black_box(buffer);
        });
    });

    // Sequential read
    group.bench_function("sequential_read", |b| {
        let buffer = vec![0xAB; size];
        b.iter(|| {
            let mut sum = 0u64;
            for &byte in &buffer {
                sum = sum.wrapping_add(byte as u64);
            }
            black_box(sum);
        });
    });

    // Copy
    group.bench_function("copy", |b| {
        let src = vec![0xAB; size];
        b.iter(|| {
            let dst = src.clone();
            black_box(dst);
        });
    });

    // Memset-style
    group.bench_function("memset", |b| {
        b.iter(|| {
            let mut buffer = vec![0u8; size];
            buffer.fill(0xCD);
            black_box(buffer);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_buffer_allocation,
    bench_buffer_reuse,
    bench_buffer_pool_recycling,
    bench_alignment_overhead,
    bench_buffer_initialization,
    bench_memory_bandwidth,
);
criterion_main!(benches);
