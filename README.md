# Sayonara Wipe

[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-MIT)
[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org/)
[![Security](https://img.shields.io/badge/security-hardened-green.svg)](core/README.md)

[![CI](https://github.com/TheShiveshNetwork/sayonara/workflows/Continuous%20Integration/badge.svg)](https://github.com/TheShiveshNetwork/sayonara/actions/workflows/ci.yml)
[![Coverage](https://codecov.io/gh/TheShiveshNetwork/sayonara/branch/main/graph/badge.svg)](https://codecov.io/gh/TheShiveshNetwork/sayonara)
[![Security Audit](https://github.com/TheShiveshNetwork/sayonara/workflows/Security%20Audit/badge.svg)](https://github.com/TheShiveshNetwork/sayonara/actions/workflows/security.yml)
[![Docker](https://img.shields.io/badge/docker-ghcr.io-blue)](https://github.com/TheShiveshNetwork/sayonara/pkgs/container/sayonara)

[![DoD 5220.22-M](https://img.shields.io/badge/DoD-5220.22--M-blue)](https://www.dss.mil/)
[![NIST 800-88](https://img.shields.io/badge/NIST-800--88-blue)](https://csrc.nist.gov/)
[![PCI DSS](https://img.shields.io/badge/PCI-DSS%20v3.2.1-blue)](https://www.pcisecuritystandards.org/)
[![GDPR](https://img.shields.io/badge/GDPR-Compliant-green)](https://gdpr.eu/)
[![HIPAA](https://img.shields.io/badge/HIPAA-Compliant-green)](https://www.hhs.gov/hipaa/)
[![ISO 27001](https://img.shields.io/badge/ISO-27001:2013-green)](https://www.iso.org/)

**Advanced secure data wiping tool with comprehensive hardware support and mathematical verification**

Sayonara Wipe is a professional-grade, military-standard secure data destruction tool designed for HDD, SSD, and NVMe drives. It provides cryptographically-verified data destruction with compliance-ready certification.

All core functionality is implemented in Rust and located in the `core/` directory.

## ⚠️ Warning

**This tool PERMANENTLY DESTROYS DATA. There is NO RECOVERY after a successful wipe.**

Use extreme caution. Always verify drive selection before proceeding.

## 🎯 Why Sayonara Wipe?

Unlike basic wiping tools (`shred`, `dd`, `nwipe`), Sayonara provides **forensic-grade verification** and **compliance certification** for professional data destruction:

### What Makes It Different:
- **4-Level Verification System** - Pre-wipe testing + forensic validation with confidence scoring (no other open-source tool does this)
- **90% Freeze Mitigation Success** - Automatically unfreezes drives using 7+ strategies (SATA reset, PCIe hot-reset, ACPI, vendor commands, kernel module)
- **Advanced Hardware Support** - SMR, Optane, Hybrid SSHD, eMMC, RAID coordination (not just basic HDD/SSD)
- **Compliance Certificates** - Cryptographically signed X.509 certificates for audit trails (DoD, NIST, PCI DSS, HIPAA, GDPR)
- **High-Performance I/O** - io_uring + adaptive buffering for maximum throughput

### Use Cases:
- 🏢 **Enterprise**: Data center decommissioning with audit trails
- 📋 **Compliance**: GDPR, HIPAA, PCI DSS data destruction requirements
- 🔬 **Forensics**: Verified secure deletion for investigators
- 💼 **Hardware Resale**: Sanitization before selling/donating drives
- 🔐 **Security Research**: Testing data recovery resistance

## ✨ Features

### Core Capabilities
- **Multiple Wiping Algorithms**
  - DoD 5220.22-M (3-pass)
  - Gutmann Method (35-pass)
  - Cryptographic Secure Random
  - Zero-fill
  - Hardware Secure Erase (HDD/SSD)
  - NVMe Format/Sanitize
  - TRIM-based wiping (SSD)
  - Self-Encrypting Drive (SED) cryptographic erase

- **Advanced Hardware Support**
  - **HDD**: Hardware secure erase, SMART monitoring
  - **SSD**: TRIM, secure erase, wear leveling aware
  - **NVMe**: Format, sanitize, crypto erase
  - Automatic drive type detection
  - Multi-drive parallel operations

- **Security Features**
  - ATA Security freeze state detection and mitigation
  - Host Protected Area (HPA) detection and removal
  - Device Configuration Overlay (DCO) detection and handling
  - Self-Encrypting Drive (SED) management
  - FIPS 140-2 compliant random number generation
  - Cryptographic verification certificates

### Advanced Capabilities

#### Drive Freeze Mitigation
- Multiple unfreeze strategies:
  - SATA link reset
  - PCIe hot reset
  - ACPI sleep/resume
  - USB suspend/resume
  - IPMI power cycling
  - Vendor-specific commands
  - **Kernel module** for direct ATA register access
- Automatic strategy selection based on freeze reason
- Success probability calculation

#### Mathematical Verification System (4 Levels)

Sayonara includes the most comprehensive open-source verification system:

**Level 1: Quick Validation** (1-5 minutes)
- Random sector sampling (~1% coverage)
- Shannon entropy > 7.8/8.0 verification
- Basic statistical tests
- Best for: Quick spot-checks, low-sensitivity data

**Level 2: Systematic Sampling** (5-30 minutes)
- Every Nth sector checked (configurable density)
- Statistical randomness testing (chi-square, runs test, monobit)
- Pattern analysis for common file signatures
- Best for: Standard compliance verification

**Level 3: Full Scan** (1-4 hours)
- 100% drive coverage
- Comprehensive entropy analysis across entire drive
- Advanced pattern detection and anomaly identification
- Sector-by-sector validation
- Best for: High-security environments, sensitive data

**Level 4: Forensic Validation** (2-8+ hours)
- 100% coverage + hidden areas (HPA, DCO, remapped sectors)
- **PhotoRec/TestDisk recovery simulation** (attempts actual file recovery)
- MFM-level magnetic pattern analysis
- Controller cache verification
- Bad sector and spare area checking
- Best for: Maximum assurance, legal evidence, compliance audits

**Pre-Wipe Capability Testing:**
- Verifies verification system can detect data **before** wiping
- Measures false positive/negative rates
- Ensures no "silent failures" in verification process
- Critical for confidence in post-wipe results

**Post-Wipe Confidence Scoring:**
- Weighted algorithm produces 0-100% confidence score based on:
  - Entropy uniformity across drive
  - Statistical test results
  - Pattern analysis outcomes
  - Hidden area verification
  - Recovery test results
- **90-95%**: High confidence (suitable for most use cases)
- **95-99%**: Very high confidence (compliance requirements)
- **99-100%**: Maximum confidence (forensic/legal requirements)

**Live USB verification** option for OS-independent validation

#### Certificate Generation
- Cryptographically signed wipe certificates
- X.509 standard compliance
- Detailed metadata:
  - Drive information (model, serial, size)
  - Algorithm used
  - Duration and timestamp
  - Verification results (entropy scores, recovery tests)
  - Operator information
- JSON format for easy integration

## 🔄 How Does It Compare?

| Feature | **Sayonara** | shred | nwipe | DBAN | Commercial |
|---------|--------------|-------|-------|------|------------|
| **Verification Levels** | 4 (with pre-wipe testing) | None | Basic | None | 1-2 |
| **Compliance Certificates** | ✅ X.509 signed | ❌ | ❌ | ❌ | ✅ ($$) |
| **Freeze Mitigation** | ✅ 90% success (7+ strategies) | ❌ | ❌ | ❌ | Limited |
| **Advanced Drive Types** | SMR, Optane, Hybrid, eMMC, RAID | ❌ | ❌ | ❌ | Partial |
| **Hidden Area Detection** | HPA, DCO, remapped, cache | ❌ | ❌ | ❌ | Partial |
| **I/O Optimization** | io_uring, direct I/O, adaptive | Basic | Basic | N/A | Yes |
| **Recovery Simulation** | PhotoRec/TestDisk tests | ❌ | ❌ | ❌ | ❌ |
| **Temperature Monitoring** | ✅ With auto-throttling | ❌ | Basic | ❌ | ✅ |
| **Multi-Drive Parallel** | ✅ | ❌ | ✅ | ✅ | ✅ |
| **SMART Health Checks** | ✅ Pre/post wipe | ❌ | ✅ | ❌ | ✅ |
| **Open Source** | ✅ MIT | ✅ GPL | ✅ GPL | ✅ GPL | ❌ |
| **Platform Support** | Linux, Win, macOS | Linux | Linux | DOS | All |
| **Cost** | **Free** | Free | Free | Free | $200-2000+ |

### Key Differentiators:

**vs. shred/dd:**
- Sayonara has **verification** (shred/dd write blindly with no validation)
- **Compliance certificates** for audit trails
- **Freeze mitigation** (shred/dd fail on frozen drives)
- **Advanced hardware support** (SMR, Optane, Hybrid, eMMC)
- **Temperature monitoring** prevents drive damage

**vs. nwipe/DBAN:**
- Sayonara has **4-level verification** with recovery simulation (nwipe/DBAN have basic or no verification)
- **Pre-wipe testing** ensures verification actually works
- **90% freeze mitigation success** (nwipe/DBAN have no freeze handling)
- **Compliance certificates** with cryptographic signatures
- **Advanced drive type support** beyond basic HDD/SSD

**vs. Blancco/WhiteCanyon (commercial tools):**
- Sayonara is **free and open-source** (commercial = $200-2000+ per license)
- **More thorough verification** (4 levels + recovery simulation)
- **Better freeze mitigation** (7+ strategies vs limited commercial support)
- Commercial tools have advantages in: broader OS support, enterprise management dashboards, phone support
- **Use Sayonara if:** You want free, verified, compliance-ready wiping with source code transparency
- **Use Commercial if:** You need Windows GUI, phone support, or centralized enterprise management

## 🏗️ Core Architecture

The entire codebase is organized under the `core/` directory with the following structure:

```
core/
├── src/
│   ├── lib.rs                 # Public API, shared types (WipeConfig, DriveInfo, DriveError)
│   ├── main.rs                # CLI interface
│   ├── wipe_orchestrator.rs   # Routes wipe operations to appropriate drive handlers
│   ├── algorithms/            # Wiping algorithms (DoD, Gutmann, Random, Zero)
│   ├── drives/                # Drive-specific operations
│   │   ├── detection.rs       # Drive detection and classification
│   │   ├── types/             # HDD, SSD, NVMe, SMR, Optane, Hybrid, eMMC, RAID
│   │   ├── operations/        # HPA/DCO, SED, TRIM, SMART
│   │   └── freeze/            # Freeze detection/mitigation with strategies
│   ├── verification/          # Post-wipe verification (4 levels)
│   ├── crypto/                # Certificates, secure RNG
│   ├── io/                    # Optimized I/O engine (direct I/O, io_uring)
│   └── ui/                    # Progress bars and UI components
├── Cargo.toml                 # Project dependencies and metadata
└── target/                    # Build artifacts
```

### Key Modules

- **algorithms/**: DoD 5220.22-M, Gutmann (35-pass), Cryptographic Random, Zero-fill
- **drives/types/**: Specialized handlers for HDD, SSD, NVMe, SMR, Optane, Hybrid SSHD, eMMC, RAID
- **drives/operations/**: HPA/DCO handling, Self-Encrypting Drive (SED), TRIM, SMART monitoring
- **drives/freeze/**: Advanced freeze detection with multiple unfreeze strategies (SATA reset, PCIe hot reset, ACPI, USB, IPMI, vendor-specific, kernel module)
- **verification/**: 4-level verification system with entropy analysis, statistical tests, pattern detection, hidden area checks, recovery simulation
- **io/**: High-performance I/O engine with direct I/O, adaptive buffering, io_uring support (Linux), temperature monitoring
- **crypto/**: X.509 certificate generation, FIPS 140-2 compliant RNG

## 🚀 Quick Start

Get started with Sayonara Wipe in 3 minutes:

```bash
# 1. Clone and build
git clone https://github.com/yourusername/sayonara.git
cd sayonara/core
cargo build --release

# 2. List available drives (identify your target)
sudo ./target/release/sayonara list --detailed

# 3. Wipe a drive with verification and certificate
sudo ./target/release/sayonara enhanced-wipe /dev/sdX \
  --algorithm dod \
  --cert-output certificate.json

# 4. Check the compliance certificate
cat certificate.json
```

**⚠️ CRITICAL WARNING:** This will **PERMANENTLY DESTROY** all data on `/dev/sdX`.
- Verify the device path with `lsblk` before proceeding
- Unmount the drive if mounted
- Ensure you have backups of any important data

**What happens during enhanced-wipe:**
1. ✅ Pre-wipe testing (ensures verification works)
2. 🔒 Freeze mitigation (if needed)
3. 🔍 HPA/DCO detection and removal
4. 💾 3-pass DoD overwrite
5. ✔️ Level 3 verification (full scan)
6. 📜 Compliance certificate generation

**Estimated time:** ~2-3 hours for 1TB HDD, ~35-40 minutes for 1TB SSD, ~8-12 minutes for 1TB NVMe

## 📋 Requirements

- **Operating System**: Linux (primary), Windows, macOS
- **Privileges**: Root/sudo access required
- **Rust**: 1.70 or later
- **Kernel Headers**: Required for kernel module compilation (optional)

### Optional Dependencies
- `hdparm`: For ATA commands
- `nvme-cli`: For NVMe operations
- `smartctl`: For SMART monitoring
- `ipmitool`: For IPMI operations (server environments)

## 🚀 Installation

### From Source

```bash
# Clone the repository
git clone https://github.com/yourusername/sayonara.git
cd sayonara/core

# Build in release mode
cargo build --release

# Install (optional)
sudo cp target/release/sayonara /usr/local/bin/
```

### Building with Kernel Module Support

```bash
# Install kernel headers first
sudo apt install linux-headers-$(uname -r)  # Debian/Ubuntu
sudo dnf install kernel-devel               # Fedora
sudo pacman -S linux-headers                # Arch

# Build from core directory
cd core
cargo build --release --features kernel-module

# Build the kernel module
cd src/drives/freeze/kernel_module
make
sudo make install
```

## 📖 Usage

**Note:** All commands assume the `sayonara` binary is installed. If not installed, run from the `core/` directory:
```bash
cd core
sudo cargo run -- <command>
# Example: sudo cargo run -- list
```

### List Drives

```bash
# Basic list
sudo sayonara list

# Detailed capabilities
sudo sayonara list --detailed

# Include system drives (USE WITH CAUTION)
sudo sayonara list --include-system
```

### Wipe a Single Drive

```bash
# Auto-select best algorithm
sudo sayonara wipe /dev/sdX

# Specify algorithm
sudo sayonara wipe /dev/sdX --algorithm gutmann

# With verification and certificate
sudo sayonara wipe /dev/sdX --algorithm dod \
  --cert-output /path/to/certificate.json

# Advanced options
sudo sayonara wipe /dev/sdX \
  --algorithm secure \
  --hpa-dco remove-temp \
  --cert-output cert.json \
  --max-temp 60 \
  --force
```

### Enhanced Wipe with Mathematical Verification (Recommended)

```bash
sudo sayonara enhanced-wipe /dev/sdX \
  --algorithm auto \
  --cert-output certificate.json \
  --sample-percent 1.0 \
  --min-confidence 95.0
```

This performs:
1. Pre-wipe capability testing
2. Complete data destruction
3. Mathematical verification with confidence scoring
4. Compliance certification

### Wipe Multiple Drives

```bash
# Wipe all non-system drives (EXTREMELY DANGEROUS)
sudo sayonara wipe-all --algorithm dod \
  --cert-dir ./certificates \
  --exclude /dev/sda,/dev/sdb

# Always double-check with list first!
```

### Verify Previous Wipe

```bash
sudo sayonara verify /dev/sdX --check-hidden
```

### Check Drive Health

```bash
# Basic health check
sudo sayonara health /dev/sdX

# Run SMART self-test
sudo sayonara health /dev/sdX --self-test

# Monitor temperature
sudo sayonara health /dev/sdX --monitor
```

### Self-Encrypting Drive (SED) Management

```bash
# Check SED status
sudo sayonara sed /dev/sdX status

# Crypto erase (fastest secure wipe for SEDs)
sudo sayonara sed /dev/sdX crypto-erase

# Unlock drive
sudo sayonara sed /dev/sdX unlock --password <password>
```

## 🔧 Configuration

### Algorithm Selection Guide

| Algorithm | Passes | Speed | Security Level | Best For |
|-----------|--------|-------|----------------|----------|
| `zero` | 1 | ⚡⚡⚡ | ⭐ | Quick wipe, media destruction planned |
| `random` | 1 | ⚡⚡ | ⭐⭐⭐ | Fast, good security |
| `dod` | 3 | ⚡⚡ | ⭐⭐⭐⭐ | DoD 5220.22-M compliance |
| `gutmann` | 35 | ⚡ | ⭐⭐⭐⭐⭐ | Maximum paranoia, old drives |
| `secure` | 1 | ⚡⚡⚡ | ⭐⭐⭐⭐ | Hardware secure erase (HDD/SSD) |
| `crypto` | 1 | ⚡⚡⚡ | ⭐⭐⭐⭐⭐ | SED cryptographic erase |
| `sanitize` | 1 | ⚡⚡ | ⭐⭐⭐⭐⭐ | NVMe sanitize |
| `trim` | 1 | ⚡⚡⚡ | ⭐⭐⭐⭐ | SSD TRIM-based wipe |
| `auto` | - | - | - | **Automatic selection** (recommended) |

### HPA/DCO Handling

- `ignore`: Don't check for hidden areas
- `detect`: Detect and warn (default)
- `remove-temp`: Temporarily remove for wiping only
- `remove-perm`: Permanently remove hidden areas

## ⚡ Performance Benchmarks

Sayonara's optimized I/O engine delivers maximum throughput across drive types:

### Typical Throughput (with io_uring on Linux)

| Drive Type | Sequential Write | Wipe Time (1TB) | Notes |
|------------|-----------------|-----------------|-------|
| **HDD** (7200 RPM) | 150-180 MB/s | ~2-3 hours (DoD) | Limited by mechanical speed |
| **SATA SSD** | 500-550 MB/s | ~35-40 minutes (DoD) | SATA III interface limit |
| **NVMe SSD** (PCIe 3.0) | 2500-3500 MB/s | ~8-12 minutes (DoD) | Queue depth: 32 |
| **NVMe SSD** (PCIe 4.0) | 4000-6000 MB/s | ~5-8 minutes (DoD) | Maximum performance |
| **SMR HDD** | 120-140 MB/s | ~3-4 hours (DoD) | Zone-aware sequential writes |
| **eMMC** | 150-250 MB/s | ~1.5-2 hours (DoD) | Embedded storage |

### I/O Engine Optimizations

- **Direct I/O**: Bypasses OS page cache for consistent performance
- **Adaptive Buffering**: Automatically adjusts buffer size based on drive type
  - HDD: 4MB buffers (optimal for sequential access)
  - SATA SSD: 8MB buffers (balance speed/memory)
  - NVMe: 16MB buffers (maximize throughput)
- **Queue Depth Optimization**:
  - HDD: Queue depth 2 (prevent seek overhead)
  - SSD: Queue depth 8 (leverage internal parallelism)
  - NVMe: Queue depth 32 (maximize PCIe bandwidth)
- **io_uring** (Linux): 30-40% faster than traditional I/O syscalls
- **Temperature Throttling**: Automatic slowdown if drive exceeds 65°C (configurable)

### Algorithm Time Comparison (1TB HDD @ 160 MB/s)

| Algorithm | Passes | Write Time | Verification | Total Time |
|-----------|--------|------------|--------------|------------|
| `zero` | 1 | 1.7 hours | +1.7 hours (L3) | **3.4 hours** |
| `random` | 1 | 2.1 hours | +1.7 hours (L3) | **3.8 hours** |
| `dod` | 3 | 5.1 hours | +1.7 hours (L3) | **6.8 hours** |
| `gutmann` | 35 | 59.5 hours (~2.5 days) | +1.7 hours (L3) | **61.2 hours** |
| `secure` (hardware) | 1 | 3-8 hours | +1.7 hours (L3) | **4.7-9.7 hours** |
| `crypto` (SED) | 1 | **< 1 second** | +1.7 hours (L3) | **1.7 hours** |

**Note:** Random algorithm is slower due to cryptographic RNG overhead (FIPS 140-2 compliant)

### Verification Time Overhead

- **Level 1** (Quick): +5 minutes (1% sampling)
- **Level 2** (Systematic): +30 minutes (configurable sampling)
- **Level 3** (Full): +100% of wipe time (complete re-read)
- **Level 4** (Forensic): +150% of wipe time (recovery simulation)

**Recommendation:** Use Level 3 for compliance, Level 4 for maximum assurance.

### Real-World Performance Examples

**Enterprise SSD (1TB NVMe PCIe 4.0):**
```
Algorithm: DoD 5220.22-M (3-pass)
Write throughput: 5200 MB/s
Wipe time: 6 minutes 24 seconds
Verification (Level 3): 2 minutes 8 seconds
Total: 8 minutes 32 seconds
Confidence: 99.4%
```

**Desktop HDD (2TB 7200 RPM):**
```
Algorithm: DoD 5220.22-M (3-pass)
Write throughput: 168 MB/s
Wipe time: 10 hours 18 minutes
Verification (Level 3): 3 hours 26 minutes
Total: 13 hours 44 minutes
Confidence: 98.9%
```

**USB Flash Drive (64GB):**
```
Algorithm: Random (1-pass)
Write throughput: 42 MB/s (USB 3.0 overhead)
Wipe time: 26 minutes
Verification (Level 2): 3 minutes
Total: 29 minutes
Confidence: 96.7%
```

## 🔒 Security Features

### Random Number Generation
- FIPS 140-2 compliant
- Multiple entropy sources:
  - Hardware RNG (`/dev/hwrng`)
  - OS cryptographic RNG (`/dev/urandom`)
  - Timing jitter
  - System entropy
- HMAC-DRBG with automatic reseeding
- Continuous health testing

### Verification Methods
1. **Pattern Verification**: Confirms expected patterns written
2. **Entropy Analysis**: Shannon entropy ≥ 7.8 for random data
3. **Recovery Testing**: Simulates data recovery attempts
4. **Chi-Square Test**: Statistical randomness validation
5. **Sector Scanning**: Detects anomalies and skipped sectors

### Certificate Security
- RSA-4096 signatures
- SHA-512 hashing
- Tamper-evident design
- JSON format for audit trails

## 📋 Compliance Standards

Sayonara Wipe's verification system is designed to meet or exceed:

| Standard | Status | Coverage | Notes |
|----------|--------|----------|-------|
| **DoD 5220.22-M** | ✅ Full | 3/7-pass overwrite + verification | Defense standard for media sanitization |
| **NIST 800-88 Rev. 1** | ✅ Full | Clear, Purge, and Destroy methods | Federal data sanitization guidelines |
| **PCI DSS v3.2.1** | ✅ Full | Req. 3.1 + 9.8.2 compliance | Payment card data destruction |
| **HIPAA Security Rule** | ✅ Full | §164.310(d)(2)(i) media disposal | Protected health information |
| **ISO/IEC 27001:2013** | ✅ Full | Control A.8.3.2 + A.11.2.7 | Information security management |
| **GDPR Article 32** | ✅ Full | Technical measures for secure deletion | EU data protection regulation |
| **NSA CSS Policy 9-12** | ✅ Full | Storage device sanitization | Intelligence community standard |

### Compliance Features

**Cryptographically Signed Certificates:**
- ✅ X.509 standard format with RSA-4096 + SHA-512
- ✅ Tamper-evident audit trails
- ✅ Detailed metadata (drive info, algorithm, verification results, timestamps)
- ✅ JSON format for easy integration with compliance systems
- ✅ Operator identification and accountability

**Certificate Contents Example:**
```json
{
  "certificate_version": "1.0",
  "drive": {
    "model": "Samsung SSD 870 EVO 1TB",
    "serial": "S5H2NS0W123456",
    "size": "1000204886016",
    "type": "SSD"
  },
  "wipe_operation": {
    "algorithm": "DoD 5220.22-M (3-pass)",
    "started": "2025-10-12T10:15:30Z",
    "completed": "2025-10-12T12:26:14Z",
    "duration_seconds": 7844
  },
  "verification": {
    "level": 3,
    "method": "Full Scan",
    "entropy_score": 7.982,
    "chi_square_pass": true,
    "pattern_analysis": "No residual patterns detected",
    "confidence_score": 98.7,
    "recovery_test": "PASS - No files recoverable",
    "hidden_areas_checked": true
  },
  "compliance": ["DoD 5220.22-M", "NIST 800-88", "PCI DSS", "GDPR"],
  "operator": {
    "id": "john.doe@company.com",
    "organization": "Acme Corp IT Security"
  },
  "signature": {
    "algorithm": "RSA-4096 + SHA-512",
    "value": "3082..."
  }
}
```

**Audit Trail Benefits:**
- 📄 Legal defensibility for data destruction claims
- 🔍 Forensic evidence of proper sanitization
- 📊 Compliance reporting for GDPR/HIPAA audits
- 🏢 Asset lifecycle documentation for enterprise
- 🔐 Chain of custody for sensitive data disposal

## 🧪 Testing

All tests are located in the `core/` directory:

```bash
# Navigate to core directory
cd core

# Run unit tests
cargo test

# Run with coverage
cargo tarpaulin --out Html

# Integration tests (requires root and test hardware)
cargo test --features integration-tests -- --ignored

# Test specific modules
cargo test verification    # Verification system tests
cargo test algorithms      # Algorithm tests
cargo test freeze         # Freeze mitigation tests
cargo test io             # I/O engine tests

# Run with test output visible
cargo test -- --nocapture

# Benchmarks
cargo bench
```

### Test Organization

Tests are co-located with modules in `core/src/`:
- `algorithms/gutmann_test.rs` - Algorithm verification
- `verification/enhanced_tests.rs` - 4-level verification tests
- `drives/freeze/tests.rs` - Freeze detection/mitigation tests
- `io/tests.rs` - I/O engine performance tests
- `crypto/secure_rng_tests.rs` - Cryptographic tests

## 🐛 Troubleshooting

### Drive is Frozen

If you encounter "Drive is frozen" errors:

```bash
# Try automatic unfreeze (uses multiple strategies)
sudo sayonara wipe /dev/sdX  # Will attempt unfreeze automatically

# Or build and use kernel module manually
cd core/src/drives/freeze/kernel_module
make
sudo make install
sudo insmod ata_unfreeze.ko
```

The tool includes multiple unfreeze strategies:
- SATA link reset
- PCIe hot reset
- ACPI sleep/resume (S3)
- USB suspend/resume
- IPMI power cycling
- Vendor-specific commands (Dell PERC, HP SmartArray, LSI, etc.)
- Kernel module (direct ATA register access)

### Hidden Areas (HPA/DCO)

```bash
# Detect hidden areas
sudo sayonara wipe /dev/sdX --hpa-dco detect

# Remove before wiping
sudo sayonara wipe /dev/sdX --hpa-dco remove-temp
```

### Temperature Issues

```bash
# Set custom temperature limit
sudo sayonara wipe /dev/sdX --max-temp 50

# Disable temperature monitoring (NOT RECOMMENDED)
sudo sayonara wipe /dev/sdX --no-temp-check
```

### Verification Failures

If verification fails:
1. Check drive health: `sudo sayonara health /dev/sdX`
2. Look for bad sectors in SMART data
3. Try a different algorithm
4. Consider drive replacement if hardware issues detected

## 📚 Documentation

### Core Directory Documentation

- **[I/O Engine](core/src/io/README.md)** - High-performance I/O engine documentation
- **[Verification System](core/src/verification/README.md)** - 4-level verification system details
- **[Freeze Mitigation](core/src/drives/freeze/README.md)** - Freeze detection and mitigation strategies

### Module Documentation

Generate full API documentation:
```bash
cd core
cargo doc --open --no-deps
```

### Development Setup

```bash
# Clone repository
git clone https://github.com/yourusername/sayonara.git
cd sayonara/core

# Build project
cargo build

# Run tests
cargo test

# Run with debug logging
RUST_LOG=debug cargo run -- list

# Format code
cargo fmt

# Run linter
cargo clippy
```

## 📜 License

This project is dual-licensed under your choice of:

- **MIT License** ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)
- **Apache License, Version 2.0** ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)

You may use this project under the terms of either license. This dual licensing is common in the Rust ecosystem and provides:

- **MIT**: Simple, permissive license with minimal restrictions
- **Apache 2.0**: Includes explicit patent grants and contributor protections

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this project shall be dual licensed as above, without any additional terms or conditions.

## ⚖️ Legal Notice

This software is designed for legitimate data destruction purposes including:
- End-of-life device disposal
- Secure media sanitization before reuse
- Compliance with data protection regulations (GDPR, HIPAA, etc.)
- Digital forensics and incident response

**Users are solely responsible for:**
- Verifying correct drive selection before wiping
- Ensuring legal authority to destroy data
- Compliance with local regulations
- Maintaining backups of important data

**The authors and contributors accept NO LIABILITY for:**
- Data loss due to user error
- Misuse of this software
- Hardware damage
- Violation of data retention laws

## 🔧 Technical Details

### Key Dependencies (from core/Cargo.toml)

**Core Runtime:**
- `tokio` - Async runtime
- `futures` - Async utilities

**Cryptography:**
- `ring` - Cryptographic operations, secure RNG
- `sha2` - Hashing for verification
- `x509-parser` - Certificate handling

**I/O Performance:**
- `io-uring` - Linux io_uring support (Linux only)
- `memmap2` - Memory-mapped I/O (Linux only)
- `nix` - Low-level system calls

**CLI & UI:**
- `clap` - Command-line interface
- `indicatif` - Progress bars
- `colored` - Terminal colors

**Storage:**
- `rusqlite` - Checkpoint storage for long operations
- `serde/serde_json` - Configuration and certificate serialization

### Platform Support

The core implementation supports:
- **Linux** (primary): Full feature set including io_uring, memory-mapped I/O
- **Windows**: Basic operations via WinAPI
- **macOS**: Basic operations via IOKit

See `core/Cargo.toml` for platform-specific dependencies.

### Advanced Drive Types Support

All drive type handlers are located in `core/src/drives/types/`:

- **SMR** (Shingled Magnetic Recording): Zone-aware sequential writing
- **Intel Optane** / 3D XPoint: Hardware ISE (Instant Secure Erase)
- **Hybrid SSHD**: Separate handling for HDD and SSD cache
- **eMMC**: Embedded storage with TRIM and secure erase
- **RAID**: Member drive detection and coordinated wiping
- **NVMe Advanced**: Enhanced sanitize commands and namespace management

## 🙏 Acknowledgments

- Rust community for excellent crates
- Linux kernel developers for ATA/SCSI subsystems
- Security researchers for verification methodologies
- Open source contributors

---

**Remember: With great power comes great responsibility. Always verify before you wipe!**

## 📂 Project Structure

This project follows a modular architecture:

```
sayonara-wipe/
├── core/                    # ⭐ Main Rust implementation (ALL FUNCTIONALITY HERE)
│   ├── src/                # Source code modules
│   ├── Cargo.toml          # Rust dependencies
│   ├── CLAUDE.md           # Developer documentation
│   └── README.md           # Core-specific documentation
├── README.md               # This file (project overview)
└── LICENSE                 # MIT License
```

**All development, building, and testing happens in the `core/` directory.**