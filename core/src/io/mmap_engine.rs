// Memory-mapped I/O engine for fast verification

use super::{IOError, IOResult};
use std::fs::File;

/// Memory-mapped file for fast read operations
pub struct MmapEngine {
    #[cfg(target_os = "linux")]
    mmap: memmap2::Mmap,
    #[cfg(not(target_os = "linux"))]
    _phantom: std::marker::PhantomData<()>,
}

impl MmapEngine {
    /// Create a memory-mapped view of a file
    #[cfg(target_os = "linux")]
    pub fn new(file: &File) -> IOResult<Self> {
        use memmap2::MmapOptions;

        let mmap = unsafe {
            MmapOptions::new()
                .map(file)
                .map_err(|e| IOError::OperationFailed(format!("Failed to mmap file: {}", e)))?
        };

        Ok(Self { mmap })
    }

    #[cfg(not(target_os = "linux"))]
    pub fn new(_file: &File) -> IOResult<Self> {
        Err(IOError::PlatformNotSupported(
            "Memory mapping not implemented for this platform".to_string(),
        ))
    }

    /// Read data from the mapped region
    #[cfg(target_os = "linux")]
    pub fn read_at(&self, offset: u64, size: usize) -> IOResult<&[u8]> {
        let offset = offset as usize;
        let end = offset + size;

        if end > self.mmap.len() {
            return Err(IOError::OperationFailed(format!(
                "Read beyond mmap bounds: {} > {}",
                end,
                self.mmap.len()
            )));
        }

        Ok(&self.mmap[offset..end])
    }

    #[cfg(not(target_os = "linux"))]
    pub fn read_at(&self, _offset: u64, _size: usize) -> IOResult<&[u8]> {
        Err(IOError::PlatformNotSupported(
            "Memory mapping not implemented".to_string(),
        ))
    }

    /// Get the full mapped region
    #[cfg(target_os = "linux")]
    pub fn as_slice(&self) -> &[u8] {
        &self.mmap[..]
    }

    #[cfg(not(target_os = "linux"))]
    pub fn as_slice(&self) -> &[u8] {
        &[]
    }

    /// Get the size of the mapped region
    #[cfg(target_os = "linux")]
    pub fn len(&self) -> usize {
        self.mmap.len()
    }

    #[cfg(not(target_os = "linux"))]
    pub fn len(&self) -> usize {
        0
    }

    /// Check if the mapped region is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Advise the kernel about access patterns
    #[cfg(target_os = "linux")]
    pub fn advise_sequential(&self) -> IOResult<()> {
        self.mmap
            .advise(memmap2::Advice::Sequential)
            .map_err(|e| IOError::OperationFailed(format!("madvise failed: {}", e)))
    }

    #[cfg(not(target_os = "linux"))]
    pub fn advise_sequential(&self) -> IOResult<()> {
        Ok(())
    }

    /// Advise random access pattern
    #[cfg(target_os = "linux")]
    pub fn advise_random(&self) -> IOResult<()> {
        self.mmap
            .advise(memmap2::Advice::Random)
            .map_err(|e| IOError::OperationFailed(format!("madvise failed: {}", e)))
    }

    #[cfg(not(target_os = "linux"))]
    pub fn advise_random(&self) -> IOResult<()> {
        Ok(())
    }

    /// Tell kernel we will need this data soon
    #[cfg(target_os = "linux")]
    pub fn advise_willneed(&self) -> IOResult<()> {
        self.mmap
            .advise(memmap2::Advice::WillNeed)
            .map_err(|e| IOError::OperationFailed(format!("madvise failed: {}", e)))
    }

    #[cfg(not(target_os = "linux"))]
    pub fn advise_willneed(&self) -> IOResult<()> {
        Ok(())
    }
}

/// Memory-mapped writable file
pub struct MmapMutEngine {
    #[cfg(target_os = "linux")]
    mmap: memmap2::MmapMut,
    #[cfg(not(target_os = "linux"))]
    _phantom: std::marker::PhantomData<()>,
}

impl MmapMutEngine {
    /// Create a writable memory-mapped view
    #[cfg(target_os = "linux")]
    pub fn new(file: &File) -> IOResult<Self> {
        use memmap2::MmapOptions;

        let mmap = unsafe {
            MmapOptions::new()
                .map_mut(file)
                .map_err(|e| IOError::OperationFailed(format!("Failed to mmap file: {}", e)))?
        };

        Ok(Self { mmap })
    }

    #[cfg(not(target_os = "linux"))]
    pub fn new(_file: &File) -> IOResult<Self> {
        Err(IOError::PlatformNotSupported(
            "Memory mapping not implemented for this platform".to_string(),
        ))
    }

    /// Write data to the mapped region
    #[cfg(target_os = "linux")]
    pub fn write_at(&mut self, offset: u64, data: &[u8]) -> IOResult<usize> {
        let offset = offset as usize;
        let end = offset + data.len();

        if end > self.mmap.len() {
            return Err(IOError::OperationFailed(format!(
                "Write beyond mmap bounds: {} > {}",
                end,
                self.mmap.len()
            )));
        }

        self.mmap[offset..end].copy_from_slice(data);
        Ok(data.len())
    }

    #[cfg(not(target_os = "linux"))]
    pub fn write_at(&mut self, _offset: u64, _data: &[u8]) -> IOResult<usize> {
        Err(IOError::PlatformNotSupported(
            "Memory mapping not implemented".to_string(),
        ))
    }

    /// Flush changes to disk
    #[cfg(target_os = "linux")]
    pub fn flush(&self) -> IOResult<()> {
        self.mmap
            .flush()
            .map_err(|e| IOError::OperationFailed(format!("mmap flush failed: {}", e)))
    }

    #[cfg(not(target_os = "linux"))]
    pub fn flush(&self) -> IOResult<()> {
        Ok(())
    }

    /// Flush asynchronously
    #[cfg(target_os = "linux")]
    pub fn flush_async(&self) -> IOResult<()> {
        self.mmap
            .flush_async()
            .map_err(|e| IOError::OperationFailed(format!("mmap async flush failed: {}", e)))
    }

    #[cfg(not(target_os = "linux"))]
    pub fn flush_async(&self) -> IOResult<()> {
        Ok(())
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_mmap_read() {
        let mut temp = NamedTempFile::new().unwrap();
        let test_data = b"Hello, memory-mapped I/O!";
        temp.write_all(test_data).unwrap();
        temp.flush().unwrap();

        let file = temp.reopen().unwrap();
        let mmap = MmapEngine::new(&file).unwrap();

        let data = mmap.read_at(0, test_data.len()).unwrap();
        assert_eq!(data, test_data);
    }

    #[test]
    fn test_mmap_advise() {
        let mut temp = NamedTempFile::new().unwrap();
        temp.write_all(&vec![0u8; 4096]).unwrap();
        temp.flush().unwrap();

        let file = temp.reopen().unwrap();
        let mmap = MmapEngine::new(&file).unwrap();

        assert!(mmap.advise_sequential().is_ok());
        assert!(mmap.advise_random().is_ok());
        assert!(mmap.advise_willneed().is_ok());
    }

    #[test]
    fn test_mmap_write() {
        let temp = NamedTempFile::new().unwrap();
        let file = temp.reopen().unwrap();

        // Resize file first
        file.set_len(4096).unwrap();

        let mut mmap = MmapMutEngine::new(&file).unwrap();
        let test_data = b"Test write data";

        let written = mmap.write_at(0, test_data).unwrap();
        assert_eq!(written, test_data.len());

        mmap.flush().unwrap();
    }
}
