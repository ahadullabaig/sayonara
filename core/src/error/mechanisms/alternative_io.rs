/// Alternative I/O methods - fallback strategies when primary I/O fails
///
/// This module provides multiple I/O fallback methods, trying them in order
/// from fastest to slowest/safest when errors occur.
use anyhow::{Context, Result};
use std::fs::OpenOptions;
use std::io::{Seek, SeekFrom, Write};
use std::os::unix::fs::OpenOptionsExt;

/// I/O method types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IOMethod {
    /// OptimizedIO with O_DIRECT (fastest, requires alignment)
    OptimizedDirect,

    /// Standard buffered I/O (kernel cache)
    Buffered,

    /// Memory-mapped I/O
    MemoryMapped,

    /// Synchronous I/O with O_SYNC (slowest, safest)
    Synchronous,
}

impl IOMethod {
    /// Get human-readable description
    pub fn description(&self) -> &'static str {
        match self {
            IOMethod::OptimizedDirect => "Optimized direct I/O (O_DIRECT)",
            IOMethod::Buffered => "Standard buffered I/O",
            IOMethod::MemoryMapped => "Memory-mapped I/O",
            IOMethod::Synchronous => "Synchronous I/O (O_SYNC)",
        }
    }

    /// Estimated relative performance (1-10, where 10 is fastest)
    pub fn performance_score(&self) -> u8 {
        match self {
            IOMethod::OptimizedDirect => 10,
            IOMethod::MemoryMapped => 8,
            IOMethod::Buffered => 6,
            IOMethod::Synchronous => 3,
        }
    }

    /// Safety/reliability score (1-10, where 10 is safest)
    pub fn safety_score(&self) -> u8 {
        match self {
            IOMethod::Synchronous => 10,
            IOMethod::Buffered => 8,
            IOMethod::MemoryMapped => 6,
            IOMethod::OptimizedDirect => 5,
        }
    }
}

/// Alternative I/O manager
pub struct AlternativeIO {
    /// Fallback order (try methods in this sequence)
    fallback_order: Vec<IOMethod>,

    /// Currently active method
    current_method: Option<IOMethod>,
}

impl AlternativeIO {
    /// Create new alternative I/O manager with default fallback order
    pub fn new() -> Self {
        Self {
            fallback_order: vec![
                IOMethod::OptimizedDirect,
                IOMethod::Buffered,
                IOMethod::MemoryMapped,
                IOMethod::Synchronous,
            ],
            current_method: None,
        }
    }

    /// Create with custom fallback order
    pub fn with_fallback_order(fallback_order: Vec<IOMethod>) -> Self {
        Self {
            fallback_order,
            current_method: None,
        }
    }

    /// Write with automatic fallback
    pub fn write_with_fallback(
        &mut self,
        device: &str,
        offset: u64,
        data: &[u8],
    ) -> Result<IOMethod> {
        let mut last_error = None;

        for method in &self.fallback_order {
            tracing::debug!(
                device = %device,
                offset = offset,
                size = data.len(),
                method = ?method,
                "Attempting write with method"
            );

            match self.write_with_method(device, offset, data, *method) {
                Ok(_) => {
                    self.current_method = Some(*method);
                    tracing::info!(
                        device = %device,
                        method = ?method,
                        "Write succeeded with method"
                    );
                    return Ok(*method);
                }
                Err(e) => {
                    tracing::warn!(
                        device = %device,
                        method = ?method,
                        error = %e,
                        "Write failed with method, trying next"
                    );
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("No I/O methods available")))
    }

    /// Write using specific method
    fn write_with_method(
        &self,
        device: &str,
        offset: u64,
        data: &[u8],
        method: IOMethod,
    ) -> Result<()> {
        match method {
            IOMethod::OptimizedDirect => self.write_optimized_direct(device, offset, data),
            IOMethod::Buffered => self.write_buffered(device, offset, data),
            IOMethod::MemoryMapped => self.write_memory_mapped(device, offset, data),
            IOMethod::Synchronous => self.write_synchronous(device, offset, data),
        }
    }

    /// Write with O_DIRECT (requires alignment)
    fn write_optimized_direct(&self, device: &str, offset: u64, data: &[u8]) -> Result<()> {
        // Check alignment (512-byte boundary for most devices)
        if offset % 512 != 0 || data.len() % 512 != 0 {
            return Err(anyhow::anyhow!("Data not aligned for O_DIRECT"));
        }

        let mut file = OpenOptions::new()
            .write(true)
            .custom_flags(libc::O_DIRECT)
            .open(device)
            .context("Failed to open device with O_DIRECT")?;

        file.seek(SeekFrom::Start(offset))
            .context("Failed to seek")?;

        file.write_all(data)
            .context("Failed to write with O_DIRECT")?;

        file.sync_all().context("Failed to sync")?;

        Ok(())
    }

    /// Standard buffered write
    fn write_buffered(&self, device: &str, offset: u64, data: &[u8]) -> Result<()> {
        let mut file = OpenOptions::new()
            .write(true)
            .open(device)
            .context("Failed to open device for buffered write")?;

        file.seek(SeekFrom::Start(offset))
            .context("Failed to seek")?;

        file.write_all(data).context("Failed to write")?;

        file.sync_all().context("Failed to sync")?;

        Ok(())
    }

    /// Memory-mapped write
    fn write_memory_mapped(&self, device: &str, offset: u64, data: &[u8]) -> Result<()> {
        #[cfg(target_os = "linux")]
        {
            use memmap2::MmapMut;
            use std::os::unix::io::AsRawFd;

            let file = OpenOptions::new()
                .read(true)
                .write(true)
                .open(device)
                .context("Failed to open device for mmap")?;

            let _fd = file.as_raw_fd();

            // Map the region
            let mut mmap =
                unsafe { MmapMut::map_mut(&file).context("Failed to create memory map")? };

            // Write data
            let start = offset as usize;
            let end = start + data.len();

            if end > mmap.len() {
                return Err(anyhow::anyhow!("Write exceeds device size"));
            }

            mmap[start..end].copy_from_slice(data);

            // Sync to disk
            mmap.flush().context("Failed to flush memory map")?;

            Ok(())
        }

        #[cfg(not(target_os = "linux"))]
        {
            Err(anyhow::anyhow!(
                "Memory-mapped I/O not supported on this platform"
            ))
        }
    }

    /// Synchronous write with O_SYNC
    fn write_synchronous(&self, device: &str, offset: u64, data: &[u8]) -> Result<()> {
        let mut file = OpenOptions::new()
            .write(true)
            .custom_flags(libc::O_SYNC)
            .open(device)
            .context("Failed to open device with O_SYNC")?;

        file.seek(SeekFrom::Start(offset))
            .context("Failed to seek")?;

        file.write_all(data)
            .context("Failed to write with O_SYNC")?;

        // O_SYNC ensures data is on disk before write returns
        Ok(())
    }

    /// Get current active method
    pub fn current_method(&self) -> Option<IOMethod> {
        self.current_method
    }

    /// Reset to default method
    pub fn reset(&mut self) {
        self.current_method = None;
    }

    /// Get fallback order
    pub fn fallback_order(&self) -> &[IOMethod] {
        &self.fallback_order
    }
}

impl Default for AlternativeIO {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_io_method_properties() {
        assert_eq!(
            IOMethod::OptimizedDirect.description(),
            "Optimized direct I/O (O_DIRECT)"
        );
        assert!(
            IOMethod::OptimizedDirect.performance_score()
                > IOMethod::Synchronous.performance_score()
        );
        assert!(IOMethod::Synchronous.safety_score() > IOMethod::OptimizedDirect.safety_score());
    }

    #[test]
    fn test_alternative_io_creation() {
        let alt_io = AlternativeIO::new();
        assert_eq!(alt_io.fallback_order().len(), 4);
        assert_eq!(alt_io.fallback_order()[0], IOMethod::OptimizedDirect);
        assert!(alt_io.current_method().is_none());
    }

    #[test]
    fn test_custom_fallback_order() {
        let order = vec![IOMethod::Buffered, IOMethod::Synchronous];
        let alt_io = AlternativeIO::with_fallback_order(order);
        assert_eq!(alt_io.fallback_order().len(), 2);
        assert_eq!(alt_io.fallback_order()[0], IOMethod::Buffered);
    }

    #[test]
    fn test_write_buffered() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();

        let alt_io = AlternativeIO::new();
        let data = vec![0x42u8; 512];

        let result = alt_io.write_buffered(path.to_str().unwrap(), 0, &data);
        assert!(result.is_ok());

        // Verify data was written
        let read_data = std::fs::read(path).unwrap();
        assert_eq!(&read_data[..512], &data[..]);
    }

    #[test]
    fn test_write_synchronous() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();

        let alt_io = AlternativeIO::new();
        let data = vec![0x55u8; 1024];

        let result = alt_io.write_synchronous(path.to_str().unwrap(), 0, &data);
        assert!(result.is_ok());

        // Verify data was written
        let read_data = std::fs::read(path).unwrap();
        assert_eq!(&read_data[..1024], &data[..]);
    }

    #[test]
    fn test_write_with_fallback() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();

        let mut alt_io = AlternativeIO::new();
        let data = vec![0xAAu8; 1024];

        // Will fail with O_DIRECT (not aligned for regular file)
        // Should succeed with buffered fallback
        let result = alt_io.write_with_fallback(path.to_str().unwrap(), 0, &data);

        assert!(result.is_ok());
        let method = result.unwrap();

        // Should have succeeded with one of the fallback methods
        // (any method is valid as long as it succeeded)
        assert!(matches!(
            method,
            IOMethod::OptimizedDirect
                | IOMethod::Buffered
                | IOMethod::MemoryMapped
                | IOMethod::Synchronous
        ));
    }

    #[test]
    fn test_current_method_tracking() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();

        let mut alt_io = AlternativeIO::new();
        assert!(alt_io.current_method().is_none());

        let data = vec![0xBBu8; 512];
        alt_io
            .write_with_fallback(path.to_str().unwrap(), 0, &data)
            .unwrap();

        assert!(alt_io.current_method().is_some());
    }

    #[test]
    fn test_reset() {
        let mut alt_io = AlternativeIO::new();
        alt_io.current_method = Some(IOMethod::Buffered);

        alt_io.reset();
        assert!(alt_io.current_method().is_none());
    }

    #[test]
    fn test_optimized_direct_alignment_check() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();

        let alt_io = AlternativeIO::new();

        // Unaligned data should fail
        let data = vec![0x11u8; 513]; // Not 512-byte aligned
        let result = alt_io.write_optimized_direct(path.to_str().unwrap(), 0, &data);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not aligned"));
    }

    #[test]
    fn test_method_performance_ordering() {
        let methods = vec![
            IOMethod::OptimizedDirect,
            IOMethod::MemoryMapped,
            IOMethod::Buffered,
            IOMethod::Synchronous,
        ];

        // Verify performance decreases
        for i in 0..methods.len() - 1 {
            assert!(methods[i].performance_score() >= methods[i + 1].performance_score());
        }
    }

    #[test]
    fn test_method_safety_ordering() {
        assert!(IOMethod::Synchronous.safety_score() > IOMethod::Buffered.safety_score());
        assert!(IOMethod::Buffered.safety_score() > IOMethod::OptimizedDirect.safety_score());
    }
}
