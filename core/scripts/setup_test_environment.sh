#!/usr/bin/env bash
#
# Setup Test Environment for Sayonara Wipe
#
# This script prepares the test environment by creating mock drives,
# loopback devices, and other test infrastructure.
#
# Usage:
#   sudo ./scripts/setup_test_environment.sh [OPTIONS]
#
# Options:
#   --size <MB>         Size of mock drives in MB (default: 100)
#   --count <N>         Number of mock drives to create (default: 3)
#   --clean             Clean up existing test environment
#   --help              Show this help message
#
# Examples:
#   sudo ./scripts/setup_test_environment.sh
#   sudo ./scripts/setup_test_environment.sh --size 500 --count 5
#   sudo ./scripts/setup_test_environment.sh --clean

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default configuration
DRIVE_SIZE_MB=100
DRIVE_COUNT=3
CLEAN_MODE=false
TEST_DIR="/tmp/sayonara-test"

# Parse command-line arguments
parse_args() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            --size)
                DRIVE_SIZE_MB="$2"
                shift 2
                ;;
            --count)
                DRIVE_COUNT="$2"
                shift 2
                ;;
            --clean)
                CLEAN_MODE=true
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
Usage: sudo $0 [OPTIONS]

Setup test environment for Sayonara Wipe

OPTIONS:
    --size <MB>         Size of mock drives in MB (default: 100)
    --count <N>         Number of mock drives to create (default: 3)
    --clean             Clean up existing test environment
    --help              Show this help message

EXAMPLES:
    sudo $0                               # Create 3x 100MB mock drives
    sudo $0 --size 500 --count 5          # Create 5x 500MB mock drives
    sudo $0 --clean                       # Clean up test environment

REQUIREMENTS:
    - Root privileges (sudo)
    - losetup command
    - dd command

NOTES:
    - Mock drives are created as loopback devices
    - Files are stored in $TEST_DIR
    - Use --clean to remove test environment before re-creating
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

# Check if running as root
check_root() {
    if [ "$EUID" -ne 0 ]; then
        log_error "This script must be run as root (use sudo)"
        exit 1
    fi
}

# Clean up existing test environment
cleanup_test_env() {
    log_step "Cleaning up existing test environment..."

    # Detach loopback devices
    if [ -d "$TEST_DIR" ]; then
        for loop_device in $(losetup -j "$TEST_DIR" | cut -d: -f1); do
            log_info "Detaching $loop_device"
            losetup -d "$loop_device" 2>/dev/null || true
        done
    fi

    # Remove test directory
    if [ -d "$TEST_DIR" ]; then
        log_info "Removing $TEST_DIR"
        rm -rf "$TEST_DIR"
    fi

    log_info "✓ Cleanup complete"
}

# Create test directory
create_test_dir() {
    log_step "Creating test directory: $TEST_DIR"
    mkdir -p "$TEST_DIR"
    chmod 755 "$TEST_DIR"
    log_info "✓ Test directory created"
}

# Create mock drive file
create_mock_drive() {
    local drive_num=$1
    local drive_file="$TEST_DIR/mock_drive_${drive_num}.img"

    log_info "Creating mock drive $drive_num: ${DRIVE_SIZE_MB}MB"

    # Create sparse file (fast, doesn't allocate all space)
    dd if=/dev/zero of="$drive_file" bs=1M count=0 seek="$DRIVE_SIZE_MB" 2>/dev/null

    # Set up loopback device
    local loop_device=$(losetup -f)
    losetup "$loop_device" "$drive_file"

    log_info "✓ Mock drive $drive_num created: $loop_device -> $drive_file"
    echo "$loop_device"
}

# Display test environment info
display_info() {
    log_step "Test Environment Summary"
    echo ""
    echo "Test Directory: $TEST_DIR"
    echo "Drive Size:     ${DRIVE_SIZE_MB} MB"
    echo "Drive Count:    $DRIVE_COUNT"
    echo ""
    echo "Loopback Devices:"
    echo "=================="
    losetup -l | grep "$TEST_DIR" || echo "No loopback devices found"
    echo ""
    echo "Disk Usage:"
    echo "==========="
    du -sh "$TEST_DIR" 2>/dev/null || echo "Directory not found"
    echo ""
    log_info "Test environment ready"
    echo ""
    echo "To use in tests, set:"
    echo "  export SAYONARA_TEST_MODE=1"
    echo "  export SAYONARA_TEST_DRIVES=\"$(losetup -l | grep "$TEST_DIR" | cut -d' ' -f1 | tr '\n' ',' | sed 's/,$//')\""
}

# Main execution
main() {
    log_info "Sayonara Wipe - Test Environment Setup"
    log_info "======================================"
    echo ""

    parse_args "$@"
    check_root

    if [ "$CLEAN_MODE" = true ]; then
        cleanup_test_env
        exit 0
    fi

    # Clean up any existing environment first
    cleanup_test_env

    # Create new test environment
    create_test_dir

    log_step "Creating $DRIVE_COUNT mock drives..."
    echo ""

    # Create mock drives
    for i in $(seq 1 "$DRIVE_COUNT"); do
        create_mock_drive "$i"
    done

    echo ""
    display_info

    echo ""
    log_info "✓ Test environment setup complete"
    echo ""
    log_warn "Remember to run 'sudo ./scripts/setup_test_environment.sh --clean' when done"
}

main "$@"
