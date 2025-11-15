# Sayonara Wipe - Production Docker Image
#
# Multi-stage build for optimal image size
#
# Usage:
#   docker build -t sayonara-wipe .
#   docker run --rm --privileged sayonara-wipe sayonara --help
#
# Notes:
#   - Requires --privileged flag for drive access
#   - Image size: ~100MB (multi-stage build)
#   - Based on Debian Bookworm (12)

# ============================================================================
# Stage 1: Builder
# ============================================================================
FROM rust:1.75-bookworm AS builder

LABEL maintainer="Sayonara Team <noreply@theshiveshnetwork.com>"
LABEL description="Build stage for Sayonara secure data wiping tool"

# Set working directory
WORKDIR /build

# Install build dependencies
RUN apt-get update && apt-get install -y \
    libssl-dev \
    pkg-config \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

# Copy dependency manifests first (for layer caching)
COPY core/Cargo.toml core/Cargo.lock ./

# Create dummy main.rs to build dependencies
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    echo "pub fn lib() {}" > src/lib.rs

# Build dependencies (cached layer)
RUN cargo build --release && \
    rm -rf src/

# Copy actual source code
COPY core/src ./src
COPY core/benches ./benches
COPY core/tests ./tests

# Build the actual project
RUN cargo build --release --bin sayonara

# Strip debug symbols to reduce binary size
RUN strip /build/target/release/sayonara

# Verify binary works
RUN /build/target/release/sayonara --version

# ============================================================================
# Stage 2: Runtime
# ============================================================================
FROM debian:bookworm-slim AS runtime

LABEL maintainer="Sayonara Team <noreply@theshiveshnetwork.com>"
LABEL description="Sayonara - Advanced secure data wiping tool"
LABEL version="1.0.0"
LABEL org.opencontainers.image.source="https://github.com/TheShiveshNetwork/sayonara"
LABEL org.opencontainers.image.licenses="MIT OR Apache-2.0"

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    # Hardware utilities
    smartmontools \
    hdparm \
    nvme-cli \
    # SSL certificates for HTTPS
    ca-certificates \
    # Minimal utilities
    util-linux \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user (though drive ops need root)
RUN useradd -m -u 1000 -s /bin/bash sayonara

# Copy binary from builder
COPY --from=builder /build/target/release/sayonara /usr/local/bin/sayonara

# Verify binary
RUN sayonara --version

# Set working directory
WORKDIR /workspace

# Default user (can be overridden with --user root for drive access)
USER sayonara

# Health check (verify binary is executable)
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=1 \
    CMD sayonara --version || exit 1

# Entrypoint
ENTRYPOINT ["/usr/local/bin/sayonara"]

# Default command (show help)
CMD ["--help"]

# ============================================================================
# Build Information
# ============================================================================
# Build command:
#   docker build -t ghcr.io/theshiveshnetwork/sayonara:latest .
#
# Run command:
#   docker run --rm --privileged ghcr.io/theshiveshnetwork/sayonara:latest list
#
# Interactive shell:
#   docker run --rm -it --privileged --entrypoint /bin/bash \
#     ghcr.io/theshiveshnetwork/sayonara:latest
#
# Image layers:
#   1. Builder stage: ~1.5GB (includes Rust toolchain)
#   2. Runtime stage: ~100MB (minimal Debian + binary + tools)
#   Final image: ~100MB
