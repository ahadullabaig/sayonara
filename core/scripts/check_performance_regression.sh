#!/usr/bin/env bash
#
# Check for Performance Regressions
#
# This script compares current benchmark results against a baseline to detect
# performance regressions. It's designed to be used in CI pipelines.
#
# Usage:
#   ./scripts/check_performance_regression.sh [OPTIONS]
#
# Options:
#   --threshold <percent>    Regression threshold percentage (default: 10)
#   --baseline <dir>         Baseline results directory (default: benchmarks/baseline)
#   --current <dir>          Current results directory (default: benchmarks/current)
#   --strict                 Fail on any regression (threshold = 0)
#   --help                   Show this help message
#
# Exit codes:
#   0 - No regressions detected
#   1 - Regressions detected
#   2 - Error (missing files, etc.)

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default configuration
THRESHOLD=10  # 10% regression threshold
BASELINE_DIR="benchmarks/baseline"
CURRENT_DIR="benchmarks/current"
STRICT_MODE=false
REGRESSIONS_FOUND=false

# Parse command-line arguments
parse_args() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            --threshold)
                THRESHOLD="$2"
                shift 2
                ;;
            --baseline)
                BASELINE_DIR="$2"
                shift 2
                ;;
            --current)
                CURRENT_DIR="$2"
                shift 2
                ;;
            --strict)
                STRICT_MODE=true
                THRESHOLD=0
                shift
                ;;
            --help)
                show_help
                exit 0
                ;;
            *)
                echo -e "${RED}Error: Unknown option: $1${NC}"
                show_help
                exit 2
                ;;
        esac
    done
}

# Show help message
show_help() {
    cat << EOF
Usage: $0 [OPTIONS]

Check for performance regressions in benchmarks

OPTIONS:
    --threshold <percent>    Regression threshold percentage (default: 10)
    --baseline <dir>         Baseline results directory (default: benchmarks/baseline)
    --current <dir>          Current results directory (default: benchmarks/current)
    --strict                 Fail on any regression (threshold = 0)
    --help                   Show this help message

EXAMPLES:
    $0                                     # Check with 10% threshold
    $0 --threshold 5                       # More strict (5% threshold)
    $0 --strict                            # Fail on any regression
    $0 --baseline old/ --current new/      # Custom directories

EXIT CODES:
    0    No regressions detected
    1    Regressions detected
    2    Error (missing files, invalid arguments, etc.)
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

log_regression() {
    echo -e "${RED}[REGRESSION]${NC} $1"
}

log_improvement() {
    echo -e "${GREEN}[IMPROVEMENT]${NC} $1"
}

# Check if required directories exist
check_directories() {
    if [ ! -d "$BASELINE_DIR" ]; then
        log_error "Baseline directory not found: $BASELINE_DIR"
        log_info "This is expected for the first run. Skipping regression check."
        exit 0  # Exit successfully (no baseline yet)
    fi

    if [ ! -d "$CURRENT_DIR" ]; then
        log_error "Current results directory not found: $CURRENT_DIR"
        exit 2
    fi

    log_info "Baseline: $BASELINE_DIR"
    log_info "Current:  $CURRENT_DIR"
}

# Extract benchmark results from Criterion output
# This is a simplified parser - adjust based on actual Criterion output format
parse_criterion_results() {
    local RESULTS_DIR="$1"
    local OUTPUT_FILE="$2"

    # Check if criterion output exists
    if [ -d "../target/criterion" ]; then
        # Parse criterion JSON reports (if available)
        find ../target/criterion -name "estimates.json" -print0 | while IFS= read -r -d '' file; do
            local BENCH_NAME=$(dirname "$file" | xargs basename)
            local MEAN_TIME=$(jq -r '.mean.point_estimate' "$file" 2>/dev/null || echo "0")
            echo "$BENCH_NAME:$MEAN_TIME" >> "$OUTPUT_FILE"
        done
    fi

    # Fallback: Parse text output
    if [ -f "$RESULTS_DIR/benchmark_output.txt" ]; then
        grep -E "time:" "$RESULTS_DIR/benchmark_output.txt" | \
            awk '{print $1":"$3}' >> "$OUTPUT_FILE" || true
    fi
}

# Compare benchmark results
compare_results() {
    log_info "Comparing benchmark results..."
    echo ""

    local BASELINE_FILE="/tmp/baseline_results.txt"
    local CURRENT_FILE="/tmp/current_results.txt"

    # Clean temp files
    rm -f "$BASELINE_FILE" "$CURRENT_FILE"

    # Parse results
    parse_criterion_results "$BASELINE_DIR" "$BASELINE_FILE"
    parse_criterion_results "$CURRENT_DIR" "$CURRENT_FILE"

    # Check if we have results to compare
    if [ ! -s "$BASELINE_FILE" ] || [ ! -s "$CURRENT_FILE" ]; then
        log_warn "Insufficient data for comparison"
        log_info "Baseline entries: $(wc -l < "$BASELINE_FILE" 2>/dev/null || echo 0)"
        log_info "Current entries:  $(wc -l < "$CURRENT_FILE" 2>/dev/null || echo 0)"
        return 0
    fi

    # Compare each benchmark
    echo "Benchmark Comparison Results:"
    echo "=============================="
    printf "%-40s %15s %15s %10s\n" "Benchmark" "Baseline" "Current" "Change"
    echo "--------------------------------------------------------------------------------"

    while IFS=: read -r bench_name baseline_time; do
        # Find corresponding current result
        local current_time=$(grep "^$bench_name:" "$CURRENT_FILE" | cut -d: -f2 || echo "0")

        if [ "$current_time" = "0" ] || [ -z "$current_time" ]; then
            log_warn "Benchmark not found in current results: $bench_name"
            continue
        fi

        # Calculate percentage change
        # Using bc for floating point arithmetic
        local change_pct=$(echo "scale=2; (($current_time - $baseline_time) / $baseline_time) * 100" | bc -l 2>/dev/null || echo "0")

        # Determine if this is a regression
        local is_regression=$(echo "$change_pct > $THRESHOLD" | bc -l)

        # Format output
        printf "%-40s %15s %15s" "$bench_name" "$baseline_time" "$current_time"

        if [ "$is_regression" -eq 1 ]; then
            printf " %s+%.2f%%%s (REGRESSION)\n" "$RED" "$change_pct" "$NC"
            REGRESSIONS_FOUND=true
            log_regression "$bench_name: +${change_pct}% slower (threshold: ${THRESHOLD}%)"
        elif [ "$(echo "$change_pct < -5" | bc -l)" -eq 1 ]; then
            printf " %s%.2f%%%s (improvement)\n" "$GREEN" "$change_pct" "$NC"
            log_improvement "$bench_name: ${change_pct}% faster"
        else
            printf " %s%.2f%%%s (no change)\n" "$NC" "$change_pct" "$NC"
        fi

    done < "$BASELINE_FILE"

    echo "--------------------------------------------------------------------------------"
    echo ""

    # Cleanup
    rm -f "$BASELINE_FILE" "$CURRENT_FILE"
}

# Generate summary
generate_summary() {
    echo ""
    echo "================================"
    echo "  Performance Regression Report  "
    echo "================================"
    echo ""
    echo "Threshold: ${THRESHOLD}%"
    echo ""

    if [ "$REGRESSIONS_FOUND" = true ]; then
        echo -e "${RED}❌ REGRESSIONS DETECTED${NC}"
        echo ""
        echo "Performance has degraded beyond the acceptable threshold."
        echo "Please investigate and optimize before merging."
        return 1
    else
        echo -e "${GREEN}✓ NO REGRESSIONS DETECTED${NC}"
        echo ""
        echo "Performance is within acceptable range."
        return 0
    fi
}

# Main execution
main() {
    log_info "Sayonara Wipe - Performance Regression Checker"
    log_info "==============================================="
    echo ""

    parse_args "$@"
    check_directories

    if [ "$STRICT_MODE" = true ]; then
        log_warn "STRICT MODE: Any performance regression will fail"
    fi

    echo ""
    compare_results

    echo ""
    if generate_summary; then
        exit 0
    else
        exit 1
    fi
}

main "$@"
