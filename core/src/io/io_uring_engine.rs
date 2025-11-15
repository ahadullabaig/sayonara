// io_uring async I/O engine for Linux (kernel 5.1+)

#[cfg(target_os = "linux")]
use super::{IOError, IOResult};
#[cfg(target_os = "linux")]
use io_uring::{opcode, types, IoUring};
#[cfg(target_os = "linux")]
use std::fs::File;
#[cfg(target_os = "linux")]
use std::os::unix::io::AsRawFd;

#[cfg(target_os = "linux")]
/// io_uring-based async I/O engine
pub struct IoUringEngine {
    ring: IoUring,
    queue_depth: usize,
}

#[cfg(target_os = "linux")]
impl IoUringEngine {
    /// Create a new io_uring engine
    pub fn new(queue_depth: u32) -> IOResult<Self> {
        let ring = IoUring::new(queue_depth)
            .map_err(|e| IOError::OperationFailed(format!("Failed to create io_uring: {}", e)))?;

        Ok(Self {
            ring,
            queue_depth: queue_depth as usize,
        })
    }

    /// Check if io_uring is available on this system
    pub fn is_available() -> bool {
        // Try to create a minimal io_uring instance
        IoUring::new(2).is_ok()
    }

    /// Submit a write operation
    pub fn write_at(&mut self, file: &File, data: &[u8], offset: u64) -> IOResult<usize> {
        let fd = types::Fd(file.as_raw_fd());

        // Create write operation
        let write_op = opcode::Write::new(fd, data.as_ptr(), data.len() as u32).offset(offset);

        // Submit to submission queue
        unsafe {
            self.ring
                .submission()
                .push(&write_op.build().user_data(0))
                .map_err(|e| IOError::OperationFailed(format!("io_uring push failed: {}", e)))?;
        }

        // Submit and wait for completion
        self.ring
            .submit_and_wait(1)
            .map_err(|e| IOError::OperationFailed(format!("io_uring submit failed: {}", e)))?;

        // Get completion result
        let cqe = self
            .ring
            .completion()
            .next()
            .ok_or_else(|| IOError::OperationFailed("No completion event".to_string()))?;

        let result = cqe.result();
        if result < 0 {
            return Err(IOError::OperationFailed(format!(
                "io_uring write failed: {}",
                std::io::Error::from_raw_os_error(-result)
            )));
        }

        Ok(result as usize)
    }

    /// Submit a read operation
    pub fn read_at(&mut self, file: &File, buffer: &mut [u8], offset: u64) -> IOResult<usize> {
        let fd = types::Fd(file.as_raw_fd());

        // Create read operation
        let read_op =
            opcode::Read::new(fd, buffer.as_mut_ptr(), buffer.len() as u32).offset(offset);

        // Submit to submission queue
        unsafe {
            self.ring
                .submission()
                .push(&read_op.build().user_data(0))
                .map_err(|e| IOError::OperationFailed(format!("io_uring push failed: {}", e)))?;
        }

        // Submit and wait for completion
        self.ring
            .submit_and_wait(1)
            .map_err(|e| IOError::OperationFailed(format!("io_uring submit failed: {}", e)))?;

        // Get completion result
        let cqe = self
            .ring
            .completion()
            .next()
            .ok_or_else(|| IOError::OperationFailed("No completion event".to_string()))?;

        let result = cqe.result();
        if result < 0 {
            return Err(IOError::OperationFailed(format!(
                "io_uring read failed: {}",
                std::io::Error::from_raw_os_error(-result)
            )));
        }

        Ok(result as usize)
    }

    /// Submit multiple write operations in parallel
    pub fn batch_write(
        &mut self,
        file: &File,
        operations: &[(u64, &[u8])], // (offset, data)
    ) -> IOResult<Vec<usize>> {
        let fd = types::Fd(file.as_raw_fd());
        let mut results = Vec::with_capacity(operations.len());

        // Submit all operations
        for (i, (offset, data)) in operations.iter().enumerate() {
            let write_op = opcode::Write::new(fd, data.as_ptr(), data.len() as u32).offset(*offset);

            unsafe {
                self.ring
                    .submission()
                    .push(&write_op.build().user_data(i as u64))
                    .map_err(|e| {
                        IOError::OperationFailed(format!("io_uring batch push failed: {}", e))
                    })?;
            }
        }

        // Submit all at once
        self.ring.submit().map_err(|e| {
            IOError::OperationFailed(format!("io_uring batch submit failed: {}", e))
        })?;

        // Wait for all completions
        for _ in 0..operations.len() {
            self.ring
                .submit_and_wait(1)
                .map_err(|e| IOError::OperationFailed(format!("io_uring wait failed: {}", e)))?;

            if let Some(cqe) = self.ring.completion().next() {
                let result = cqe.result();
                if result < 0 {
                    return Err(IOError::OperationFailed(format!(
                        "io_uring batch write failed: {}",
                        std::io::Error::from_raw_os_error(-result)
                    )));
                }
                results.push(result as usize);
            }
        }

        Ok(results)
    }

    /// Submit multiple read operations in parallel
    pub fn batch_read(
        &mut self,
        file: &File,
        operations: &mut [(u64, &mut [u8])], // (offset, buffer)
    ) -> IOResult<Vec<usize>> {
        let fd = types::Fd(file.as_raw_fd());
        let mut results = Vec::with_capacity(operations.len());

        // Submit all operations
        for (i, (offset, buffer)) in operations.iter_mut().enumerate() {
            let read_op =
                opcode::Read::new(fd, buffer.as_mut_ptr(), buffer.len() as u32).offset(*offset);

            unsafe {
                self.ring
                    .submission()
                    .push(&read_op.build().user_data(i as u64))
                    .map_err(|e| {
                        IOError::OperationFailed(format!("io_uring batch push failed: {}", e))
                    })?;
            }
        }

        // Submit all at once
        self.ring.submit().map_err(|e| {
            IOError::OperationFailed(format!("io_uring batch submit failed: {}", e))
        })?;

        // Wait for all completions
        for _ in 0..operations.len() {
            self.ring
                .submit_and_wait(1)
                .map_err(|e| IOError::OperationFailed(format!("io_uring wait failed: {}", e)))?;

            if let Some(cqe) = self.ring.completion().next() {
                let result = cqe.result();
                if result < 0 {
                    return Err(IOError::OperationFailed(format!(
                        "io_uring batch read failed: {}",
                        std::io::Error::from_raw_os_error(-result)
                    )));
                }
                results.push(result as usize);
            }
        }

        Ok(results)
    }

    /// Sync file data to disk
    pub fn fsync(&mut self, file: &File) -> IOResult<()> {
        let fd = types::Fd(file.as_raw_fd());

        let fsync_op = opcode::Fsync::new(fd);

        unsafe {
            self.ring
                .submission()
                .push(&fsync_op.build().user_data(0))
                .map_err(|e| {
                    IOError::OperationFailed(format!("io_uring fsync push failed: {}", e))
                })?;
        }

        self.ring.submit_and_wait(1).map_err(|e| {
            IOError::OperationFailed(format!("io_uring fsync submit failed: {}", e))
        })?;

        let cqe = self
            .ring
            .completion()
            .next()
            .ok_or_else(|| IOError::OperationFailed("No fsync completion".to_string()))?;

        let result = cqe.result();
        if result < 0 {
            return Err(IOError::OperationFailed(format!(
                "io_uring fsync failed: {}",
                std::io::Error::from_raw_os_error(-result)
            )));
        }

        Ok(())
    }

    /// Get queue depth
    pub fn queue_depth(&self) -> usize {
        self.queue_depth
    }
}

#[cfg(not(target_os = "linux"))]
/// Stub for non-Linux platforms
pub struct IoUringEngine;

#[cfg(not(target_os = "linux"))]
impl IoUringEngine {
    pub fn is_available() -> bool {
        false
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_io_uring_availability() {
        // Just check if we can detect io_uring
        let available = IoUringEngine::is_available();
        println!("io_uring available: {}", available);
    }

    #[test]
    fn test_io_uring_write_read() {
        if !IoUringEngine::is_available() {
            println!("io_uring not available, skipping test");
            return;
        }

        let mut temp = NamedTempFile::new().unwrap();
        temp.write_all(&vec![0u8; 4096]).unwrap();
        temp.flush().unwrap();

        let file = temp.reopen().unwrap();
        let mut engine = IoUringEngine::new(4).unwrap();

        // Write test
        let data = b"Hello io_uring!";
        let written = engine.write_at(&file, data, 0).unwrap();
        assert_eq!(written, data.len());

        // Read test
        let mut buffer = vec![0u8; 32];
        let read = engine.read_at(&file, &mut buffer, 0).unwrap();
        assert_eq!(read, 32);
        assert_eq!(&buffer[..data.len()], data);
    }
}
