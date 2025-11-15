#!/usr/bin/env bash
#
# Run CI Tests Locally
#
# This script runs the same tests and checks that run in CI/CD, allowing developers
# to verify their changes before pushing to GitHub.
#
# Usage:
#   ./scripts/ci_test.sh [OPTIONS]
#
# Options:
#   --fast              Skip slow tests (coverage, benchmarks)
#   --skip-build        Skip build step
#   --skip-tests        Skip test execution
#   --skip-lint         Skip linting (clippy, fmt)
#   --skip-security     Skip security checks
#   --help              Show this help message
#
# Examples:
#   ./scripts/ci_test.sh              # Run all checks (full CI simulation)
#   ./scripts/ci_test.sh --fast       # Quick checks only
#   ./scripts/ci_test.sh --skip-tests # Build and lint only

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
MAGENTA='\033[0;35m'
NC='\033[0m' # No Color

# Default configuration
FAST_MODE=false
SKIP_BUILD=false
SKIP_TESTS=false
SKIP_LINT=false
SKIP_SECURITY=false

# Track results
FAILED_CHECKS=()
PASSED_CHECKS=()

# Parse command-line arguments
parse_args() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            --fast)
                FAST_MODE=true
                shift
                ;;
            --skip-build)
                SKIP_BUILD=true
                shift
                ;;
            --skip-tests)
                SKIP_TESTS=true
                shift
                ;;
            --skip-lint)
                SKIP_LINT=true
                shift
                ;;
            --skip-security)
                SKIP_SECURITY=true
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

Run CI tests locally before pushing to GitHub

OPTIONS:
    --fast              Skip slow tests (coverage, benchmarks)
    --skip-build        Skip build step
    --skip-tests        Skip test execution
    --skip-lint         Skip linting (clippy, fmt)
    --skip-security     Skip security checks
    --help              Show this help message

EXAMPLES:
    $0                   # Full CI simulation (recommended before PR)
    $0 --fast            # Quick checks (during development)
    $0 --skip-tests      # Build and lint only

CI CHECKS RUN:
    1. Build (debug + release)
    2. Tests (all 888 tests)
    3. Format check (cargo fmt)
    4. Linting (cargo clippy)
    5. Security audit (cargo audit + deny)
    6. Coverage report (if not --fast)
    7. Benchmarks (if not --fast)
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

log_check() {
    echo -e "${MAGENTA}[CHECK]${NC} $1"
}

# Record check result
record_result() {
    local check_name="$1"
    local exit_code="$2"

    if [ "$exit_code" -eq 0 ]; then
        PASSED_CHECKS+=("$check_name")
        log_info "✓ $check_name PASSED"
    else
        FAILED_CHECKS+=("$check_name")
        log_error "✗ $check_name FAILED"
    fi
}

# Run a check with error handling
run_check() {
    local check_name="$1"
    shift
    local command=("$@")

    log_check "Running: $check_name"
    echo ""

    if "${command[@]}"; then
        record_result "$check_name" 0
    else
        record_result "$check_name" 1
    fi

    echo ""
}

# Build project
check_build() {
    log_step "1. Build Verification"
    echo ""

    run_check "Debug Build" cargo build --verbose
    run_check "Release Build" cargo build --release --verbose
}

# Run tests
check_tests() {
    log_step "2. Test Execution"
    echo ""

    run_check "Unit Tests" cargo test --all-features --verbose
    run_check "Doc Tests" cargo test --doc --verbose
}

# Check code formatting
check_format() {
    log_step "3. Code Formatting Check"
    echo ""

    run_check "Format Check" cargo fmt --all -- --check
}

# Run linting
check_lint() {
    log_step "4. Linting (Clippy)"
    echo ""

    run_check "Clippy" cargo clippy --all-targets --all-features -- -D warnings
}

# Run security checks
check_security() {
    log_step "5. Security Checks"
    echo ""

    # Check if cargo-audit is installed
    if command -v cargo-audit &> /dev/null; then
        run_check "Vulnerability Scan" cargo audit
    else
        log_warn "cargo-audit not installed (skipping)"
        log_info "Install with: cargo install cargo-audit"
    fi

    # Check if cargo-deny is installed
    if command -v cargo-deny &> /dev/null; then
        run_check "License Check" cargo deny check licenses
        run_check "Advisory Check" cargo deny check advisories
    else
        log_warn "cargo-deny not installed (skipping)"
        log_info "Install with: cargo install cargo-deny"
    fi
}

# Generate coverage report
check_coverage() {
    log_step "6. Code Coverage"
    echo ""

    if command -v cargo-tarpaulin &> /dev/null; then
        run_check "Coverage Report" cargo tarpaulin --out Html --skip-clean
        log_info "Coverage report: coverage/index.html"
    else
        log_warn "cargo-tarpaulin not installed (skipping)"
        log_info "Install with: cargo install cargo-tarpaulin"
    fi
}

# Run benchmarks
check_benchmarks() {
    log_step "7. Performance Benchmarks"
    echo ""

    run_check "Benchmarks" cargo bench --no-run
}

# Display summary
display_summary() {
    echo ""
    echo "========================================"
    echo "        CI Test Summary"
    echo "========================================"
    echo ""

    if [ ${#PASSED_CHECKS[@]} -gt 0 ]; then
        echo -e "${GREEN}PASSED CHECKS (${#PASSED_CHECKS[@]}):${NC}"
        for check in "${PASSED_CHECKS[@]}"; do
            echo -e "  ${GREEN}✓${NC} $check"
        done
        echo ""
    fi

    if [ ${#FAILED_CHECKS[@]} -gt 0 ]; then
        echo -e "${RED}FAILED CHECKS (${#FAILED_CHECKS[@]}):${NC}"
        for check in "${FAILED_CHECKS[@]}"; do
            echo -e "  ${RED}✗${NC} $check"
        done
        echo ""
    fi

    echo "========================================"
    echo ""

    if [ ${#FAILED_CHECKS[@]} -eq 0 ]; then
        echo -e "${GREEN}✓ All checks passed!${NC}"
        echo ""
        echo "Your code is ready to push to GitHub."
        echo "CI pipeline should pass successfully."
        return 0
    else
        echo -e "${RED}✗ Some checks failed!${NC}"
        echo ""
        echo "Please fix the issues before pushing to GitHub."
        echo "CI pipeline will likely fail."
        return 1
    fi
}

# Main execution
main() {
    log_info "Sayonara Wipe - CI Test Runner"
    log_info "==============================="
    echo ""

    parse_args "$@"

    if [ "$FAST_MODE" = true ]; then
        log_info "Running in FAST mode (skipping coverage and benchmarks)"
        echo ""
    fi

    local start_time=$(date +%s)

    # Run checks
    if [ "$SKIP_BUILD" = false ]; then
        check_build
        echo ""
    fi

    if [ "$SKIP_TESTS" = false ]; then
        check_tests
        echo ""
    fi

    if [ "$SKIP_LINT" = false ]; then
        check_format
        echo ""
        check_lint
        echo ""
    fi

    if [ "$SKIP_SECURITY" = false ]; then
        check_security
        echo ""
    fi

    # Slow checks (skip in fast mode)
    if [ "$FAST_MODE" = false ]; then
        check_coverage
        echo ""
        check_benchmarks
        echo ""
    fi

    local end_time=$(date +%s)
    local total_time=$((end_time - start_time))

    echo ""
    log_info "Total execution time: ${total_time}s"
    echo ""

    if display_summary; then
        exit 0
    else
        exit 1
    fi
}

main "$@"
