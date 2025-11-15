// Aligned buffer pool for Direct I/O with huge pages and NUMA support

use super::{IOError, IOResult};
use std::alloc::{alloc, dealloc, Layout};
use std::ptr::NonNull;
use std::sync::{Arc, Mutex};

/// Alignment requirements for Direct I/O
pub const SECTOR_SIZE: usize = 512;
pub const PAGE_SIZE: usize = 4096;
pub const HUGE_PAGE_2MB: usize = 2 * 1024 * 1024;
pub const HUGE_PAGE_1GB: usize = 1024 * 1024 * 1024;

/// Buffer allocation strategy
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AllocStrategy {
    /// Standard malloc/alloc
    Standard,
    /// Use huge pages via mmap (2MB)
    HugePages2MB,
    /// Use huge pages via mmap (1GB)
    HugePages1GB,
    /// NUMA-aware allocation
    NumaAware { node: i32 },
}

/// Aligned buffer for Direct I/O operations
pub struct AlignedBuffer {
    ptr: NonNull<u8>,
    layout: Layout,
    size: usize,
    alignment: usize,
    #[allow(dead_code)] // Used for debugging and introspection
    strategy: AllocStrategy,
    #[cfg(target_os = "linux")]
    mmap_region: Option<(*mut libc::c_void, usize)>,
}

impl AlignedBuffer {
    /// Create a new aligned buffer with strategy
    pub fn new_with_strategy(
        size: usize,
        alignment: usize,
        strategy: AllocStrategy,
    ) -> IOResult<Self> {
        // Ensure alignment is power of 2
        if !alignment.is_power_of_two() {
            return Err(IOError::AlignmentError(format!(
                "Alignment {} is not a power of 2",
                alignment
            )));
        }

        // Ensure size is multiple of alignment
        let aligned_size = (size + alignment - 1) & !(alignment - 1);

        #[cfg(target_os = "linux")]
        {
            match strategy {
                AllocStrategy::HugePages2MB | AllocStrategy::HugePages1GB => {
                    return Self::allocate_huge_pages(aligned_size, alignment, strategy);
                }
                AllocStrategy::NumaAware { node } => {
                    return Self::allocate_numa(aligned_size, alignment, node);
                }
                _ => {}
            }
        }

        // Standard allocation
        let layout = Layout::from_size_align(aligned_size, alignment)
            .map_err(|e| IOError::AllocationFailed(e.to_string()))?;

        let ptr = unsafe {
            let raw_ptr = alloc(layout);
            if raw_ptr.is_null() {
                return Err(IOError::AllocationFailed(format!(
                    "Failed to allocate {} bytes",
                    aligned_size
                )));
            }
            NonNull::new_unchecked(raw_ptr)
        };

        Ok(Self {
            ptr,
            layout,
            size: aligned_size,
            alignment,
            strategy: AllocStrategy::Standard,
            #[cfg(target_os = "linux")]
            mmap_region: None,
        })
    }

    /// Create a new aligned buffer (standard allocation)
    pub fn new(size: usize, alignment: usize) -> IOResult<Self> {
        Self::new_with_strategy(size, alignment, AllocStrategy::Standard)
    }

    #[cfg(target_os = "linux")]
    fn allocate_huge_pages(
        size: usize,
        alignment: usize,
        strategy: AllocStrategy,
    ) -> IOResult<Self> {
        let huge_page_size = match strategy {
            AllocStrategy::HugePages2MB => HUGE_PAGE_2MB,
            AllocStrategy::HugePages1GB => HUGE_PAGE_1GB,
            _ => unreachable!(),
        };

        // Round up to huge page boundary
        let aligned_size = ((size + huge_page_size - 1) / huge_page_size) * huge_page_size;

        unsafe {
            let addr = libc::mmap(
                std::ptr::null_mut(),
                aligned_size,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_PRIVATE | libc::MAP_ANONYMOUS | libc::MAP_HUGETLB,
                -1,
                0,
            );

            if addr == libc::MAP_FAILED {
                // Fallback to standard allocation if huge pages unavailable
                return Self::new_with_strategy(size, alignment, AllocStrategy::Standard);
            }

            // Check alignment
            if (addr as usize) % alignment != 0 {
                libc::munmap(addr, aligned_size);
                return Err(IOError::AlignmentError(format!(
                    "Huge page allocation not aligned to {}",
                    alignment
                )));
            }

            let ptr = NonNull::new_unchecked(addr as *mut u8);
            let layout = Layout::from_size_align_unchecked(aligned_size, alignment);

            Ok(Self {
                ptr,
                layout,
                size: aligned_size,
                alignment,
                strategy,
                mmap_region: Some((addr, aligned_size)),
            })
        }
    }

    #[cfg(target_os = "linux")]
    fn allocate_numa(size: usize, alignment: usize, _node: i32) -> IOResult<Self> {
        // Try NUMA-aware allocation
        // This requires hwloc or numa library
        // For now, fallback to standard allocation
        // TODO: Implement proper NUMA allocation with hwloc2
        Self::new_with_strategy(size, alignment, AllocStrategy::Standard)
    }

    /// Create buffer aligned to sector boundary (512 bytes)
    pub fn sector_aligned(size: usize) -> IOResult<Self> {
        Self::new(size, SECTOR_SIZE)
    }

    /// Create buffer aligned to page boundary (4KB)
    pub fn page_aligned(size: usize) -> IOResult<Self> {
        Self::new(size, PAGE_SIZE)
    }

    /// Create buffer aligned to huge page (2MB) - for large operations
    pub fn huge_page_aligned(size: usize) -> IOResult<Self> {
        Self::new(size, HUGE_PAGE_2MB)
    }

    /// Get mutable slice to buffer
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.ptr.as_ptr(), self.size) }
    }

    /// Get immutable slice to buffer
    pub fn as_slice(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.ptr.as_ptr(), self.size) }
    }

    /// Get buffer size
    pub fn size(&self) -> usize {
        self.size
    }

    /// Get buffer alignment
    pub fn alignment(&self) -> usize {
        self.alignment
    }

    /// Zero out the buffer
    pub fn zero(&mut self) {
        unsafe {
            std::ptr::write_bytes(self.ptr.as_ptr(), 0, self.size);
        }
    }

    /// Fill buffer with a pattern
    pub fn fill(&mut self, pattern: &[u8]) {
        let slice = self.as_mut_slice();
        for (i, byte) in slice.iter_mut().enumerate() {
            *byte = pattern[i % pattern.len()];
        }
    }
}

impl Drop for AlignedBuffer {
    fn drop(&mut self) {
        #[cfg(target_os = "linux")]
        {
            if let Some((addr, size)) = self.mmap_region {
                unsafe {
                    libc::munmap(addr, size);
                }
                return;
            }
        }

        unsafe {
            dealloc(self.ptr.as_ptr(), self.layout);
        }
    }
}

unsafe impl Send for AlignedBuffer {}
unsafe impl Sync for AlignedBuffer {}

/// Pool of pre-allocated aligned buffers for reuse
pub struct BufferPool {
    buffers: Arc<Mutex<Vec<AlignedBuffer>>>,
    buffer_size: usize,
    alignment: usize,
    max_buffers: usize,
    allocated_count: Arc<Mutex<usize>>,
}

impl BufferPool {
    /// Create a new buffer pool
    pub fn new(buffer_size: usize, alignment: usize, max_buffers: usize) -> Self {
        Self {
            buffers: Arc::new(Mutex::new(Vec::with_capacity(max_buffers))),
            buffer_size,
            alignment,
            max_buffers,
            allocated_count: Arc::new(Mutex::new(0)),
        }
    }

    /// Create a pool optimized for Direct I/O
    pub fn direct_io_pool(buffer_size: usize, max_buffers: usize) -> Self {
        Self::new(buffer_size, PAGE_SIZE, max_buffers)
    }

    /// Get a buffer from the pool or allocate a new one
    pub fn acquire(&self) -> IOResult<PooledBuffer> {
        // Try to get from pool first
        let mut buffers = self.buffers.lock().unwrap();

        if let Some(buffer) = buffers.pop() {
            drop(buffers);
            return Ok(PooledBuffer {
                buffer,
                pool: self.buffers.clone(),
            });
        }

        drop(buffers);

        // Check if we can allocate more
        let mut count = self.allocated_count.lock().unwrap();
        if *count >= self.max_buffers {
            return Err(IOError::AllocationFailed(format!(
                "Buffer pool exhausted (max: {})",
                self.max_buffers
            )));
        }

        *count += 1;
        drop(count);

        // Allocate new buffer
        let buffer = AlignedBuffer::new(self.buffer_size, self.alignment)?;

        Ok(PooledBuffer {
            buffer,
            pool: self.buffers.clone(),
        })
    }

    /// Pre-allocate buffers in the pool
    pub fn preallocate(&self, count: usize) -> IOResult<()> {
        let actual_count = count.min(self.max_buffers);
        let mut buffers = self.buffers.lock().unwrap();

        for _ in 0..actual_count {
            let buffer = AlignedBuffer::new(self.buffer_size, self.alignment)?;
            buffers.push(buffer);
        }

        *self.allocated_count.lock().unwrap() = actual_count;
        Ok(())
    }

    /// Get pool statistics
    pub fn stats(&self) -> PoolStats {
        let buffers = self.buffers.lock().unwrap();
        let allocated = *self.allocated_count.lock().unwrap();

        PoolStats {
            available: buffers.len(),
            allocated,
            max_buffers: self.max_buffers,
            buffer_size: self.buffer_size,
            total_memory: allocated * self.buffer_size,
        }
    }
}

/// Buffer that returns to pool when dropped
pub struct PooledBuffer {
    buffer: AlignedBuffer,
    pool: Arc<Mutex<Vec<AlignedBuffer>>>,
}

impl PooledBuffer {
    /// Get mutable access to the buffer
    pub fn as_mut(&mut self) -> &mut AlignedBuffer {
        &mut self.buffer
    }

    /// Get immutable access to the buffer
    pub fn as_ref(&self) -> &AlignedBuffer {
        &self.buffer
    }
}

impl Drop for PooledBuffer {
    fn drop(&mut self) {
        // Zero out buffer before returning to pool for security
        self.buffer.zero();

        // Return to pool
        let buffer = std::mem::replace(
            &mut self.buffer,
            AlignedBuffer::new(0, SECTOR_SIZE).unwrap(),
        );

        let mut pool = self.pool.lock().unwrap();
        pool.push(buffer);
    }
}

impl std::ops::Deref for PooledBuffer {
    type Target = AlignedBuffer;

    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}

impl std::ops::DerefMut for PooledBuffer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.buffer
    }
}

/// Buffer pool statistics
#[derive(Debug, Clone)]
pub struct PoolStats {
    pub available: usize,
    pub allocated: usize,
    pub max_buffers: usize,
    pub buffer_size: usize,
    pub total_memory: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aligned_buffer_creation() {
        let buffer = AlignedBuffer::sector_aligned(4096).unwrap();
        assert_eq!(buffer.size(), 4096);
        assert_eq!(buffer.alignment(), SECTOR_SIZE);

        // Check alignment
        let ptr = buffer.as_slice().as_ptr();
        assert_eq!(ptr as usize % SECTOR_SIZE, 0);
    }

    #[test]
    fn test_page_aligned_buffer() {
        let buffer = AlignedBuffer::page_aligned(8192).unwrap();
        assert_eq!(buffer.alignment(), PAGE_SIZE);

        let ptr = buffer.as_slice().as_ptr();
        assert_eq!(ptr as usize % PAGE_SIZE, 0);
    }

    #[test]
    fn test_buffer_fill() {
        let mut buffer = AlignedBuffer::sector_aligned(1024).unwrap();
        buffer.fill(&[0xAA, 0xBB]);

        let slice = buffer.as_slice();
        assert_eq!(slice[0], 0xAA);
        assert_eq!(slice[1], 0xBB);
        assert_eq!(slice[2], 0xAA);
        assert_eq!(slice[3], 0xBB);
    }

    #[test]
    fn test_buffer_pool_acquire() {
        let pool = BufferPool::direct_io_pool(4096, 10);

        let buffer1 = pool.acquire().unwrap();
        assert_eq!(buffer1.size(), 4096);

        let buffer2 = pool.acquire().unwrap();
        assert_eq!(buffer2.size(), 4096);

        drop(buffer1);
        drop(buffer2);

        // Buffers should be returned to pool
        let stats = pool.stats();
        assert_eq!(stats.available, 2);
    }

    #[test]
    fn test_buffer_pool_preallocate() {
        let pool = BufferPool::direct_io_pool(4096, 10);
        pool.preallocate(5).unwrap();

        let stats = pool.stats();
        assert_eq!(stats.available, 5);
        assert_eq!(stats.allocated, 5);
    }

    #[test]
    fn test_buffer_pool_exhaustion() {
        let pool = BufferPool::direct_io_pool(4096, 2);

        let _b1 = pool.acquire().unwrap();
        let _b2 = pool.acquire().unwrap();

        // Should fail - pool exhausted
        assert!(pool.acquire().is_err());
    }
}
