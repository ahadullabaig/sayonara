# Testing Guide for Sayonara Wipe

This document provides comprehensive guidance for testing the Sayonara Wipe project, including running tests, adding new tests, generating coverage reports, and using mock infrastructure.

---

## Table of Contents

- [Quick Start](#quick-start)
- [Test Organization](#test-organization)
- [Running Tests](#running-tests)
- [Writing Tests](#writing-tests)
- [Coverage Reports](#coverage-reports)
- [Mock Infrastructure](#mock-infrastructure)
- [Test-Driven Development](#test-driven-development)
- [CI/CD Integration](#cicd-integration)
- [Troubleshooting](#troubleshooting)

---

## Quick Start

```bash
# Run all tests
cargo test

# Run with output visible
cargo test -- --nocapture

# Run only unit tests
cargo test --lib

# Run only integration tests
cargo test --test '*'

# Run a specific test
cargo test test_name

# Check code coverage (requires tarpaulin)
cargo tarpaulin --out Html
```

---

## Test Organization

### Unit Tests

Unit tests are co-located with the code they test using the `#[cfg(test)]` attribute:

```
src/
├── algorithms/
│   ├── gutmann.rs
│   └── gutmann_test.rs              # Unit tests for Gutmann
├── crypto/
│   └── secure_rng_tests.rs          # Unit tests for SecureRNG
├── verification/
│   ├── enhanced.rs
│   └── enhanced_tests.rs            # Unit tests for verification
└── ...
```

**Location:** `src/**/`
**Naming:** `<module>_test.rs` or `#[cfg(test)] mod tests` in same file
**Purpose:** Test individual functions and methods in isolation

### Integration Tests

Integration tests are in the `tests/` directory and test the public API:

```
tests/
├── basic_wipe.rs          # Integration tests for basic wipe operations
├── common/                # Shared test utilities
│   ├── mod.rs
│   ├── mock_drive.rs      # Mock drive infrastructure
│   └── test_helpers.rs    # Helper functions
└── ...
```

**Location:** `tests/`
**Naming:** Descriptive names like `basic_wipe.rs`, `concurrent_operations.rs`
**Purpose:** Test end-to-end flows and public API contracts

---

## Running Tests

### Basic Commands

```bash
# All tests (unit + integration)
cargo test

# Unit tests only
cargo test --lib

# Integration tests only
cargo test --test '*'

# Specific integration test
cargo test --test basic_wipe

# Specific test by name
cargo test test_gutmann_patterns

# Show test output (println! etc.)
cargo test -- --nocapture

# Run tests in sequence (not parallel)
cargo test -- --test-threads=1

# List all tests without running
cargo test -- --list

# Run ignored tests
cargo test -- --ignored

# Run tests matching pattern
cargo test checkpoint
```

### Running Specific Modules

```bash
# Run all tests in a module
cargo test algorithms::gutmann

# Run all tests in verification module
cargo test verification

# Run all freeze mitigation tests
cargo test drives::freeze
```

### Performance Testing

```bash
# Run benchmarks
cargo bench

# Run specific benchmark
cargo bench entropy_calculation
```

---

## Writing Tests

### Unit Test Example

```rust
// src/algorithms/my_algorithm.rs

pub fn calculate_checksum(data: &[u8]) -> u32 {
    data.iter().map(|&b| b as u32).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_checksum() {
        let data = vec![1, 2, 3, 4, 5];
        let checksum = calculate_checksum(&data);
        assert_eq!(checksum, 15);
    }

    #[test]
    fn test_calculate_checksum_empty() {
        let data = vec![];
        let checksum = calculate_checksum(&data);
        assert_eq!(checksum, 0);
    }

    #[test]
    #[should_panic(expected = "overflow")]
    fn test_calculate_checksum_overflow() {
        let data = vec![u8::MAX; 1000];
        calculate_checksum(&data);  // Should panic
    }
}
```

### Integration Test Example

```rust
// tests/my_integration_test.rs

use sayonara_wipe::io::{OptimizedIO, IOConfig};

#[path = "common/mod.rs"]
mod common;

use common::mock_drive::MockDrive;

#[test]
fn test_end_to_end_wipe() {
    // Create mock drive
    let mock = MockDrive::create_hdd(10).unwrap();

    // Configure I/O
    let mut config = IOConfig::default();
    config.use_direct_io = false;

    // Perform operation
    let mut handle = OptimizedIO::open(mock.path_str(), config).unwrap();

    // ... test logic ...

    // Verify results
    assert!(some_condition);
}
```

### Test Best Practices

1. **Arrange-Act-Assert Pattern**
   ```rust
   #[test]
   fn test_example() {
       // Arrange: Set up test data
       let input = vec![1, 2, 3];

       // Act: Execute the operation
       let result = function_under_test(&input);

       // Assert: Verify the outcome
       assert_eq!(result, expected_value);
   }
   ```

2. **Use Descriptive Names**
   ```rust
   // Good
   #[test]
   fn test_checkpoint_saves_current_progress()

   // Bad
   #[test]
   fn test1()
   ```

3. **Test One Thing Per Test**
   ```rust
   // Good: Each test focuses on one aspect
   #[test]
   fn test_save_checkpoint_creates_file()

   #[test]
   fn test_save_checkpoint_contains_correct_data()

   // Bad: Testing multiple things
   #[test]
   fn test_checkpoint()  // Too broad
   ```

4. **Use Test Fixtures and Helpers**
   ```rust
   fn create_test_config() -> WipeConfig {
       WipeConfig {
           algorithm: Algorithm::Zero,
           ..Default::default()
       }
   }

   #[test]
   fn test_something() {
       let config = create_test_config();
       // Use config...
   }
   ```

---

## Coverage Reports

### Installing cargo-tarpaulin

**Prerequisites:**
```bash
# Ubuntu/Debian
sudo apt-get install libssl-dev pkg-config

# Fedora
sudo dnf install openssl-devel

# macOS
brew install openssl
```

**Install:**
```bash
cargo install cargo-tarpaulin
```

### Generating Coverage

```bash
# HTML report (recommended)
cargo tarpaulin --out Html --output-dir coverage/

# Open report
firefox coverage/index.html  # or your browser

# XML for CI/CD
cargo tarpaulin --out Xml

# Console output
cargo tarpaulin

# Exclude files from coverage
cargo tarpaulin --exclude-files 'target/*' --exclude-files '*test*'

# Coverage for specific package
cargo tarpaulin --package sayonara-wipe
```

### Coverage Targets

| Module | Target | Priority |
|--------|--------|----------|
| algorithms/ | 95%+ | High |
| verification/ | 95%+ | Critical |
| drives/operations/ | 90%+ | High |
| crypto/ | 95%+ | Critical |
| error/ | 90%+ | High |
| io/ | 90%+ | Medium |

**Overall Target:** 90%+ line coverage

---

## Mock Infrastructure

### Using Mock Drives

The `tests/common/mock_drive.rs` module provides mock drives for testing:

```rust
use common::mock_drive::MockDrive;

// Create a mock HDD (100MB default)
let mock = MockDrive::create_hdd(100).unwrap();

// Create a mock SSD
let mock = MockDrive::create_ssd(50).unwrap();

// Create a mock NVMe
let mock = MockDrive::create_nvme(200).unwrap();

// Get the file path
let path = mock.path_str();

// Get the size in bytes
let size = mock.size_bytes();

// The mock drive is automatically cleaned up when dropped
```

### Custom Mock Configuration

```rust
use common::mock_drive::{MockDrive, MockDriveConfig, MockDriveType};

let config = MockDriveConfig {
    drive_type: MockDriveType::SMR,
    size_mb: 500,
    sector_size: 4096,
    simulate_errors: true,
    freeze_state: false,
};

let mock = MockDrive::new(config).unwrap();
```

### Test Helper Functions

```rust
use common::test_helpers::{verify_all_zeros, verify_pattern, calculate_file_entropy};

// Verify a file is all zeros
assert!(verify_all_zeros(path).unwrap());

// Verify a file contains a specific pattern
let pattern = [0xAA, 0x55];
assert!(verify_pattern(path, &pattern).unwrap());

// Calculate entropy
let entropy = calculate_file_entropy(path).unwrap();
assert!(entropy > 7.5);  // High entropy indicates randomness
```

### Loopback Devices (Linux Only)

For more realistic testing with actual block devices:

```rust
#[cfg(target_os = "linux")]
use common::mock_drive::loopback::{create_loopback, detach_loopback};

#[test]
#[ignore]  // Requires root
fn test_with_loopback() {
    // Create loopback device (requires sudo)
    let loop_dev = create_loopback("/tmp/test.img", 100).unwrap();

    // ... perform tests with loop_dev ...

    // Clean up
    detach_loopback(&loop_dev).unwrap();
}
```

Run with: `sudo cargo test -- --ignored`

---

## Test-Driven Development

### TDD Workflow

1. **Red:** Write a failing test
   ```rust
   #[test]
   fn test_new_feature() {
       let result = new_feature();
       assert_eq!(result, expected);
   }
   ```
   Result: `error[E0425]: cannot find function 'new_feature'`

2. **Green:** Write minimum code to pass
   ```rust
   pub fn new_feature() -> ExpectedType {
       expected  // Hardcoded return
   }
   ```
   Result: `test test_new_feature ... ok`

3. **Refactor:** Improve implementation
   ```rust
   pub fn new_feature() -> ExpectedType {
       // Proper implementation
       calculate_result()
   }
   ```
   Result: `test test_new_feature ... ok`

### Example TDD Session

```bash
# 1. Write test
$ cargo test new_feature
   ...
   error[E0425]: cannot find function `new_feature`

# 2. Add function stub
$ cargo test new_feature
   ...
   test algorithms::test_new_feature ... FAILED

# 3. Implement feature
$ cargo test new_feature
   ...
   test algorithms::test_new_feature ... ok

# 4. Add more tests for edge cases
$ cargo test new_feature
   ...
   test algorithms::test_new_feature_edge_case_1 ... ok
   test algorithms::test_new_feature_edge_case_2 ... ok
```

---

## CI/CD Integration

Sayonara Wipe has a comprehensive CI/CD pipeline that automatically tests, validates, and publishes code changes.

### Overview

Our CI/CD system consists of 5 automated workflows:

1. **Continuous Integration (ci.yml)** - Build, test, lint, format check
2. **Code Coverage (coverage.yml)** - Coverage tracking with Codecov
3. **Performance Benchmarks (benchmark.yml)** - Performance regression detection
4. **Security Audit (security.yml)** - Vulnerability and license compliance scanning
5. **Docker Build (docker.yml)** - Docker image building and publishing

All workflows run automatically on push/PR and can be triggered manually via GitHub Actions UI.

### Running CI Tests Locally

Before pushing code, run the same checks that CI runs:

```bash
# Navigate to core directory
cd core

# Full CI simulation (recommended before PR)
./scripts/ci_test.sh

# Quick checks (during development)
./scripts/ci_test.sh --fast

# Specific checks only
./scripts/ci_test.sh --skip-tests  # Build and lint only
./scripts/ci_test.sh --skip-build  # Tests and lint only
```

The `ci_test.sh` script runs:
- ✓ Build (debug + release)
- ✓ All tests (~689 tests)
- ✓ Format check (`cargo fmt`)
- ✓ Linting (`cargo clippy`)
- ✓ Security audit (if tools installed)
- ✓ Coverage report (unless --fast)
- ✓ Benchmarks (unless --fast)

### CI Workflow Details

#### 1. Continuous Integration (ci.yml)

**Triggers:** Push to main/ahad, Pull Requests
**Duration:** ~5-10 minutes (with caching)

**Jobs:**
- **build** - Compile debug and release binaries
- **test** - Run all ~689 tests (unit + integration + compliance)
- **format** - Check code formatting (`cargo fmt --check`)
- **lint** - Run Clippy with zero-warnings policy (`-D warnings`)
- **check** - Run `cargo check` for compilation warnings

**Status:** Required for PR merge

#### 2. Code Coverage (coverage.yml)

**Triggers:** Push to main/ahad, Pull Requests
**Duration:** ~10-15 minutes

**Process:**
1. Run `cargo-tarpaulin` with LLVM engine
2. Generate XML + HTML + Lcov reports
3. Upload to Codecov.io
4. Post PR comment with coverage diff
5. Archive reports as GitHub artifacts

**Current Coverage:** ~26-27% (8,461 test lines / 24,349 production lines) → Target: 90%
**Codecov Dashboard:** https://codecov.io/gh/TheShiveshNetwork/sayonara

**Note:** Some tests may have minor failures due to ongoing refactoring. Run `cargo test` to see current status.

**Status:** Informational (doesn't block PRs)

#### 3. Performance Benchmarks (benchmark.yml)

**Triggers:** Push to main, Pull Requests (optional)
**Duration:** ~15-20 minutes

**Process:**
1. Download baseline results (from main branch)
2. Run all 5 benchmark suites (throughput, latency, scaling, buffer_pool, adaptive_tuning)
3. Compare results against baseline using `check_performance_regression.sh`
4. Fail if any benchmark >10% slower
5. Upload results as artifacts
6. Post PR comment with benchmark summary

**Benchmark Suites:**
- `throughput` - Write throughput benchmarks
- `latency` - I/O latency measurements
- `scaling` - Concurrent drive scaling tests
- `buffer_pool` - Buffer management benchmarks
- `adaptive_tuning` - Adaptive performance tuning

**Status:** Informational on PRs, enforced on main

#### 4. Security Audit (security.yml)

**Triggers:** Push, Pull Requests, Weekly schedule (Mondays 9am UTC)
**Duration:** ~2-5 minutes

**Jobs:**
- **audit** - `cargo-audit` for dependency vulnerabilities (RUSTSEC)
- **deny** - `cargo-deny` for license compliance and dependency validation
- **create-issue** - Auto-create GitHub issue if vulnerabilities found

**Configuration:**
- Allowed licenses: MIT, Apache-2.0, BSD-2/3-Clause, ISC, Zlib
- Denied licenses: GPL, LGPL, AGPL, MPL
- See `deny.toml` for full config

**Status:** Required for PR merge

#### 5. Docker Build (docker.yml)

**Triggers:** Push to main, Pull Requests, Tags (v*.*.*)
**Duration:** ~10-15 minutes

**Jobs:**
- **build-test-image** - Build test environment (`docker/test.Dockerfile`)
- **build-prod-image** - Build production image (`Dockerfile`) with multi-stage build
- **scan** - Trivy vulnerability scanning
- **publish** - Push to GitHub Container Registry (ghcr.io)

**Images:**
- `ghcr.io/theshiveshnetwork/sayonara:latest` - Production (multi-stage, ~100MB)
- `ghcr.io/theshiveshnetwork/sayonara-test:latest` - Test environment

**Status:** Build required, publish only on main branch

### Pre-Commit Checklist

Before committing code, ensure:

```bash
# 1. Code is formatted
cargo fmt

# 2. No clippy warnings
cargo clippy --fix --all-targets --all-features

# 3. All tests pass
cargo test

# 4. No compilation warnings
cargo check --all-targets --all-features

# 5. Security audit passes (optional)
cargo audit
cargo deny check

# 6. Run full CI simulation
./scripts/ci_test.sh --fast
```

### CI/CD Scripts

The following scripts are available for local development in `core/scripts/`:

- **`install_dependencies.sh`** - Install system dependencies (smartmontools, hdparm, etc.)
- **`generate_coverage_report.sh`** - Generate coverage report locally
- **`check_performance_regression.sh`** - Check benchmark regressions
- **`setup_test_environment.sh`** - Create mock drives for testing (requires sudo)
- **`build_release.sh`** - Build optimized release binary
- **`ci_test.sh`** - Run full CI pipeline locally

**Usage:**
```bash
# Navigate to core directory first
cd core

# Install dependencies (one-time setup)
./scripts/install_dependencies.sh

# Generate coverage report
./scripts/generate_coverage_report.sh --html-only --open

# Build release binary
./scripts/build_release.sh --strip --check

# Run CI tests before pushing
./scripts/ci_test.sh --fast
```

### Continuous Deployment

**Automated Releases:**

When a tag is pushed (e.g., `v1.0.0`):
1. Docker images are built and published to ghcr.io
2. GitHub Release is created with release notes
3. Binaries are attached to the release (future)

**Manual Release Process:**
```bash
# 1. Update version in Cargo.toml
# 2. Create and push tag
git tag -a v1.0.0 -m "Release v1.0.0"
git push origin v1.0.0

# 3. GitHub Actions will automatically:
#    - Build Docker images
#    - Publish to ghcr.io
#    - Create GitHub Release
```

### Monitoring CI Health

**View Workflow Runs:**
- https://github.com/TheShiveshNetwork/sayonara/actions

**Check Coverage Trends:**
- https://codecov.io/gh/TheShiveshNetwork/sayonara

**Security Advisories:**
- GitHub Security tab
- Auto-created issues by security.yml workflow

### Troubleshooting CI Failures

**Build Failure:**
```bash
# Reproduce locally
cargo build --verbose

# Check for dependency issues
cargo update
cargo build
```

**Test Failure:**
```bash
# Reproduce exact test
cargo test test_name -- --nocapture

# Run with same environment as CI
export SAYONARA_TEST_MODE=1
cargo test
```

**Coverage Failure:**
```bash
# Generate coverage locally
./scripts/generate_coverage_report.sh
# View report: coverage/index.html
```

**Clippy Failure:**
```bash
# See all clippy warnings
cargo clippy --all-targets --all-features

# Auto-fix where possible
cargo clippy --fix --all-targets --all-features
```

**Security Audit Failure:**
```bash
# Check vulnerabilities
cargo audit

# Update dependencies
cargo update

# Check licenses
cargo deny check licenses
```

**Benchmark Regression:**
```bash
# Run benchmarks locally
cargo bench

# Check for regressions
./scripts/check_performance_regression.sh
```

### CI/CD Best Practices

1. **Always run `ci_test.sh --fast` before pushing**
2. **Fix clippy warnings immediately** (use `--fix` flag)
3. **Keep coverage above 80%** for new code
4. **Don't merge PRs with failing CI**
5. **Review security audit failures promptly**
6. **Monitor benchmark trends** to catch performance regressions early

---

## Troubleshooting

### Common Issues

#### 1. Tests Hanging

```bash
# Run with timeout
cargo test -- --test-threads=1 --nocapture

# Check for deadlocks or infinite loops in test code
```

#### 2. Flaky Tests

```bash
# Run multiple times to identify flakiness
for i in {1..100}; do cargo test test_name || break; done

# Common causes:
# - Race conditions
# - Time-dependent logic
# - External dependencies
# - Filesystem state
```

#### 3. Permission Denied

```bash
# Some tests require root
sudo cargo test

# Or mark tests as ignored
#[test]
#[ignore]
fn test_requires_root() { ... }

# Run ignored tests
sudo cargo test -- --ignored
```

#### 4. Test Compilation Errors

```bash
# Ensure test dependencies are in Cargo.toml [dev-dependencies]
# Not [dependencies]

[dev-dependencies]
tempfile = "3.8"
criterion = "0.5"
```

#### 5. Coverage Tool Fails

```bash
# Install required system packages
sudo apt-get install libssl-dev pkg-config

# Use alternative
rustup component add llvm-tools-preview
cargo install cargo-llvm-cov
cargo llvm-cov --html
```

---

## Additional Resources

- **Coverage Baseline:** See `coverage_baseline.md` for current test coverage metrics
- **Phase 1 Roadmap:** See `PHASE1_COMPLETION_ROADMAP.md` for testing targets
- **Code Documentation:** Run `cargo doc --open` for API docs
- **Rust Testing Book:** https://doc.rust-lang.org/book/ch11-00-testing.html
- **cargo-tarpaulin:** https://github.com/xd009642/tarpaulin

---

## Test Coverage Requirements

### For New Code

All new code must include:
1. ✅ Unit tests for all public functions
2. ✅ Edge case tests (empty input, max values, etc.)
3. ✅ Error handling tests
4. ✅ Integration tests for new features
5. ✅ Documentation with examples

### For Pull Requests

Before submitting a PR:
1. ✅ `cargo test` passes with 0 failures
2. ✅ `cargo fmt -- --check` passes
3. ✅ `cargo clippy -- -D warnings` passes
4. ✅ Test coverage doesn't decrease
5. ✅ All new code has tests

---

---

## Current Test Status

**As of Latest Update:**
- **Total Tests:** ~689 (688 passing, 1 failing)
- **Test Coverage:** 26-27% (8,461 test lines / 24,349 production lines)
- **Target Coverage:** 90%+
- **CI/CD Status:** ✅ Fully operational (7 workflows)
- **Known Issues:**
  - `test_suspicious_data_low_entropy` currently failing (verification module)
  - Some integration tests may require root privileges

**Test Breakdown:**
- Unit tests: ~580
- Integration tests: ~100
- Compliance tests: ~9
- Benchmark tests: Available via `cargo bench`

---

**Last Updated:** January 2025
**Maintained By:** Sayonara Wipe Team
