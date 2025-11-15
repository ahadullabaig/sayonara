#!/usr/bin/env bash
#
# Build Optimized Release Binary for Sayonara Wipe
#
# This script builds a highly optimized release binary with maximum performance
# and minimal size. Includes symbol stripping and verification.
#
# Usage:
#   ./scripts/build_release.sh [OPTIONS]
#
# Options:
#   --target <triple>   Target platform (default: host platform)
#   --strip             Strip debug symbols (recommended for production)
#   --check             Verify binary after build
#   --install           Install to /usr/local/bin (requires sudo)
#   --help              Show this help message
#
# Examples:
#   ./scripts/build_release.sh
#   ./scripts/build_release.sh --strip --check
#   ./scripts/build_release.sh --target x86_64-unknown-linux-musl
#   sudo ./scripts/build_release.sh --strip --install

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default configuration
TARGET_PLATFORM=""
STRIP_BINARY=false
CHECK_BINARY=false
INSTALL_BINARY=false

# Parse command-line arguments
parse_args() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            --target)
                TARGET_PLATFORM="$2"
                shift 2
                ;;
            --strip)
                STRIP_BINARY=true
                shift
                ;;
            --check)
                CHECK_BINARY=true
                shift
                ;;
            --install)
                INSTALL_BINARY=true
                shift
                ;;
            --help)
                show_help
                exit 0
                ;;
            *)
                echo -e "${RED}Error: Unknown option: $1${NC}"
                show_help
                exit 1
                ;;
        esac
    done
}

# Show help message
show_help() {
    cat << EOF
Usage: $0 [OPTIONS]

Build optimized release binary for Sayonara Wipe

OPTIONS:
    --target <triple>   Target platform (default: host platform)
    --strip             Strip debug symbols (recommended for production)
    --check             Verify binary after build
    --install           Install to /usr/local/bin (requires sudo)
    --help              Show this help message

EXAMPLES:
    $0                                  # Build for host platform
    $0 --strip --check                  # Build, strip, and verify
    $0 --target x86_64-unknown-linux-musl --strip   # Cross-compile for Linux musl
    sudo $0 --strip --install           # Build and install system-wide

COMMON TARGETS:
    x86_64-unknown-linux-gnu           # Linux x86_64 (glibc)
    x86_64-unknown-linux-musl          # Linux x86_64 (musl, static)
    x86_64-apple-darwin                # macOS x86_64
    aarch64-apple-darwin               # macOS ARM64 (M1/M2)
    x86_64-pc-windows-msvc             # Windows x86_64

OPTIMIZATIONS APPLIED:
    - opt-level = 3 (maximum optimization)
    - lto = true (link-time optimization)
    - codegen-units = 1 (better optimization)
    - strip = true (remove debug symbols)
    - panic = "abort" (smaller binary)
EOF
}

# Print colored message
log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

log_step() {
    echo -e "${BLUE}[STEP]${NC} $1"
}

# Display build configuration
display_config() {
    log_step "Build Configuration"
    echo ""
    echo "Rust Version:    $(rustc --version)"
    echo "Cargo Version:   $(cargo --version)"
    echo "Target Platform: ${TARGET_PLATFORM:-<host>}"
    echo "Strip Symbols:   $STRIP_BINARY"
    echo "Verify Binary:   $CHECK_BINARY"
    echo "Install Binary:  $INSTALL_BINARY"
    echo ""
}

# Build release binary
build_release() {
    log_step "Building release binary..."

    local BUILD_CMD="cargo build --release --bin sayonara"

    if [ -n "$TARGET_PLATFORM" ]; then
        BUILD_CMD="$BUILD_CMD --target $TARGET_PLATFORM"
        log_info "Cross-compiling for: $TARGET_PLATFORM"

        # Check if target is installed
        if ! rustup target list --installed | grep -q "$TARGET_PLATFORM"; then
            log_warn "Target not installed. Installing: $TARGET_PLATFORM"
            rustup target add "$TARGET_PLATFORM"
        fi
    fi

    log_info "Running: $BUILD_CMD"
    echo ""

    # Build with timing
    local start_time=$(date +%s)
    eval "$BUILD_CMD"
    local end_time=$(date +%s)
    local build_time=$((end_time - start_time))

    log_info "✓ Build completed in ${build_time}s"
}

# Get binary path
get_binary_path() {
    if [ -n "$TARGET_PLATFORM" ]; then
        echo "target/$TARGET_PLATFORM/release/sayonara"
    else
        echo "target/release/sayonara"
    fi
}

# Strip debug symbols
strip_symbols() {
    local binary_path=$(get_binary_path)

    log_step "Stripping debug symbols..."

    if [ ! -f "$binary_path" ]; then
        log_error "Binary not found: $binary_path"
        exit 1
    fi

    local before_size=$(ls -lh "$binary_path" | awk '{print $5}')

    strip "$binary_path"

    local after_size=$(ls -lh "$binary_path" | awk '{print $5}')

    log_info "✓ Symbols stripped"
    log_info "  Before: $before_size"
    log_info "  After:  $after_size"
}

# Verify binary
verify_binary() {
    local binary_path=$(get_binary_path)

    log_step "Verifying binary..."

    if [ ! -f "$binary_path" ]; then
        log_error "Binary not found: $binary_path"
        exit 1
    fi

    # Check if binary is executable
    if [ ! -x "$binary_path" ]; then
        log_error "Binary is not executable"
        exit 1
    fi

    # Run version check
    if "$binary_path" --version >/dev/null 2>&1; then
        log_info "✓ Binary verification passed"
        "$binary_path" --version
    else
        log_error "Binary verification failed"
        exit 1
    fi

    # Display binary info
    echo ""
    log_info "Binary Information:"
    echo "  Path:    $binary_path"
    echo "  Size:    $(ls -lh "$binary_path" | awk '{print $5}')"
    echo "  Type:    $(file "$binary_path" | cut -d: -f2)"

    # Check for debug symbols
    if file "$binary_path" | grep -q "not stripped"; then
        log_warn "Binary contains debug symbols (use --strip to remove)"
    else
        log_info "Binary is stripped (optimized)"
    fi
}

# Install binary
install_binary() {
    local binary_path=$(get_binary_path)
    local install_path="/usr/local/bin/sayonara"

    log_step "Installing binary..."

    if [ "$EUID" -ne 0 ]; then
        log_error "Installation requires root privileges (use sudo)"
        exit 1
    fi

    if [ ! -f "$binary_path" ]; then
        log_error "Binary not found: $binary_path"
        exit 1
    fi

    # Backup existing binary if it exists
    if [ -f "$install_path" ]; then
        log_warn "Backing up existing binary to $install_path.bak"
        cp "$install_path" "$install_path.bak"
    fi

    # Install binary
    cp "$binary_path" "$install_path"
    chmod 755 "$install_path"

    log_info "✓ Binary installed to: $install_path"

    # Verify installation
    if sayonara --version >/dev/null 2>&1; then
        log_info "✓ Installation verified"
        sayonara --version
    else
        log_error "Installation verification failed"
        exit 1
    fi
}

# Display build summary
display_summary() {
    local binary_path=$(get_binary_path)

    echo ""
    log_step "Build Summary"
    echo ""
    echo "✓ Release binary built successfully"
    echo ""
    echo "Binary Path:   $binary_path"
    echo "Binary Size:   $(ls -lh "$binary_path" | awk '{print $5}')"
    echo ""
    echo "To run:"
    echo "  ./$binary_path --help"
    echo ""
    if [ "$INSTALL_BINARY" = false ]; then
        echo "To install system-wide:"
        echo "  sudo ./scripts/build_release.sh --strip --install"
    fi
}

# Main execution
main() {
    log_info "Sayonara Wipe - Release Build Script"
    log_info "====================================="
    echo ""

    parse_args "$@"
    display_config

    build_release

    echo ""

    if [ "$STRIP_BINARY" = true ]; then
        strip_symbols
        echo ""
    fi

    if [ "$CHECK_BINARY" = true ]; then
        verify_binary
        echo ""
    fi

    if [ "$INSTALL_BINARY" = true ]; then
        install_binary
        echo ""
    fi

    display_summary
}

main "$@"
