# Sayonara Wipe - Test Environment Docker Image
#
# This image provides a complete testing environment for Sayonara,
# including all dependencies, test frameworks, and coverage tools.
#
# Usage:
#   docker build -f docker/test.Dockerfile -t sayonara-test .
#   docker run --rm sayonara-test cargo test
#
# Features:
#   - Rust toolchain with all components
#   - Hardware utilities (smartmontools, hdparm, nvme-cli)
#   - Test coverage tools (tarpaulin)
#   - Mock drive infrastructure
#   - Test execution environment

FROM rust:1.75-bookworm

LABEL maintainer="Sayonara Team <noreply@theshiveshnetwork.com>"
LABEL description="Test environment for Sayonara secure data wiping tool"

# Set environment variables
ENV RUST_BACKTRACE=1 \
    CARGO_TERM_COLOR=always \
    SAYONARA_TEST_MODE=1

# Set working directory
WORKDIR /workspace

# Install system dependencies
RUN apt-get update && apt-get install -y \
    # Hardware utilities
    smartmontools \
    hdparm \
    nvme-cli \
    sg3-utils \
    # Build dependencies
    libssl-dev \
    pkg-config \
    build-essential \
    # Test utilities
    jq \
    bc \
    # Git (for version info)
    git \
    # Debugging tools
    gdb \
    valgrind \
    strace \
    # Documentation tools
    graphviz \
    && rm -rf /var/lib/apt/lists/*

# Install Rust components
RUN rustup component add \
    rustfmt \
    clippy \
    llvm-tools-preview

# Install Rust testing tools
RUN cargo install \
    cargo-tarpaulin \
    cargo-audit \
    cargo-deny \
    cargo-nextest \
    cargo-watch \
    --locked

# Copy project files
COPY core/Cargo.toml core/Cargo.lock ./
COPY core/src ./src
COPY core/tests ./tests
COPY core/benches ./benches
COPY core/scripts ./scripts
COPY core/.tarpaulin.toml ./.tarpaulin.toml
COPY core/deny.toml ./deny.toml

# Build dependencies (for caching)
RUN cargo fetch

# Build project in debug mode
RUN cargo build --all-features

# Make scripts executable
RUN chmod +x scripts/*.sh || true

# Create directories for test artifacts
RUN mkdir -p \
    /workspace/coverage \
    /workspace/test-results \
    /workspace/benchmarks

# Health check (verify Rust toolchain)
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=1 \
    CMD cargo --version || exit 1

# Default command: run all tests
CMD ["cargo", "test", "--all-features", "--verbose"]

# ============================================================================
# Usage Examples
# ============================================================================
#
# Build:
#   docker build -f docker/test.Dockerfile -t sayonara-test .
#
# Run tests:
#   docker run --rm sayonara-test cargo test
#
# Run specific test:
#   docker run --rm sayonara-test cargo test test_checkpoint_creation
#
# Run with coverage:
#   docker run --rm -v $(pwd)/coverage:/workspace/coverage \
#     sayonara-test cargo tarpaulin --out Html
#
# Run benchmarks:
#   docker run --rm sayonara-test cargo bench
#
# Interactive shell:
#   docker run --rm -it --entrypoint /bin/bash sayonara-test
#
# Run formatting check:
#   docker run --rm sayonara-test cargo fmt -- --check
#
# Run clippy:
#   docker run --rm sayonara-test cargo clippy -- -D warnings
#
# Run security audit:
#   docker run --rm sayonara-test cargo audit
#
# ============================================================================
# CI/CD Integration
# ============================================================================
#
# This image is designed to be used in CI/CD pipelines:
#
# GitHub Actions example:
#   - name: Run tests in Docker
#     run: |
#       docker build -f docker/test.Dockerfile -t sayonara-test .
#       docker run --rm sayonara-test cargo test
#
# GitLab CI example:
#   test:
#     image: ghcr.io/theshiveshnetwork/sayonara-test:latest
#     script:
#       - cargo test --all-features
