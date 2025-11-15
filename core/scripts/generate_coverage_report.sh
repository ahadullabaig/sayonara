#!/usr/bin/env bash
#
# Generate Code Coverage Report for Sayonara Wipe
#
# This script generates a comprehensive code coverage report using cargo-tarpaulin.
# It supports multiple output formats and can optionally upload results to Codecov.
#
# Usage:
#   ./scripts/generate_coverage_report.sh [OPTIONS]
#
# Options:
#   --upload        Upload coverage to Codecov (requires CODECOV_TOKEN)
#   --html-only     Generate only HTML report (faster, no XML)
#   --open          Open HTML report in browser after generation
#   --help          Show this help message
#
# Examples:
#   ./scripts/generate_coverage_report.sh                  # Generate reports locally
#   ./scripts/generate_coverage_report.sh --upload         # Generate and upload to Codecov
#   ./scripts/generate_coverage_report.sh --html-only --open  # Quick HTML report

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default options
UPLOAD_CODECOV=false
HTML_ONLY=false
OPEN_REPORT=false

# Parse command-line arguments
parse_args() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            --upload)
                UPLOAD_CODECOV=true
                shift
                ;;
            --html-only)
                HTML_ONLY=true
                shift
                ;;
            --open)
                OPEN_REPORT=true
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

Generate code coverage report for Sayonara Wipe

OPTIONS:
    --upload        Upload coverage to Codecov (requires CODECOV_TOKEN)
    --html-only     Generate only HTML report (faster, no XML)
    --open          Open HTML report in browser after generation
    --help          Show this help message

EXAMPLES:
    $0                                 # Generate reports locally
    $0 --upload                        # Generate and upload to Codecov
    $0 --html-only --open              # Quick HTML report and open in browser

ENVIRONMENT VARIABLES:
    CODECOV_TOKEN   Token for uploading to Codecov (required if --upload is used)

REQUIREMENTS:
    - cargo-tarpaulin installed (cargo install cargo-tarpaulin)
    - LLVM installed (for LLVM engine)
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

# Check if cargo-tarpaulin is installed
check_tarpaulin() {
    if ! command -v cargo-tarpaulin &> /dev/null; then
        log_error "cargo-tarpaulin not found"
        log_info "Install it with: cargo install cargo-tarpaulin --locked"
        exit 1
    fi
    log_info "✓ cargo-tarpaulin found: $(cargo tarpaulin --version)"
}

# Clean previous coverage data
clean_coverage() {
    log_step "Cleaning previous coverage data..."
    rm -rf coverage/
    mkdir -p coverage/
    log_info "✓ Coverage directory cleaned"
}

# Generate coverage report
generate_coverage() {
    log_step "Generating coverage report..."

    local OUTPUT_FORMATS
    if [ "$HTML_ONLY" = true ]; then
        OUTPUT_FORMATS="Html"
        log_info "Generating HTML report only (fast mode)"
    else
        OUTPUT_FORMATS="Xml,Html,Lcov"
        log_info "Generating all formats: XML, HTML, Lcov"
    fi

    # Run tarpaulin using config file
    SAYONARA_TEST_MODE=1 cargo tarpaulin \
        --verbose \
        --all-features \
        --workspace \
        --timeout 600 \
        --out "$OUTPUT_FORMATS" \
        --output-dir coverage \
        --exclude-files 'tests/*' 'benches/*' '*/mock_*' \
        --engine llvm \
        --count

    if [ $? -eq 0 ]; then
        log_info "✓ Coverage report generated successfully"
    else
        log_error "Coverage generation failed"
        exit 1
    fi
}

# Display coverage summary
show_coverage_summary() {
    log_step "Coverage Summary"
    echo ""

    if [ -f coverage/index.html ]; then
        # Extract coverage percentage from HTML (basic grep)
        local COVERAGE_PCT=$(grep -oP 'Coverage: \K[0-9.]+%' coverage/index.html | head -1 || echo "N/A")
        echo -e "${GREEN}Coverage: ${COVERAGE_PCT}${NC}"
    fi

    echo ""
    echo "Generated files:"
    ls -lh coverage/ | tail -n +2
    echo ""

    local TOTAL_SIZE=$(du -sh coverage/ | cut -f1)
    echo "Total size: $TOTAL_SIZE"
}

# Upload to Codecov
upload_codecov() {
    log_step "Uploading coverage to Codecov..."

    if [ -z "${CODECOV_TOKEN:-}" ]; then
        log_error "CODECOV_TOKEN environment variable not set"
        log_info "Set it with: export CODECOV_TOKEN=your_token_here"
        exit 1
    fi

    if [ ! -f coverage/cobertura.xml ]; then
        log_error "Coverage XML file not found"
        exit 1
    fi

    # Upload using bash uploader
    if command -v codecov &> /dev/null; then
        codecov -f coverage/cobertura.xml -t "$CODECOV_TOKEN"
    else
        # Fallback: use curl to upload
        curl -Os https://uploader.codecov.io/latest/linux/codecov
        chmod +x codecov
        ./codecov -f coverage/cobertura.xml -t "$CODECOV_TOKEN"
        rm codecov
    fi

    if [ $? -eq 0 ]; then
        log_info "✓ Coverage uploaded to Codecov"
    else
        log_error "Codecov upload failed"
        exit 1
    fi
}

# Open HTML report in browser
open_html_report() {
    log_step "Opening HTML report in browser..."

    local HTML_FILE="coverage/index.html"

    if [ ! -f "$HTML_FILE" ]; then
        log_error "HTML report not found: $HTML_FILE"
        exit 1
    fi

    # Detect OS and open browser
    if [[ "$OSTYPE" == "linux-gnu"* ]]; then
        if command -v xdg-open &> /dev/null; then
            xdg-open "$HTML_FILE"
        elif command -v firefox &> /dev/null; then
            firefox "$HTML_FILE" &
        else
            log_warn "Could not open browser. HTML report: $(pwd)/$HTML_FILE"
        fi
    elif [[ "$OSTYPE" == "darwin"* ]]; then
        open "$HTML_FILE"
    else
        log_warn "Unsupported OS for auto-opening browser"
        log_info "HTML report: $(pwd)/$HTML_FILE"
    fi

    log_info "✓ HTML report: $(pwd)/$HTML_FILE"
}

# Main execution
main() {
    log_info "Sayonara Wipe - Coverage Report Generator"
    log_info "=========================================="
    echo ""

    parse_args "$@"

    check_tarpaulin
    clean_coverage
    generate_coverage
    show_coverage_summary

    if [ "$UPLOAD_CODECOV" = true ]; then
        echo ""
        upload_codecov
    fi

    if [ "$OPEN_REPORT" = true ]; then
        echo ""
        open_html_report
    fi

    echo ""
    log_info "✓ Coverage report generation complete"
    echo ""
    log_info "View HTML report: coverage/index.html"

    if [ "$HTML_ONLY" = false ]; then
        log_info "View XML report: coverage/cobertura.xml"
        log_info "View Lcov report: coverage/lcov.info"
    fi
}

main "$@"
