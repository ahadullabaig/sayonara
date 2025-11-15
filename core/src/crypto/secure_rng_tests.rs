#[cfg(test)]
mod tests {
    use crate::crypto::secure_rng::{
        get_secure_rng, secure_random_bytes, verify_randomness, ContinuousTest, EntropyPool,
        EntropySource, HardwareRNG, HmacDrbg, JitterEntropy, RingSystemRNG, SecureRNG,
        ThreadSafeRNG, URandom,
    };
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
    use std::sync::{Arc, Mutex};
    type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

    /// Test that RNG can be initialized
    #[test]
    fn test_rng_initialization() {
        let rng = SecureRNG::new();
        assert!(rng.is_ok(), "RNG should initialize successfully");

        let rng = rng.unwrap();
        assert!(rng.is_healthy(), "RNG should be healthy after init");
    }

    /// Test entropy calculation
    #[test]
    fn test_entropy_calculation() {
        // All zeros - minimum entropy
        let zeros = vec![0u8; 1000];
        let entropy = SecureRNG::calculate_entropy(&zeros);
        assert!(entropy < 0.1, "All zeros should have near-zero entropy");

        // All ones - minimum entropy
        let ones = vec![0xFF; 1000];
        let entropy = SecureRNG::calculate_entropy(&ones);
        assert!(entropy < 0.1, "All ones should have near-zero entropy");

        // Perfect distribution - maximum entropy
        let mut perfect = Vec::new();
        for _ in 0..4 {
            for i in 0..256 {
                perfect.push(i as u8);
            }
        }
        let entropy = SecureRNG::calculate_entropy(&perfect);
        assert!(
            entropy > 7.99,
            "Perfect distribution should have ~8 bits/byte entropy"
        );

        // Half zeros, half ones - 1 bit entropy
        let mut half = vec![0u8; 500];
        half.extend(vec![0xFF; 500]);
        let entropy = SecureRNG::calculate_entropy(&half);
        assert!(
            entropy > 0.9 && entropy < 1.1,
            "Half/half should have ~1 bit entropy"
        );
    }

    /// Test FIPS 140-2 continuous test
    #[test]
    fn test_continuous_test() {
        let mut test = ContinuousTest::new();

        // First block should always pass (no comparison)
        let block1 = vec![1u8; 16];
        assert!(test.test(&block1), "First block should pass");

        // Different block should pass
        let block2 = vec![2u8; 16];
        assert!(test.test(&block2), "Different block should pass");

        // Identical block should fail
        let block3 = vec![2u8; 16];
        assert!(!test.test(&block3), "Identical block should fail");

        // Different block should pass again
        let block4 = vec![3u8; 16];
        assert!(
            test.test(&block4),
            "Different block should pass after failure"
        );
    }

    /// Test HMAC-DRBG implementation
    #[test]
    fn test_hmac_drbg() -> Result<()> {
        let seed = b"test seed material with sufficient entropy";
        let mut drbg = HmacDrbg::new(seed);

        // Generate some outputs
        let mut out1 = vec![0u8; 32];
        let mut out2 = vec![0u8; 32];
        let mut out3 = vec![0u8; 64];

        drbg.generate(&mut out1)?;
        drbg.generate(&mut out2)?;
        drbg.generate(&mut out3)?;

        // Outputs should be different
        assert_ne!(out1, out2, "DRBG should produce different outputs");
        assert_ne!(out1, &out3[..32], "DRBG outputs should be unique");

        // Test entropy of output
        let mut large_output = vec![0u8; 10000];
        drbg.generate(&mut large_output)?;
        let entropy = SecureRNG::calculate_entropy(&large_output);
        assert!(
            entropy > 7.8,
            "DRBG output should have high entropy: {}",
            entropy
        );

        // Test reseed
        let new_seed = b"completely different seed material";
        drbg.reseed(new_seed);

        let mut out4 = vec![0u8; 32];
        drbg.generate(&mut out4)?;
        assert_ne!(out1, out4, "Output should differ after reseed");

        Ok(())
    }

    /// Test HMAC-DRBG reseed counter limit
    #[test]
    fn test_hmac_drbg_reseed_limit() -> Result<()> {
        let seed = b"test seed";
        let mut drbg = HmacDrbg::new(seed);

        // Set counter to MAX_REQUESTS - 2

        drbg.reseed_counter = HmacDrbg::MAX_REQUESTS - 2;

        let mut out = vec![0u8; 32];

        // This should work (counter becomes MAX_REQUESTS - 1)
        assert!(drbg.generate(&mut out).is_ok());

        // This should also work (counter becomes MAX_REQUESTS)
        assert!(drbg.generate(&mut out).is_ok());

        assert!(
            drbg.generate(&mut out).is_err(),
            "Should fail when reseed counter hits limit"
        );

        // After reseed, should work again
        drbg.reseed(b"fresh entropy");
        assert!(drbg.generate(&mut out).is_ok(), "Should work after reseed");

        Ok(())
    }

    /// Test RNG output randomness
    #[test]
    fn test_rng_randomness() -> Result<()> {
        let mut rng = SecureRNG::new()?;

        // Generate 10KB of random data
        let mut buffer = vec![0u8; 10240];
        rng.fill_bytes(&mut buffer)?;

        // Test entropy
        let entropy = SecureRNG::calculate_entropy(&buffer);
        assert!(
            entropy > 7.5,
            "RNG output should have high entropy: {}",
            entropy
        );

        // Test for obvious patterns
        assert!(buffer.iter().any(|&b| b != 0), "Should not be all zeros");
        assert!(buffer.iter().any(|&b| b != 0xFF), "Should not be all ones");

        // Test uniqueness - generate another buffer
        let mut buffer2 = vec![0u8; 10240];
        rng.fill_bytes(&mut buffer2)?;

        assert_ne!(buffer, buffer2, "Two random buffers should be different");

        // Test randomness verification
        assert!(verify_randomness(&buffer)?, "Should pass randomness tests");

        Ok(())
    }

    /// Test RNG reseeding
    #[test]
    fn test_reseeding() -> Result<()> {
        let mut rng = SecureRNG::new()?;

        // Generate initial data
        let mut buffer1 = vec![0u8; 32];
        rng.fill_bytes(&mut buffer1)?;

        // Force reseed
        rng.reseed()?;

        // Generate post-reseed data
        let mut buffer2 = vec![0u8; 32];
        rng.fill_bytes(&mut buffer2)?;

        // Should be different (extremely high probability)
        assert_ne!(buffer1, buffer2, "Output should differ after reseed");

        Ok(())
    }

    /// Test automatic reseeding after byte limit
    #[test]
    fn test_automatic_reseed() -> Result<()> {
        let mut rng = SecureRNG::new()?;

        // Set low limit for testing (normally 2^32)
        rng.max_bytes_before_reseed = 1024;

        // Generate data up to limit
        let mut buffer = vec![0u8; 512];
        rng.fill_bytes(&mut buffer)?;

        let bytes_before = rng.bytes_since_reseed.load(Ordering::SeqCst);
        assert_eq!(bytes_before, 512, "Should track bytes generated");

        // Generate more to trigger reseed
        let mut buffer2 = vec![0u8; 600];
        rng.fill_bytes(&mut buffer2)?;

        let bytes_after = rng.bytes_since_reseed.load(Ordering::SeqCst);
        assert_eq!(bytes_after, 600, "Should reset counter after reseed");

        Ok(())
    }

    /// Test entropy sources
    #[test]
    fn test_entropy_sources() {
        // Test Hardware RNG
        let hwrng = HardwareRNG::new();
        println!("Hardware RNG available: {}", hwrng.is_available());
        if hwrng.is_available() {
            let mut buffer = vec![0u8; 32];
            assert!(hwrng.fill_bytes(&mut buffer).is_ok());
            assert_eq!(hwrng.quality(), 1.0);
        }

        // Test Ring System RNG (should always work)
        let ring_rng = RingSystemRNG::new();
        assert!(ring_rng.is_available());
        let mut buffer = vec![0u8; 32];
        assert!(ring_rng.fill_bytes(&mut buffer).is_ok());
        assert!(ring_rng.quality() > 0.9);

        // Test URandom
        let urandom = URandom::new();
        if urandom.is_available() {
            let mut buffer = vec![0u8; 32];
            assert!(urandom.fill_bytes(&mut buffer).is_ok());
        }

        // Test Jitter Entropy (always available)
        let jitter = JitterEntropy::new();
        assert!(jitter.is_available());
        let mut buffer = vec![0u8; 32];
        assert!(jitter.fill_bytes(&mut buffer).is_ok());

        // Verify jitter produces different values
        let mut buffer2 = vec![0u8; 32];
        assert!(jitter.fill_bytes(&mut buffer2).is_ok());
        assert_ne!(buffer, buffer2, "Jitter should produce different values");
    }

    /// Test entropy pool mixing
    #[test]
    fn test_entropy_pool() {
        let mut pool = EntropyPool::new();

        // Add entropy
        let entropy1 = vec![0xAA; 32];
        pool.add_entropy(&entropy1);

        // Extract should be different from input
        let mut output1 = vec![0u8; 32];
        pool.extract_bytes(&mut output1);
        assert_ne!(output1, entropy1, "Pool should transform entropy");

        // Add more entropy
        let entropy2 = vec![0x55; 32];
        pool.add_entropy(&entropy2);

        // Extract again - should be different
        let mut output2 = vec![0u8; 32];
        pool.extract_bytes(&mut output2);
        assert_ne!(
            output2, output1,
            "Pool output should change with new entropy"
        );
    }

    /// Test thread safety
    #[test]
    fn test_thread_safety() -> Result<()> {
        use std::sync::Arc;
        use std::thread;

        let rng = Arc::new(ThreadSafeRNG::new()?);
        let mut handles = vec![];

        // Spawn multiple threads using the RNG
        for i in 0..10 {
            let rng_clone = Arc::clone(&rng);
            let handle = thread::spawn(move || {
                let mut buffer = vec![0u8; 1024];
                for _ in 0..100 {
                    rng_clone.fill_bytes(&mut buffer).unwrap();
                    // Verify entropy
                    let entropy = SecureRNG::calculate_entropy(&buffer);
                    assert!(entropy > 7.0, "Thread {} produced low entropy", i);
                }
            });
            handles.push(handle);
        }

        // Wait for all threads
        for handle in handles {
            handle.join().unwrap();
        }

        // RNG should still be healthy
        assert!(rng.is_healthy());

        Ok(())
    }

    /// Test global RNG instance
    #[test]
    fn test_global_rng() -> Result<()> {
        // Get global instance
        let rng = get_secure_rng();

        // Should be healthy
        assert!(rng.is_healthy());

        // Should produce random data
        let mut buffer = vec![0u8; 256];
        rng.fill_bytes(&mut buffer)?;

        let entropy = SecureRNG::calculate_entropy(&buffer);
        assert!(entropy > 7.0, "Global RNG should produce high entropy");

        // Convenience function should work
        let mut buffer2 = vec![0u8; 256];
        secure_random_bytes(&mut buffer2)?;

        assert_ne!(buffer, buffer2, "Should produce different data");

        Ok(())
    }

    /// Test fallback mechanism
    #[test]
    fn test_fallback_sources() -> Result<()> {
        // Create RNG with only jitter entropy (simulate no hardware RNG)
        let primary = Box::new(JitterEntropy::new()) as Box<dyn EntropySource>;
        let secondary = Box::new(RingSystemRNG::new()) as Box<dyn EntropySource>;
        let tertiary = Box::new(JitterEntropy::new()) as Box<dyn EntropySource>;

        let mut rng = SecureRNG {
            primary_source: primary,
            secondary_source: secondary,
            tertiary_source: tertiary,
            entropy_pool: Arc::new(Mutex::new(EntropyPool::new())),
            bytes_since_reseed: Arc::new(AtomicU64::new(0)),
            max_bytes_before_reseed: 1u64 << 32,
            is_healthy: Arc::new(AtomicBool::new(true)),
            continuous_test: Arc::new(Mutex::new(ContinuousTest::new())),
            drbg: None,
        };

        // Initialize DRBG (this was missing!)
        rng.seed_drbg()?;

        // Should still work with fallback sources
        let mut buffer = vec![0u8; 1024];
        rng.fill_bytes(&mut buffer)?;

        let entropy = SecureRNG::calculate_entropy(&buffer);
        assert!(
            entropy > 7.0,
            "Fallback sources should provide good entropy"
        );

        Ok(())
    }

    /// NIST SP 800-22 randomness tests (simplified)
    #[test]
    fn test_nist_randomness() -> Result<()> {
        let mut rng = SecureRNG::new()?;

        // Generate test data
        let mut data = vec![0u8; 10000];
        rng.fill_bytes(&mut data)?;

        // Monobit frequency test
        let ones: u64 = data.iter().map(|b| b.count_ones() as u64).sum();
        let zeros = (data.len() * 8) as u64 - ones;
        let ratio = ones as f64 / (ones + zeros) as f64;

        assert!(
            ratio > 0.49 && ratio < 0.51,
            "Bit ratio should be near 0.5: {}",
            ratio
        );

        // Runs test (consecutive 0s or 1s)
        let mut runs = 0;
        let mut last_bit = false;

        for byte in &data {
            for i in 0..8 {
                let bit = (byte >> i) & 1 == 1;
                if bit != last_bit {
                    runs += 1;
                    last_bit = bit;
                }
            }
        }

        let expected_runs = data.len() * 4; // Approximately
        let runs_ratio = runs as f64 / expected_runs as f64;

        assert!(
            runs_ratio > 0.9 && runs_ratio < 1.1,
            "Runs ratio should be near 1.0: {}",
            runs_ratio
        );

        Ok(())
    }

    /// Test entropy quality estimate
    #[test]
    fn test_entropy_estimate() -> Result<()> {
        let rng = SecureRNG::new()?;

        let estimate = rng.get_entropy_estimate();
        assert!(
            estimate > 0.0 && estimate <= 1.0,
            "Entropy estimate should be between 0 and 1: {}",
            estimate
        );

        println!("Entropy quality estimate: {:.2}", estimate);

        Ok(())
    }

    /// Performance benchmark
    #[test]
    fn test_performance() -> Result<()> {
        use std::time::Instant;

        let mut rng = SecureRNG::new()?;
        let mut buffer = vec![0u8; 1024 * 1024]; // 1MB

        let start = Instant::now();
        rng.fill_bytes(&mut buffer)?;
        let duration = start.elapsed();

        let mb_per_sec = 1.0 / duration.as_secs_f64();
        println!("RNG Performance: {:.2} MB/s", mb_per_sec);

        // Should be at least 10 MB/s for acceptable performance (relaxed for CI/debug builds)
        assert!(
            mb_per_sec > 5.0,
            "RNG too slow: {:.2} MB/s (minimum 5 MB/s)",
            mb_per_sec
        );

        Ok(())
    }
}

#[cfg(test)]
mod statistical_tests {
    use crate::crypto::secure_rng::SecureRNG;
    type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

    /// Chi-square test for uniform distribution
    #[test]
    fn test_chi_square() -> Result<()> {
        let mut rng = SecureRNG::new()?;
        let mut data = vec![0u8; 100000];
        rng.fill_bytes(&mut data)?;

        // Count byte frequencies
        let mut frequencies = [0u64; 256];
        for &byte in &data {
            frequencies[byte as usize] += 1;
        }

        // Calculate chi-square statistic
        let expected = data.len() as f64 / 256.0;
        let mut chi_square = 0.0;

        for &freq in &frequencies {
            let diff = freq as f64 - expected;
            chi_square += (diff * diff) / expected;
        }

        // Degrees of freedom = 255, critical value at 99% confidence ≈ 310
        println!("Chi-square statistic: {:.2}", chi_square);
        assert!(
            chi_square < 310.0,
            "Chi-square too high, distribution may not be uniform"
        );

        Ok(())
    }

    /// Autocorrelation test
    #[test]
    fn test_autocorrelation() -> Result<()> {
        let mut rng = SecureRNG::new()?;
        let mut data = vec![0u8; 10000];
        rng.fill_bytes(&mut data)?;

        // Test autocorrelation at different lags
        for lag in [1, 2, 8, 16].iter() {
            let mut correlation = 0.0;
            let n = data.len() - lag;

            for i in 0..n {
                correlation += (data[i] as f64) * (data[i + lag] as f64);
            }

            correlation /= n as f64;
            let expected = 127.5 * 127.5; // Expected for random data
            let ratio = correlation / expected;

            println!("Autocorrelation at lag {}: {:.3}", lag, ratio);

            assert!(
                ratio > 0.97 && ratio < 1.03,
                "Autocorrelation should be near 1.0 for lag {}",
                lag
            );
        }

        Ok(())
    }
}
