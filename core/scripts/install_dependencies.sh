#!/usr/bin/env bash
#
# Install System Dependencies for Sayonara Wipe
#
# This script installs all required system dependencies for building and testing
# the Sayonara secure data wiping tool.
#
# Supported platforms:
#   - Ubuntu/Debian (apt)
#   - macOS (brew)
#   - Fedora/RHEL (dnf/yum)
#
# Usage:
#   ./scripts/install_dependencies.sh

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

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

# Detect OS
detect_os() {
    if [[ "$OSTYPE" == "linux-gnu"* ]]; then
        if [ -f /etc/os-release ]; then
            . /etc/os-release
            OS=$ID
        else
            log_error "Cannot detect Linux distribution"
            exit 1
        fi
    elif [[ "$OSTYPE" == "darwin"* ]]; then
        OS="macos"
    else
        log_error "Unsupported OS: $OSTYPE"
        exit 1
    fi
}

# Install dependencies on Ubuntu/Debian
install_debian() {
    log_info "Installing dependencies for Debian/Ubuntu..."

    sudo apt-get update

    # Hardware utilities
    sudo apt-get install -y \
        smartmontools \
        hdparm \
        nvme-cli \
        sg3-utils

    # Build dependencies
    sudo apt-get install -y \
        libssl-dev \
        pkg-config \
        build-essential

    # Optional: Coverage and profiling tools
    if command -v cargo &> /dev/null; then
        log_info "Installing Rust tools..."
        cargo install cargo-tarpaulin --locked || log_warn "Failed to install cargo-tarpaulin"
    fi

    log_info "Dependencies installed successfully on Debian/Ubuntu"
}

# Install dependencies on Fedora/RHEL
install_fedora() {
    log_info "Installing dependencies for Fedora/RHEL..."

    local PKG_MANAGER="dnf"
    if ! command -v dnf &> /dev/null; then
        PKG_MANAGER="yum"
    fi

    # Hardware utilities
    sudo $PKG_MANAGER install -y \
        smartmontools \
        hdparm \
        nvme-cli \
        sg3_utils

    # Build dependencies
    sudo $PKG_MANAGER install -y \
        openssl-devel \
        pkg-config \
        gcc \
        gcc-c++

    # Optional: Coverage tools
    if command -v cargo &> /dev/null; then
        log_info "Installing Rust tools..."
        cargo install cargo-tarpaulin --locked || log_warn "Failed to install cargo-tarpaulin"
    fi

    log_info "Dependencies installed successfully on Fedora/RHEL"
}

# Install dependencies on macOS
install_macos() {
    log_info "Installing dependencies for macOS..."

    # Check if Homebrew is installed
    if ! command -v brew &> /dev/null; then
        log_error "Homebrew not found. Please install it from https://brew.sh"
        exit 1
    fi

    # Hardware utilities
    brew install smartmontools

    # Note: hdparm is Linux-only, nvme-cli available via brew
    log_warn "hdparm is not available on macOS (Linux-only tool)"

    # NVMe utilities (if available)
    brew install nvme-cli || log_warn "nvme-cli not available via Homebrew"

    # Build dependencies
    brew install openssl pkg-config

    log_info "Dependencies installed successfully on macOS"
    log_warn "Note: Some hardware-specific tools are Linux-only"
}

# Main installation logic
main() {
    log_info "Sayonara Wipe - Dependency Installer"
    log_info "====================================="
    echo ""

    detect_os
    log_info "Detected OS: $OS"
    echo ""

    case $OS in
        ubuntu|debian)
            install_debian
            ;;
        fedora|rhel|centos)
            install_fedora
            ;;
        macos)
            install_macos
            ;;
        *)
            log_error "Unsupported distribution: $OS"
            log_info "Please manually install: smartmontools, hdparm, nvme-cli, libssl-dev, pkg-config"
            exit 1
            ;;
    esac

    echo ""
    log_info "✓ All dependencies installed successfully"
    log_info ""
    log_info "Next steps:"
    log_info "  1. Build the project:  cargo build"
    log_info "  2. Run tests:          cargo test"
    log_info "  3. Run coverage:       ./scripts/generate_coverage_report.sh"
}

main "$@"
