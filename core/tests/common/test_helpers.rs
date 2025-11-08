/// Common test helper functions

use std::fs;
use std::io::Read;

/// Verify that a file contains only zeros
pub fn verify_all_zeros(path: &std::path::Path) -> std::io::Result<bool> {
    let mut file = fs::File::open(path)?;
    let mut buffer = vec![0u8; 4096];

    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }

        for &byte in &buffer[..bytes_read] {
            if byte != 0 {
                return Ok(false);
            }
        }
    }

    Ok(true)
}

/// Verify that a file contains a specific pattern
pub fn verify_pattern(path: &std::path::Path, pattern: &[u8]) -> std::io::Result<bool> {
    let mut file = fs::File::open(path)?;
    let mut buffer = vec![0u8; 4096];

    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }

        for (i, &byte) in buffer[..bytes_read].iter().enumerate() {
            let expected = pattern[i % pattern.len()];
            if byte != expected {
                return Ok(false);
            }
        }
    }

    Ok(true)
}

/// Calculate Shannon entropy of a file
pub fn calculate_file_entropy(path: &std::path::Path) -> std::io::Result<f64> {
    let mut file = fs::File::open(path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;

    let mut counts = [0u64; 256];
    for &byte in &buffer {
        counts[byte as usize] += 1;
    }

    let length = buffer.len() as f64;
    let mut entropy = 0.0;

    for &count in &counts {
        if count > 0 {
            let probability = count as f64 / length;
            entropy -= probability * probability.log2();
        }
    }

    Ok(entropy)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;

    #[test]
    fn test_verify_all_zeros() {
        let mut temp = NamedTempFile::new().unwrap();
        temp.write_all(&vec![0u8; 1024]).unwrap();
        temp.flush().unwrap();

        assert!(verify_all_zeros(temp.path()).unwrap());
    }

    #[test]
    fn test_verify_pattern() {
        let mut temp = NamedTempFile::new().unwrap();
        let pattern = [0xAA, 0xBB];
        let data: Vec<u8> = (0..1024).map(|i| pattern[i % 2]).collect();
        temp.write_all(&data).unwrap();
        temp.flush().unwrap();

        assert!(verify_pattern(temp.path(), &pattern).unwrap());
    }

    #[test]
    fn test_calculate_file_entropy() {
        let mut temp = NamedTempFile::new().unwrap();
        // All zeros should have near-zero entropy
        temp.write_all(&vec![0u8; 1000]).unwrap();
        temp.flush().unwrap();

        let entropy = calculate_file_entropy(temp.path()).unwrap();
        assert!(entropy < 0.1);
    }
}
