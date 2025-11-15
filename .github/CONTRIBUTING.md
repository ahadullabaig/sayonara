# Contributing to Sayonara Wipe

Thank you for your interest in contributing to Sayonara Wipe! This document provides guidelines and instructions for contributing to the project.

---

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Workflow](#development-workflow)
- [Coding Standards](#coding-standards)
- [Testing Requirements](#testing-requirements)
- [Pull Request Process](#pull-request-process)
- [Commit Message Guidelines](#commit-message-guidelines)
- [Documentation](#documentation)
- [CI/CD Pipeline](#cicd-pipeline)
- [Getting Help](#getting-help)

---

## Code of Conduct

### Our Pledge

We are committed to providing a welcoming and inclusive experience for everyone. We expect all contributors to:

- Be respectful and considerate
- Use welcoming and inclusive language
- Accept constructive criticism gracefully
- Focus on what is best for the community
- Show empathy towards other community members

### Unacceptable Behavior

- Harassment, discrimination, or offensive comments
- Trolling, insulting/derogatory comments, and personal attacks
- Public or private harassment
- Publishing others' private information without permission
- Other conduct which could reasonably be considered inappropriate

---

## Getting Started

### Prerequisites

**Required:**
- Rust 1.70+ (stable toolchain)
- Git
- Linux, macOS, or Windows (Linux preferred for testing)

**System Dependencies (Linux):**
```bash
# Install dependencies
cd core
./scripts/install_dependencies.sh

# Or manually:
sudo apt-get install smartmontools hdparm nvme-cli libssl-dev pkg-config
```

**Rust Tools:**
```bash
# Install required components
rustup component add rustfmt clippy

# Install development tools (optional but recommended)
cargo install cargo-tarpaulin cargo-audit cargo-deny cargo-nextest
```

### Fork and Clone

1. Fork the repository on GitHub
2. Clone your fork locally:
   ```bash
   git clone https://github.com/YOUR_USERNAME/sayonara.git
   cd sayonara/core
   ```
3. Add upstream remote:
   ```bash
   git remote add upstream https://github.com/TheShiveshNetwork/sayonara.git
   ```
4. Create a feature branch:
   ```bash
   git checkout -b feature/your-feature-name
   ```

### Build and Test

```bash
# Build the project
cargo build

# Run tests
cargo test

# Run CI checks locally (recommended)
./scripts/ci_test.sh --fast
```

---

## Development Workflow

### 1. Sync with Upstream

Before starting work, sync with the latest upstream changes:

```bash
git fetch upstream
git checkout main
git merge upstream/main
git push origin main
```

### 2. Create a Feature Branch

```bash
git checkout -b feature/descriptive-name
# or
git checkout -b fix/bug-description
```

**Branch Naming Conventions:**
- `feature/` - New features
- `fix/` - Bug fixes
- `docs/` - Documentation only changes
- `refactor/` - Code refactoring
- `perf/` - Performance improvements
- `test/` - Adding or updating tests
- `ci/` - CI/CD changes

### 3. Make Changes

- Write clean, idiomatic Rust code
- Follow the coding standards (see below)
- Add tests for new functionality
- Update documentation as needed

### 4. Test Your Changes

```bash
# Run tests
cargo test

# Check formatting
cargo fmt --check

# Run clippy
cargo clippy --all-targets --all-features

# Run full CI simulation
./scripts/ci_test.sh --fast
```

### 5. Commit Your Changes

Follow commit message guidelines (see below):

```bash
git add .
git commit -m "feat: add support for ZNS drives"
```

### 6. Push and Create PR

```bash
git push origin feature/your-feature-name
```

Then create a Pull Request on GitHub.

---

## Coding Standards

### Rust Style Guide

We follow the official [Rust Style Guide](https://doc.rust-lang.org/beta/style-guide/).

**Key Requirements:**
- Use `rustfmt` for formatting (run `cargo fmt`)
- Pass `clippy` with zero warnings (`cargo clippy -- -D warnings`)
- No compilation warnings
- Use descriptive variable names
- Add doc comments for public APIs

### Code Quality

```rust
// Good: Descriptive names, documented
/// Detects if a drive is frozen and returns the freeze reason
pub fn detect_freeze_state(device: &str) -> Result<FreezeState, DriveError> {
    // Implementation
}

// Bad: Unclear names, no docs
pub fn detect(d: &str) -> Result<u8, E> {
    // Implementation
}
```

### Error Handling

- Use `Result<T, DriveError>` for all fallible operations
- Use `anyhow::Result` for application-level errors
- Provide descriptive error messages
- Use custom error types when appropriate

```rust
// Good
if !device.exists() {
    return Err(DriveError::DeviceNotFound {
        device: device.to_string(),
    });
}

// Bad
if !device.exists() {
    return Err(DriveError::Other("not found".into()));
}
```

### Safety

- Avoid `unsafe` code unless absolutely necessary
- Document all `unsafe` blocks with safety invariants
- Get approval from maintainers before adding `unsafe` code
- Validate all external input
- Check for buffer overflows
- Prevent command injection

### Performance

- Profile before optimizing
- Use benchmarks to validate improvements
- Avoid premature optimization
- Document performance-critical code

---

## Testing Requirements

### Test Coverage

- **New features:** Must have 80%+ test coverage
- **Bug fixes:** Must include regression test
- **Refactoring:** Maintain existing test coverage

### Test Types

**Unit Tests:**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_name() {
        // Arrange
        let input = setup_test_data();

        // Act
        let result = function_under_test(input);

        // Assert
        assert_eq!(result, expected_output);
    }
}
```

**Integration Tests:**
```rust
// core/tests/integration_feature.rs
use sayonara_wipe::*;

#[test]
fn test_end_to_end_workflow() -> Result<()> {
    // Test complete workflows
    Ok(())
}
```

**Compliance Tests:**
```rust
// core/tests/compliance/nist_800_88.rs
#[test]
fn test_nist_compliance() {
    // Verify compliance with standards
}
```

### Running Tests Locally

```bash
# All tests
cargo test

# Specific test
cargo test test_name

# With coverage
./scripts/generate_coverage_report.sh --html-only --open

# CI simulation
./scripts/ci_test.sh
```

---

## Pull Request Process

### Before Creating a PR

1. **Ensure CI passes locally:**
   ```bash
   ./scripts/ci_test.sh
   ```

2. **Update documentation** if needed

3. **Add/update tests** for your changes

4. **Rebase on latest main:**
   ```bash
   git fetch upstream
   git rebase upstream/main
   ```

### PR Checklist

- [ ] Code follows Rust style guide (`cargo fmt`)
- [ ] No clippy warnings (`cargo clippy -- -D warnings`)
- [ ] All tests pass (`cargo test`)
- [ ] New tests added for new features/fixes
- [ ] Documentation updated (if applicable)
- [ ] CI checks pass
- [ ] Commit messages follow conventions
- [ ] PR description is clear and detailed

### PR Title Format

Use conventional commits format:

```
<type>(<scope>): <description>

Examples:
feat(nvme): add ZNS namespace support
fix(verification): correct entropy calculation
docs(readme): update installation instructions
test(integration): add freeze mitigation tests
perf(io): optimize buffer allocation
```

### PR Description Template

```markdown
## Description
Brief description of changes

## Motivation
Why are these changes needed?

## Changes
- Change 1
- Change 2
- Change 3

## Testing
How were these changes tested?

## Checklist
- [ ] Tests added/updated
- [ ] Documentation updated
- [ ] CI passes locally
- [ ] No breaking changes (or documented)
```

### Review Process

1. **Automated checks** run (CI, coverage, benchmarks, security)
2. **Code review** by maintainers
3. **Discussion** and requested changes
4. **Approval** and merge

**Timeline:**
- Initial review: 2-5 days
- Follow-up reviews: 1-3 days

---

## Commit Message Guidelines

We use [Conventional Commits](https://www.conventionalcommits.org/):

### Format

```
<type>(<scope>): <description>

[optional body]

[optional footer]
```

### Types

- **feat:** New feature
- **fix:** Bug fix
- **docs:** Documentation changes
- **style:** Formatting (no code change)
- **refactor:** Code restructuring
- **perf:** Performance improvements
- **test:** Adding/updating tests
- **ci:** CI/CD changes
- **chore:** Maintenance tasks
- **revert:** Revert previous commit

### Examples

```bash
# Feature
git commit -m "feat(nvme): add support for ZNS sanitize command"

# Bug fix
git commit -m "fix(verify): correct Shannon entropy calculation edge case"

# Documentation
git commit -m "docs(contributing): add testing guidelines"

# With body
git commit -m "feat(freeze): add ACPI S3 sleep strategy

Implements ACPI S3 sleep/resume cycle to unfreeze drives.
Tested on Dell PowerEdge and HP ProLiant servers.

Closes #123"
```

---

## Documentation

### Code Documentation

- **Public APIs:** Must have doc comments with examples
- **Modules:** Include module-level docs
- **Complex logic:** Add inline comments
- **Safety invariants:** Document unsafe code

**Example:**
```rust
/// Detects the freeze state of a drive
///
/// # Arguments
/// * `device` - Path to the device (e.g., `/dev/sda`)
///
/// # Returns
/// * `Ok(FreezeState)` - Freeze state detected
/// * `Err(DriveError)` - Detection failed
///
/// # Example
/// ```
/// use sayonara_wipe::freeze::detect_freeze_state;
///
/// let state = detect_freeze_state("/dev/sda")?;
/// println!("Freeze state: {:?}", state);
/// ```
pub fn detect_freeze_state(device: &str) -> Result<FreezeState, DriveError> {
    // Implementation
}
```

### User Documentation

Update these files when relevant:
- `README.md` - Project overview
- `core/TESTING.md` - Testing guide
- `CONTRIBUTING.md` - This file
- `CLAUDE.md` - AI assistance guide

---

## CI/CD Pipeline

Our CI/CD pipeline runs automatically on all PRs:

### Automated Checks

1. **Build** - Compile debug and release
2. **Tests** - Run all 888 tests
3. **Format** - Check `cargo fmt`
4. **Lint** - Run `cargo clippy`
5. **Coverage** - Generate coverage report
6. **Benchmarks** - Check for performance regressions
7. **Security** - Vulnerability and license scanning
8. **Docker** - Build container images

### Local CI Simulation

```bash
# Full simulation (recommended before PR)
./scripts/ci_test.sh

# Quick checks (during development)
./scripts/ci_test.sh --fast

# Specific checks
./scripts/ci_test.sh --skip-tests
```

### CI Status Badges

Check CI status at: https://github.com/TheShiveshNetwork/sayonara/actions

---

## Getting Help

### Resources

- **Documentation:** `/core/README.md`, `/core/TESTING.md`
- **Examples:** `/core/examples/` directory
- **Tests:** `/core/tests/` for usage examples

### Contact

- **Issues:** [GitHub Issues](https://github.com/TheShiveshNetwork/sayonara/issues)
- **Discussions:** [GitHub Discussions](https://github.com/TheShiveshNetwork/sayonara/discussions)
- **Email:** maintainers@theshiveshnetwork.com (for security issues)

### Reporting Security Vulnerabilities

**DO NOT** open a public issue for security vulnerabilities.

Instead:
1. Email: security@theshiveshnetwork.com
2. Include:
   - Description of vulnerability
   - Steps to reproduce
   - Potential impact
   - Suggested fix (if any)

We will respond within 48 hours.

---

## License

By contributing to Sayonara Wipe, you agree that your contributions will be licensed under the same dual MIT/Apache-2.0 license as the project.

---

## Recognition

Contributors are recognized in:
- `CONTRIBUTORS.md` file
- GitHub contributors page
- Release notes (for significant contributions)

---

## Questions?

If you have questions not covered here:
1. Check existing issues/discussions
2. Create a new discussion on GitHub
3. Tag maintainers for urgent matters

**Thank you for contributing to Sayonara Wipe!**
