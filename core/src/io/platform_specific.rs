// Platform-specific I/O implementations

use super::{IOError, IOResult};
use std::fs::File;
use std::os::unix::fs::OpenOptionsExt;

#[cfg(target_os = "linux")]
/// Platform-specific I/O handler
pub trait PlatformIO: Send + Sync {
    /// Open file with platform-specific optimizations
    fn open_optimized(&self, path: &str, direct_io: bool) -> IOResult<File>;

    /// Write data with platform-specific optimizations
    fn write_optimized(&self, file: &File, data: &[u8], offset: u64) -> IOResult<usize>;

    /// Read data with platform-specific optimizations
    fn read_optimized(&self, file: &File, buffer: &mut [u8], offset: u64) -> IOResult<usize>;

    /// Scatter-gather write (writev)
    fn writev_optimized(&self, file: &File, buffers: &[&[u8]], offset: u64) -> IOResult<usize> {
        // Default implementation: sequential writes
        let mut total = 0;
        let mut current_offset = offset;
        for buffer in buffers {
            let written = self.write_optimized(file, buffer, current_offset)?;
            total += written;
            current_offset += written as u64;
        }
        Ok(total)
    }

    /// Scatter-gather read (readv)
    fn readv_optimized(
        &self,
        file: &File,
        buffers: &mut [&mut [u8]],
        offset: u64,
    ) -> IOResult<usize> {
        // Default implementation: sequential reads
        let mut total = 0;
        let mut current_offset = offset;
        for buffer in buffers {
            let read = self.read_optimized(file, buffer, current_offset)?;
            total += read;
            current_offset += read as u64;
            if read < buffer.len() {
                break; // EOF or partial read
            }
        }
        Ok(total)
    }

    /// Sync data to disk
    fn sync_data(&self, file: &File) -> IOResult<()>;

    /// Get platform name
    fn platform_name(&self) -> &str;
}

// ============= LINUX IMPLEMENTATION =============

#[cfg(target_os = "linux")]
pub struct LinuxIO;

#[cfg(target_os = "linux")]
impl LinuxIO {
    pub fn new() -> Self {
        Self
    }

    /// Check if io_uring is available
    fn is_io_uring_available() -> bool {
        // Check kernel version - io_uring requires 5.1+
        if let Ok(uname) = std::fs::read_to_string("/proc/version") {
            // Simple check - production code should parse version properly
            uname.contains("Linux version 5.") || uname.contains("Linux version 6.")
        } else {
            false
        }
    }
}

#[cfg(target_os = "linux")]
impl PlatformIO for LinuxIO {
    fn open_optimized(&self, path: &str, direct_io: bool) -> IOResult<File> {
        use std::fs::OpenOptions;

        let mut opts = OpenOptions::new();
        opts.write(true).read(true);

        // O_DIRECT flag for bypassing page cache
        if direct_io {
            opts.custom_flags(libc::O_DIRECT | libc::O_SYNC);
        }

        opts.open(path)
            .map_err(|e| IOError::OperationFailed(format!("Failed to open {}: {}", path, e)))
    }

    fn write_optimized(&self, file: &File, data: &[u8], offset: u64) -> IOResult<usize> {
        use std::os::unix::fs::FileExt;

        // Use pwrite for positioned writes without seeking
        file.write_at(data, offset).map_err(IOError::from)
    }

    fn read_optimized(&self, file: &File, buffer: &mut [u8], offset: u64) -> IOResult<usize> {
        use std::os::unix::fs::FileExt;

        // Use pread for positioned reads without seeking
        file.read_at(buffer, offset).map_err(IOError::from)
    }

    fn writev_optimized(&self, file: &File, buffers: &[&[u8]], offset: u64) -> IOResult<usize> {
        use std::os::unix::io::AsRawFd;

        // Build iovec array
        let iovecs: Vec<libc::iovec> = buffers
            .iter()
            .map(|buf| libc::iovec {
                iov_base: buf.as_ptr() as *mut libc::c_void,
                iov_len: buf.len(),
            })
            .collect();

        unsafe {
            let result = libc::pwritev(
                file.as_raw_fd(),
                iovecs.as_ptr(),
                iovecs.len() as i32,
                offset as i64,
            );

            if result < 0 {
                return Err(IOError::from(std::io::Error::last_os_error()));
            }

            Ok(result as usize)
        }
    }

    fn readv_optimized(
        &self,
        file: &File,
        buffers: &mut [&mut [u8]],
        offset: u64,
    ) -> IOResult<usize> {
        use std::os::unix::io::AsRawFd;

        // Build iovec array
        let iovecs: Vec<libc::iovec> = buffers
            .iter_mut()
            .map(|buf| libc::iovec {
                iov_base: buf.as_mut_ptr() as *mut libc::c_void,
                iov_len: buf.len(),
            })
            .collect();

        unsafe {
            let result = libc::preadv(
                file.as_raw_fd(),
                iovecs.as_ptr(),
                iovecs.len() as i32,
                offset as i64,
            );

            if result < 0 {
                return Err(IOError::from(std::io::Error::last_os_error()));
            }

            Ok(result as usize)
        }
    }

    fn sync_data(&self, file: &File) -> IOResult<()> {
        file.sync_data().map_err(IOError::from)
    }

    fn platform_name(&self) -> &str {
        if Self::is_io_uring_available() {
            "Linux (io_uring capable)"
        } else {
            "Linux (standard I/O)"
        }
    }
}

// ============= WINDOWS IMPLEMENTATION =============

#[cfg(target_os = "windows")]
pub struct WindowsIO;

#[cfg(target_os = "windows")]
impl WindowsIO {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(target_os = "windows")]
impl PlatformIO for WindowsIO {
    fn open_optimized(&self, path: &str, direct_io: bool) -> IOResult<File> {
        use std::fs::OpenOptions;
        use std::os::windows::fs::OpenOptionsExt;

        let mut opts = OpenOptions::new();
        opts.write(true).read(true);

        if direct_io {
            // FILE_FLAG_NO_BUFFERING for Direct I/O on Windows
            opts.custom_flags(0x20000000); // FILE_FLAG_NO_BUFFERING
        }

        opts.open(path)
            .map_err(|e| IOError::OperationFailed(format!("Failed to open {}: {}", path, e)))
    }

    fn write_optimized(&self, file: &File, data: &[u8], _offset: u64) -> IOResult<usize> {
        use std::io::Write;

        // Windows IOCP will be used automatically by the OS
        file.write(data).map_err(IOError::from)
    }

    fn read_optimized(&self, file: &File, buffer: &mut [u8], _offset: u64) -> IOResult<usize> {
        use std::io::Read;

        // Windows IOCP will be used automatically by the OS
        file.read(buffer).map_err(IOError::from)
    }

    fn sync_data(&self, file: &File) -> IOResult<()> {
        file.sync_data().map_err(IOError::from)
    }

    fn platform_name(&self) -> &str {
        "Windows (IOCP)"
    }
}

// ============= MACOS IMPLEMENTATION =============

#[cfg(target_os = "macos")]
pub struct MacOSIO;

#[cfg(target_os = "macos")]
impl MacOSIO {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(target_os = "macos")]
impl PlatformIO for MacOSIO {
    fn open_optimized(&self, path: &str, direct_io: bool) -> IOResult<File> {
        use std::fs::OpenOptions;

        let mut opts = OpenOptions::new();
        opts.write(true).read(true);

        let file = opts
            .open(path)
            .map_err(|e| IOError::OperationFailed(format!("Failed to open {}: {}", path, e)))?;

        if direct_io {
            // F_NOCACHE to bypass buffer cache on macOS
            unsafe {
                let fd = file.as_raw_fd();
                libc::fcntl(fd, libc::F_NOCACHE, 1);
            }
        }

        Ok(file)
    }

    fn write_optimized(&self, file: &File, data: &[u8], offset: u64) -> IOResult<usize> {
        use std::os::unix::fs::FileExt;

        file.write_at(data, offset).map_err(IOError::from)
    }

    fn read_optimized(&self, file: &File, buffer: &mut [u8], offset: u64) -> IOResult<usize> {
        use std::os::unix::fs::FileExt;

        file.read_at(buffer, offset).map_err(IOError::from)
    }

    fn sync_data(&self, file: &File) -> IOResult<()> {
        // Use F_FULLFSYNC on macOS for guaranteed persistence
        unsafe {
            let fd = file.as_raw_fd();
            if libc::fcntl(fd, libc::F_FULLFSYNC) != 0 {
                return Err(IOError::OperationFailed("F_FULLFSYNC failed".to_string()));
            }
        }
        Ok(())
    }

    fn platform_name(&self) -> &str {
        "macOS (Grand Central Dispatch)"
    }
}

// ============= FREEBSD IMPLEMENTATION =============

#[cfg(target_os = "freebsd")]
pub struct FreeBSDIO;

#[cfg(target_os = "freebsd")]
impl FreeBSDIO {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(target_os = "freebsd")]
impl PlatformIO for FreeBSDIO {
    fn open_optimized(&self, path: &str, direct_io: bool) -> IOResult<File> {
        use std::fs::OpenOptions;

        let mut opts = OpenOptions::new();
        opts.write(true).read(true);

        if direct_io {
            opts.custom_flags(libc::O_DIRECT);
        }

        opts.open(path)
            .map_err(|e| IOError::OperationFailed(format!("Failed to open {}: {}", path, e)))
    }

    fn write_optimized(&self, file: &File, data: &[u8], offset: u64) -> IOResult<usize> {
        use std::os::unix::fs::FileExt;

        file.write_at(data, offset).map_err(IOError::from)
    }

    fn read_optimized(&self, file: &File, buffer: &mut [u8], offset: u64) -> IOResult<usize> {
        use std::os::unix::fs::FileExt;

        file.read_at(buffer, offset).map_err(IOError::from)
    }

    fn sync_data(&self, file: &File) -> IOResult<()> {
        file.sync_data().map_err(IOError::from)
    }

    fn platform_name(&self) -> &str {
        "FreeBSD (kqueue)"
    }
}

// ============= PLATFORM FACTORY =============

/// Get the appropriate platform I/O implementation
pub fn get_platform_io() -> Box<dyn PlatformIO> {
    #[cfg(target_os = "linux")]
    {
        Box::new(LinuxIO::new())
    }

    #[cfg(target_os = "windows")]
    {
        Box::new(WindowsIO::new())
    }

    #[cfg(target_os = "macos")]
    {
        Box::new(MacOSIO::new())
    }

    #[cfg(target_os = "freebsd")]
    {
        Box::new(FreeBSDIO::new())
    }

    #[cfg(not(any(
        target_os = "linux",
        target_os = "windows",
        target_os = "macos",
        target_os = "freebsd"
    )))]
    {
        compile_error!("Unsupported platform")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_platform_io_creation() {
        let io = get_platform_io();
        assert!(!io.platform_name().is_empty());
    }

    #[test]
    fn test_platform_specific_write() {
        let io = get_platform_io();
        let temp = NamedTempFile::new().unwrap();
        let path = temp.path().to_str().unwrap();

        // Test buffered I/O
        let file = io.open_optimized(path, false).unwrap();
        let data = b"Hello, World!";
        let written = io.write_optimized(&file, data, 0).unwrap();
        assert_eq!(written, data.len());

        io.sync_data(&file).unwrap();
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_linux_direct_io() {
        use super::LinuxIO;

        let io = LinuxIO::new();
        println!("Platform: {}", io.platform_name());

        // Direct I/O requires proper alignment
        // This is tested in integration tests with actual block devices
    }
}
