#!/usr/bin/env bash
# E2E Test Runner
# Surface Dial Volume Controller
#
# Runs all E2E tests and produces a summary report.
# Usage: ./run_all.sh [--quick] [test_name...]

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
BOLD='\033[1m'
NC='\033[0m'

# === Configuration ===
LOG_DIR="${LOG_DIR:-/tmp/surface-dial-tests}"
mkdir -p "$LOG_DIR"
RUNNER_LOG="$LOG_DIR/runner_$(date +%Y%m%d_%H%M%S).log"

TOTAL_PASSED=0
TOTAL_FAILED=0
TOTAL_SKIPPED=0
FAILED_SUITES=()

# === Functions ===

log() {
    echo -e "$@" | tee -a "$RUNNER_LOG"
}

header() {
    log ""
    log "${BOLD}╔══════════════════════════════════════════════════════════════╗${NC}"
    log "${BOLD}║ Surface Dial E2E Test Suite                                   ║${NC}"
    log "${BOLD}╚══════════════════════════════════════════════════════════════╝${NC}"
    log ""
}

check_prerequisites() {
    log "${BLUE}Checking prerequisites...${NC}"

    # Check for binary
    local binary="${BINARY:-$PROJECT_ROOT/target/debug/surface-dial}"
    if [[ ! -x "$binary" ]]; then
        binary="$PROJECT_ROOT/target/release/surface-dial"
    fi

    if [[ ! -x "$binary" ]]; then
        log "${RED}Error: Binary not found. Please build first:${NC}"
        log "  cargo build --release"
        exit 1
    fi

    export BINARY="$binary"
    log "  Binary: $BINARY"

    # Check for jq (used in JSON tests)
    if ! command -v jq >/dev/null 2>&1; then
        log "${YELLOW}Warning: jq not found. Some JSON tests may be skipped.${NC}"
    fi

    log "${GREEN}Prerequisites OK${NC}"
    log ""
}

run_test_suite() {
    local test_file="$1"
    local test_name
    test_name=$(basename "$test_file" .sh)

    log "${BLUE}Running: $test_name${NC}"
    log "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

    # Run the test suite
    local exit_code=0
    bash "$test_file" 2>&1 | tee -a "$RUNNER_LOG" || exit_code=$?

    if [[ $exit_code -eq 0 ]]; then
        log "${GREEN}$test_name: PASSED${NC}"
    else
        log "${RED}$test_name: FAILED${NC}"
        FAILED_SUITES+=("$test_name")
    fi

    log ""
    return $exit_code
}

run_all_tests() {
    local test_files=()

    # Find all test files
    for f in "$SCRIPT_DIR"/test_*.sh; do
        if [[ -f "$f" ]]; then
            test_files+=("$f")
        fi
    done

    if [[ ${#test_files[@]} -eq 0 ]]; then
        log "${RED}No test files found!${NC}"
        exit 1
    fi

    log "Found ${#test_files[@]} test suite(s)"
    log ""

    local suite_passed=0
    local suite_failed=0

    for test_file in "${test_files[@]}"; do
        if run_test_suite "$test_file"; then
            ((suite_passed++)) || true
        else
            ((suite_failed++)) || true
        fi
    done

    return $suite_failed
}

run_specific_tests() {
    local args=("$@")
    local suite_failed=0

    for arg in "${args[@]}"; do
        local test_file="$SCRIPT_DIR/test_${arg}.sh"
        if [[ -f "$test_file" ]]; then
            if ! run_test_suite "$test_file"; then
                ((suite_failed++)) || true
            fi
        else
            log "${RED}Test not found: $arg${NC}"
            ((suite_failed++)) || true
        fi
    done

    return $suite_failed
}

print_final_summary() {
    log ""
    log "${BOLD}╔══════════════════════════════════════════════════════════════╗${NC}"
    log "${BOLD}║ Final Summary                                                  ║${NC}"
    log "${BOLD}╚══════════════════════════════════════════════════════════════╝${NC}"
    log ""

    if [[ ${#FAILED_SUITES[@]} -eq 0 ]]; then
        log "${GREEN}${BOLD}All test suites passed!${NC}"
    else
        log "${RED}${BOLD}Failed suites:${NC}"
        for suite in "${FAILED_SUITES[@]}"; do
            log "  ${RED}✗ $suite${NC}"
        done
    fi

    log ""
    log "Log file: $RUNNER_LOG"
    log ""
}

# === Main ===

header
check_prerequisites

# Parse arguments
QUICK_MODE=false
SPECIFIC_TESTS=()

while [[ $# -gt 0 ]]; do
    case "$1" in
        --quick|-q)
            QUICK_MODE=true
            shift
            ;;
        --help|-h)
            echo "Usage: $0 [--quick] [test_name...]"
            echo ""
            echo "Options:"
            echo "  --quick, -q    Run only fast tests"
            echo "  --help, -h     Show this help"
            echo ""
            echo "Examples:"
            echo "  $0              # Run all tests"
            echo "  $0 cli config   # Run only cli and config tests"
            exit 0
            ;;
        *)
            SPECIFIC_TESTS+=("$1")
            shift
            ;;
    esac
done

# Run tests
failed=0
if [[ ${#SPECIFIC_TESTS[@]} -gt 0 ]]; then
    run_specific_tests "${SPECIFIC_TESTS[@]}" || failed=$?
else
    run_all_tests || failed=$?
fi

print_final_summary

exit $failed
