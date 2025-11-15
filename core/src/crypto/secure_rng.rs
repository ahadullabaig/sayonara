#![allow(dead_code)]
use anyhow::{anyhow, Result};
use ring::rand::{SecureRandom, SystemRandom};
use sha2::{Digest, Sha256, Sha512};
use std::fs::File;
use std::io::Read;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

/// FIPS 140-2 compliant secure random number generator with multiple entropy sources
pub struct SecureRNG {
    /// Primary entropy source (hardware if available)
    pub(crate) primary_source: Box<dyn EntropySource>,
    /// Secondary entropy source (OS cryptographic RNG)
    pub(crate) secondary_source: Box<dyn EntropySource>,
    /// Tertiary entropy source (fallback)
    pub(crate) tertiary_source: Box<dyn EntropySource>,
    /// Entropy pool for mixing multiple sources
    pub(crate) entropy_pool: Arc<Mutex<EntropyPool>>,
    /// Bytes generated since last reseed
    pub(crate) bytes_since_reseed: Arc<AtomicU64>,
    /// Maximum bytes before automatic reseed (2^32)
    pub(crate) max_bytes_before_reseed: u64,
    /// Health check status
    pub(crate) is_healthy: Arc<AtomicBool>,
    /// FIPS 140-2 continuous test state
    pub(crate) continuous_test: Arc<Mutex<ContinuousTest>>,
    /// Persistent HMAC-DRBG seeded from the entropy pool (fast)
    pub(crate) drbg: Option<HmacDrbg>,
}

/// Trait for entropy sources
pub trait EntropySource: Send + Sync {
    /// Fill buffer with random bytes
    fn fill_bytes(&self, dest: &mut [u8]) -> Result<()>;
    /// Get entropy quality estimate (0.0 to 1.0)
    fn quality(&self) -> f64;
    /// Check if source is available
    fn is_available(&self) -> bool;
    /// Get source name for logging
    fn name(&self) -> &str;
}

/// Hardware RNG entropy source (/dev/hwrng on Linux)
pub struct HardwareRNG {
    available: bool,
}

impl Default for HardwareRNG {
    fn default() -> Self {
        Self::new()
    }
}

impl HardwareRNG {
    pub fn new() -> Self {
        // Check if hardware RNG is available
        let mut available = std::path::Path::new("/dev/hwrng").exists();
        if available {
            // Try a tiny test read to ensure the device is actually usable
            match File::open("/dev/hwrng").and_then(|mut f| {
                let mut buf = [0u8; 1];
                f.read_exact(&mut buf)
            }) {
                Ok(_) => println!("✓ Hardware RNG detected and readable (/dev/hwrng)"),
                Err(e) => {
                    println!("✗ /dev/hwrng exists but is not readable: {}", e);
                    available = false;
                }
            }
        } else {
            println!("✗ Hardware RNG not available");
        }
        Self { available }
    }
}

impl EntropySource for HardwareRNG {
    fn fill_bytes(&self, dest: &mut [u8]) -> Result<()> {
        if !self.available {
            return Err(anyhow!("Hardware RNG not available"));
        }

        let mut file =
            File::open("/dev/hwrng").map_err(|e| anyhow!("Failed to open /dev/hwrng: {}", e))?;

        // Robust read: loop until we've filled the requested buffer
        let mut total_read = 0usize;
        while total_read < dest.len() {
            let n = file
                .read(&mut dest[total_read..])
                .map_err(|e| anyhow!("Failed to read from /dev/hwrng: {}", e))?;
            if n == 0 {
                return Err(anyhow!("Unexpected EOF reading /dev/hwrng"));
            }
            total_read += n;
        }

        Ok(())
    }

    fn quality(&self) -> f64 {
        if self.available {
            1.0
        } else {
            0.0
        }
    }

    fn is_available(&self) -> bool {
        self.available
    }

    fn name(&self) -> &str {
        "HardwareRNG"
    }
}

/// Ring-based system random (uses OS facilities)
pub struct RingSystemRNG {
    rng: SystemRandom,
}

impl Default for RingSystemRNG {
    fn default() -> Self {
        Self::new()
    }
}

impl RingSystemRNG {
    pub fn new() -> Self {
        Self {
            rng: SystemRandom::new(),
        }
    }
}

impl EntropySource for RingSystemRNG {
    fn fill_bytes(&self, dest: &mut [u8]) -> Result<()> {
        self.rng
            .fill(dest)
            .map_err(|_| anyhow!("Ring SystemRandom failed"))?;
        Ok(())
    }

    fn quality(&self) -> f64 {
        0.95 // High quality OS entropy
    }

    fn is_available(&self) -> bool {
        true // Always available
    }

    fn name(&self) -> &str {
        "RingSystemRNG"
    }
}

/// OS urandom entropy source
pub struct URandom {
    available: bool,
}

impl Default for URandom {
    fn default() -> Self {
        Self::new()
    }
}

impl URandom {
    pub fn new() -> Self {
        let available = std::path::Path::new("/dev/urandom").exists();
        Self { available }
    }
}

impl EntropySource for URandom {
    fn fill_bytes(&self, dest: &mut [u8]) -> Result<()> {
        if !self.available {
            return Err(anyhow!("/dev/urandom not available"));
        }

        let mut file = File::open("/dev/urandom")
            .map_err(|e| anyhow!("Failed to open /dev/urandom: {}", e))?;

        file.read_exact(dest)
            .map_err(|e| anyhow!("Failed to read from /dev/urandom: {}", e))?;

        Ok(())
    }

    fn quality(&self) -> f64 {
        if self.available {
            0.9
        } else {
            0.0
        }
    }

    fn is_available(&self) -> bool {
        self.available
    }

    fn name(&self) -> &str {
        "URandom"
    }
}

/// Jitter entropy source (CPU timing variations)
pub struct JitterEntropy {
    last_value: Arc<Mutex<u64>>,
}

impl Default for JitterEntropy {
    fn default() -> Self {
        Self::new()
    }
}

impl JitterEntropy {
    pub fn new() -> Self {
        Self {
            last_value: Arc::new(Mutex::new(0)),
        }
    }

    fn collect_jitter_entropy(&self) -> Vec<u8> {
        let mut entropy = Vec::new();

        // Collect timing jitter from various sources
        for _ in 0..256 {
            let start = Instant::now();

            // Do some CPU work to create jitter
            let mut x = 1u64;
            for i in 1..100 {
                x = x.wrapping_mul(i).wrapping_add(i);
                std::hint::black_box(&x);
            }

            let elapsed = start.elapsed().as_nanos() as u64;

            // Mix in system time
            let sys_time = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64;

            // XOR with previous value for differential entropy
            let mut last = self.last_value.lock().unwrap();
            let diff = elapsed ^ *last ^ sys_time;
            *last = elapsed;

            entropy.extend_from_slice(&diff.to_le_bytes());
        }

        // Additional entropy from memory allocation timing
        for _ in 0..32 {
            let start = Instant::now();
            let _v: Vec<u8> = Vec::with_capacity(1024);
            let elapsed = start.elapsed().as_nanos() as u64;
            entropy.extend_from_slice(&elapsed.to_le_bytes());
        }

        entropy
    }
}

impl EntropySource for JitterEntropy {
    fn fill_bytes(&self, dest: &mut [u8]) -> Result<()> {
        let raw_entropy = self.collect_jitter_entropy();

        // Use SHA-512 to whiten the entropy
        let mut hasher = Sha512::new();
        hasher.update(&raw_entropy);
        let hash = hasher.finalize();

        // Fill destination with whitened entropy
        for (i, byte) in dest.iter_mut().enumerate() {
            *byte = hash[i % hash.len()];
        }

        // If we need more than 64 bytes, hash again with counter
        if dest.len() > 64 {
            let mut offset = 64;
            let mut counter = 0u64;

            while offset < dest.len() {
                let mut hasher = Sha512::new();
                hasher.update(&raw_entropy);
                hasher.update(counter.to_le_bytes());
                let hash = hasher.finalize();

                let copy_len = std::cmp::min(64, dest.len() - offset);
                dest[offset..offset + copy_len].copy_from_slice(&hash[..copy_len]);

                offset += copy_len;
                counter += 1;
            }
        }

        Ok(())
    }

    fn quality(&self) -> f64 {
        0.5 // Lower quality but always available
    }

    fn is_available(&self) -> bool {
        true // Always available
    }

    fn name(&self) -> &str {
        "JitterEntropy"
    }
}

/// Minimal HMAC-SHA256 helper (self-contained to avoid hmac crate version conflicts).
fn hmac_sha256(key: &[u8], data: &[u8]) -> [u8; 32] {
    const BLOCK_SIZE: usize = 64;
    // Step 1: K0 = (key > blocksize) ? SHA256(key) : key ; then pad to BLOCK_SIZE with zeros
    let mut key_block = if key.len() > BLOCK_SIZE {
        let mut h = Sha256::new();
        h.update(key);
        h.finalize().to_vec()
    } else {
        key.to_vec()
    };
    key_block.resize(BLOCK_SIZE, 0u8);

    // ipad / opad
    let mut ipad = [0x36u8; BLOCK_SIZE];
    let mut opad = [0x5cu8; BLOCK_SIZE];
    for i in 0..BLOCK_SIZE {
        ipad[i] ^= key_block[i];
        opad[i] ^= key_block[i];
    }

    // inner = SHA256(ipad || data)
    let mut inner = Sha256::new();
    inner.update(ipad);
    inner.update(data);
    let inner_res = inner.finalize();

    // outer = SHA256(opad || inner)
    let mut outer = Sha256::new();
    outer.update(opad);
    outer.update(inner_res);
    let out = outer.finalize();

    let mut ret = [0u8; 32];
    ret.copy_from_slice(&out);
    ret
}

/// Minimal HMAC-DRBG (HMAC-SHA256) implementation (NIST SP800-90A style),
/// implemented using the local `hmac_sha256` helper (no external hmac crate).
pub(crate) struct HmacDrbg {
    k: Vec<u8>, // Key (K) - 32 bytes
    v: Vec<u8>, // Value (V) - 32 bytes
    pub(crate) reseed_counter: u64,
}

impl HmacDrbg {
    /// Maximum number of requests between reseeds (2^48 per NIST SP800-90A)
    pub(crate) const MAX_REQUESTS: u64 = 1u64 << 48;

    /// Instantiate with seed_material (entropy_input || nonce || personalization)
    pub(crate) fn new(seed_material: &[u8]) -> Self {
        // K = 0x00..00, V = 0x01..01 (32 bytes each for SHA-256)
        let mut drbg = Self {
            k: vec![0u8; 32],
            v: vec![0x01u8; 32],
            reseed_counter: 1,
        };
        drbg.update(seed_material);
        drbg
    }

    /// Update function as in SP800-90A (accepts optional provided_data)
    fn update(&mut self, provided_data: &[u8]) {
        // Step 1: K = HMAC(K, V || 0x00 || provided_data)
        let mut t = Vec::with_capacity(self.v.len() + 1 + provided_data.len());
        t.extend_from_slice(&self.v);
        t.push(0x00);
        if !provided_data.is_empty() {
            t.extend_from_slice(provided_data);
        }
        self.k = hmac_sha256(&self.k, &t).to_vec();

        // V = HMAC(K, V)
        self.v = hmac_sha256(&self.k, &self.v).to_vec();

        // If provided_data present, do second update round with 0x01
        if !provided_data.is_empty() {
            let mut t2 = Vec::with_capacity(self.v.len() + 1 + provided_data.len());
            t2.extend_from_slice(&self.v);
            t2.push(0x01);
            t2.extend_from_slice(provided_data);
            self.k = hmac_sha256(&self.k, &t2).to_vec();
            self.v = hmac_sha256(&self.k, &self.v).to_vec();
        }
    }

    /// Reseed the DRBG with fresh seed_material
    pub(crate) fn reseed(&mut self, seed_material: &[u8]) {
        self.update(seed_material);
        self.reseed_counter = 1;
    }

    /// Generate output bytes into `out`.
    pub(crate) fn generate(&mut self, out: &mut [u8]) -> Result<()> {
        // Check if reseed is required
        if self.reseed_counter >= Self::MAX_REQUESTS {
            return Err(anyhow!(
                "DRBG requires reseeding after {} requests",
                Self::MAX_REQUESTS
            ));
        }

        let mut generated = Vec::with_capacity(out.len());
        while generated.len() < out.len() {
            self.v = hmac_sha256(&self.k, &self.v).to_vec();
            generated.extend_from_slice(&self.v);
        }

        out.copy_from_slice(&generated[..out.len()]);

        // Per SP800-90A, do Update with no additional input to advance internal state
        self.update(&[]);
        self.reseed_counter = self.reseed_counter.saturating_add(1);

        Ok(())
    }

    /// Check if DRBG needs reseeding
    pub(crate) fn needs_reseed(&self) -> bool {
        self.reseed_counter >= Self::MAX_REQUESTS
    }
}

/// Entropy pool for mixing multiple sources
pub(crate) struct EntropyPool {
    pub(crate) pool: Vec<u8>,
    pub(crate) position: usize,
    pub(crate) hash_state: Sha256,
}

impl EntropyPool {
    pub(crate) fn new() -> Self {
        Self {
            pool: vec![0u8; 512], // 512 pool for better performance
            position: 0,
            hash_state: Sha256::new(),
        }
    }

    pub(crate) fn add_entropy(&mut self, data: &[u8]) {
        // Mix new entropy into pool using XOR
        for (i, &byte) in data.iter().enumerate() {
            let idx = (self.position + i) % self.pool.len();
            self.pool[idx] ^= byte;
        }

        // Update hash state
        self.hash_state.update(data);

        // Advance position
        self.position = (self.position + data.len()) % self.pool.len();
    }

    pub(crate) fn extract_bytes(&mut self, dest: &mut [u8]) {
        // Build seed material from pool + position + time
        let mut hasher = Sha256::new();
        hasher.update(&self.pool);
        hasher.update(self.position.to_le_bytes());
        let time_bytes = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
            .to_le_bytes();
        hasher.update(time_bytes);
        let seed = hasher.finalize();

        // Instantiate HMAC-DRBG seeded with the hashed pool
        let mut drbg = HmacDrbg::new(&seed);

        // Generate requested bytes
        drbg.generate(dest)
            .expect("Fresh DRBG should not need reseed");

        // Fold some of the output back into the pool to mix state (whitening)
        let mut fold_hasher = Sha256::new();
        fold_hasher.update(&dest[..]); // <-- reborrow immutably to avoid move errors
        let fold = fold_hasher.finalize(); // 32 bytes

        for (i, &b) in fold.iter().enumerate() {
            let idx = (self.position + i) % self.pool.len();
            self.pool[idx] ^= b;
        }

        // Advance position
        self.position = (self.position + dest.len()) % self.pool.len();
    }
}

/// FIPS 140-2 continuous random number generator test
pub(crate) struct ContinuousTest {
    last_block: Option<Vec<u8>>,
    failure_count: u64,
}

impl ContinuousTest {
    pub(crate) fn new() -> Self {
        Self {
            last_block: None,
            failure_count: 0,
        }
    }

    pub(crate) fn test(&mut self, data: &[u8]) -> bool {
        // Test 16-byte blocks as per FIPS 140-2
        if data.len() < 16 {
            return true; // Skip test for small blocks
        }

        let test_block = &data[..16];

        if let Some(ref last) = self.last_block {
            if last == test_block {
                self.failure_count += 1;
                println!("⚠️ FIPS 140-2 continuous test failed! Identical blocks detected.");
                return false;
            }
        }

        self.last_block = Some(test_block.to_vec());
        true
    }
}

impl SecureRNG {
    /// Create a new FIPS 140-2 compliant secure RNG
    pub fn new() -> Result<Self> {
        println!("Initializing Secure RNG with multiple entropy sources...");

        // Initialize entropy sources in order of preference
        let primary = Box::new(HardwareRNG::new()) as Box<dyn EntropySource>;
        let secondary = Box::new(RingSystemRNG::new()) as Box<dyn EntropySource>;
        let tertiary = Box::new(JitterEntropy::new()) as Box<dyn EntropySource>;

        // Check that at least one source is available
        if !primary.is_available() && !secondary.is_available() && !tertiary.is_available() {
            return Err(anyhow!("No entropy sources available!"));
        }

        let mut rng = Self {
            primary_source: primary,
            secondary_source: secondary,
            tertiary_source: tertiary,
            entropy_pool: Arc::new(Mutex::new(EntropyPool::new())),
            bytes_since_reseed: Arc::new(AtomicU64::new(0)),
            max_bytes_before_reseed: 1u64 << 32, // 4GB
            is_healthy: Arc::new(AtomicBool::new(true)),
            continuous_test: Arc::new(Mutex::new(ContinuousTest::new())),
            drbg: None,
        };

        // Initial seeding from all available sources
        rng.reseed()?;

        // Initialize persistent HMAC-DRBG seeded from pool
        rng.seed_drbg()?;

        println!("✓ Secure RNG initialized successfully");

        Ok(rng)
    }

    /// Fill buffer with cryptographically secure random bytes
    pub fn fill_bytes(&mut self, dest: &mut [u8]) -> Result<()> {
        // Check if reseed is needed
        let bytes_generated = self.bytes_since_reseed.load(Ordering::SeqCst);
        let request_len = dest.len() as u64;

        // Also check if DRBG needs reseeding
        let drbg_needs_reseed = self.drbg.as_ref().is_some_and(|d| d.needs_reseed());

        if bytes_generated.saturating_add(request_len) >= self.max_bytes_before_reseed
            || drbg_needs_reseed
        {
            // Reseed before generating so post-call counter reflects only this request.
            self.reseed()?;
        }

        // Try primary source first
        let mut success = false;
        if self.primary_source.is_available() {
            if let Ok(()) = self.primary_source.fill_bytes(dest) {
                success = true;
            }
        }

        // Fallback to secondary source
        if !success && self.secondary_source.is_available() {
            if let Ok(()) = self.secondary_source.fill_bytes(dest) {
                success = true;
            }
        }

        // Last resort: tertiary source
        if !success {
            self.tertiary_source.fill_bytes(dest)?;
        }

        // Mix with entropy pool for defense in depth (use persistent HMAC-DRBG seeded from pool)
        {
            let mut pool = self.entropy_pool.lock().unwrap();
            let mut pool_bytes = vec![0u8; dest.len()];

            // Fast path: use persistent DRBG if available
            if let Some(ref mut drbg) = self.drbg {
                // drbg is owned by SecureRNG; we're executing under &mut self so it's safe to call
                drbg.generate(&mut pool_bytes)?;
            } else {
                // Fallback: use the pool's (older) extract_bytes implementation
                pool.extract_bytes(&mut pool_bytes);
            }

            // XOR keystream with output
            for (d, p) in dest.iter_mut().zip(pool_bytes.iter()) {
                *d ^= *p;
            }

            // Fold some of the keystream back into the pool to improve mixing
            let mut fold_hasher = Sha256::new();
            fold_hasher.update(&pool_bytes[..std::cmp::min(pool_bytes.len(), 64)]);
            let fold = fold_hasher.finalize();
            pool.add_entropy(&fold);
        }

        // Run FIPS 140-2 continuous test
        {
            let mut test = self.continuous_test.lock().unwrap();
            if !test.test(dest) {
                self.is_healthy.store(false, Ordering::SeqCst);
                return Err(anyhow!("FIPS 140-2 continuous test failed"));
            }
        }

        // Update byte counter
        self.bytes_since_reseed
            .fetch_add(dest.len() as u64, Ordering::SeqCst);

        Ok(())
    }

    /// Reseed the RNG from all available entropy sources
    pub fn reseed(&mut self) -> Result<()> {
        println!("Reseeding RNG from all entropy sources...");

        let mut seed_data = Vec::new();

        // Collect from all available sources
        let sources: Vec<&Box<dyn EntropySource>> = vec![
            &self.primary_source,
            &self.secondary_source,
            &self.tertiary_source,
        ];

        for source in sources {
            if source.is_available() {
                let mut buffer = vec![0u8; 256];
                if source.fill_bytes(&mut buffer).is_ok() {
                    seed_data.extend_from_slice(&buffer);
                    println!("  ✓ Collected entropy from {}", source.name());
                }
            }
        }

        // Add time-based entropy
        let time_entropy = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
            .to_le_bytes();
        seed_data.extend_from_slice(&time_entropy);

        // Mix into entropy pool
        {
            let mut pool = self.entropy_pool.lock().unwrap();
            pool.add_entropy(&seed_data);
        }

        // Reset counter
        self.bytes_since_reseed.store(0, Ordering::SeqCst);

        // Reseed persistent DRBG from updated pool
        self.seed_drbg()?;

        println!("✓ RNG reseeded with {} bytes of entropy", seed_data.len());

        Ok(())
    }

    /// Initialize or reseed the persistent HMAC-DRBG from the current entropy pool.
    pub(crate) fn seed_drbg(&mut self) -> Result<()> {
        // Build seed material from pool + position + time
        let mut hasher = Sha256::new();
        {
            let pool = self.entropy_pool.lock().unwrap();
            hasher.update(&pool.pool);
            hasher.update(pool.position.to_le_bytes());
        }
        let time_bytes = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
            .to_le_bytes();
        hasher.update(time_bytes);
        let seed = hasher.finalize();

        // Replace or create persistent DRBG
        if let Some(ref mut drbg) = self.drbg {
            drbg.reseed(&seed);
        } else {
            self.drbg = Some(HmacDrbg::new(&seed));
        }

        Ok(())
    }

    /// Get entropy quality estimate (0.0 to 1.0)
    pub fn get_entropy_estimate(&self) -> f64 {
        let primary_quality = if self.primary_source.is_available() {
            self.primary_source.quality()
        } else {
            0.0
        };

        let secondary_quality = if self.secondary_source.is_available() {
            self.secondary_source.quality()
        } else {
            0.0
        };

        let tertiary_quality = self.tertiary_source.quality();

        // Weighted average with primary source having most weight
        (primary_quality * 0.5 + secondary_quality * 0.3 + tertiary_quality * 0.2).clamp(0.0, 1.0)
    }

    /// Check if RNG is healthy
    pub fn is_healthy(&self) -> bool {
        self.is_healthy.load(Ordering::SeqCst)
    }

    /// Calculate Shannon entropy of data (for verification)
    pub fn calculate_entropy(data: &[u8]) -> f64 {
        let mut counts = [0u64; 256];
        for &byte in data {
            counts[byte as usize] += 1;
        }

        let length = data.len() as f64;
        let mut entropy = 0.0;

        for &count in &counts {
            if count > 0 {
                let probability = count as f64 / length;
                entropy -= probability * probability.log2();
            }
        }

        entropy
    }
}

/// Thread-safe wrapper for SecureRNG
pub struct ThreadSafeRNG {
    inner: Arc<Mutex<SecureRNG>>,
}

impl ThreadSafeRNG {
    pub fn new() -> Result<Self> {
        Ok(Self {
            inner: Arc::new(Mutex::new(SecureRNG::new()?)),
        })
    }

    pub fn fill_bytes(&self, dest: &mut [u8]) -> Result<()> {
        let mut rng = self.inner.lock().unwrap();
        rng.fill_bytes(dest)
    }

    pub fn reseed(&self) -> Result<()> {
        let mut rng = self.inner.lock().unwrap();
        rng.reseed()
    }

    pub fn get_entropy_estimate(&self) -> f64 {
        let rng = self.inner.lock().unwrap();
        rng.get_entropy_estimate()
    }

    pub fn is_healthy(&self) -> bool {
        let rng = self.inner.lock().unwrap();
        rng.is_healthy()
    }
}

lazy_static::lazy_static! {
    static ref GLOBAL_RNG: ThreadSafeRNG = ThreadSafeRNG::new()
        .expect("Failed to initialize global secure RNG");
}

/// Get the global secure RNG instance
pub fn get_secure_rng() -> &'static ThreadSafeRNG {
    &GLOBAL_RNG
}

/// Convenience function to fill bytes using global RNG
pub fn secure_random_bytes(dest: &mut [u8]) -> Result<()> {
    GLOBAL_RNG.fill_bytes(dest)
}

/// Verify randomness quality using NIST SP 800-22 tests (simplified)
pub fn verify_randomness(data: &[u8]) -> Result<bool> {
    if data.len() < 1000 {
        return Err(anyhow!("Need at least 1000 bytes for randomness testing"));
    }

    // Test 1: Monobit frequency test
    let ones = data.iter().map(|b| b.count_ones() as u64).sum::<u64>();
    let zeros = (data.len() * 8) as u64 - ones;
    let diff = (ones as f64 - zeros as f64).abs();
    let expected_diff = (2.0 * (data.len() * 8) as f64).sqrt();

    if diff > expected_diff * 3.0 {
        println!(
            "Failed monobit test: too many {}s",
            if ones > zeros { "1" } else { "0" }
        );
        return Ok(false);
    }

    // Test 2: Entropy test
    let entropy = SecureRNG::calculate_entropy(data);
    if entropy < 7.0 {
        println!(
            "Failed entropy test: {:.2} bits/byte (minimum 7.0)",
            entropy
        );
        return Ok(false);
    }

    // Test 3: Consecutive identical bytes test
    let mut max_run = 0;
    let mut current_run = 1;
    let mut last_byte = data[0];

    for &byte in &data[1..] {
        if byte == last_byte {
            current_run += 1;
            max_run = max_run.max(current_run);
        } else {
            current_run = 1;
            last_byte = byte;
        }
    }

    // Probability of n consecutive identical bytes is (1/256)^(n-1)
    // For 1000 bytes, runs > 5 are very unlikely
    if max_run > 5 {
        println!("Failed run test: {} consecutive identical bytes", max_run);
        return Ok(false);
    }

    println!(
        "✓ Randomness tests passed (entropy: {:.2} bits/byte)",
        entropy
    );
    Ok(true)
}
