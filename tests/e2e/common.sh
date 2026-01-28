#!/usr/bin/env bash
# E2E Test Framework - Common utilities and logging
# Surface Dial Volume Controller
#
# Usage: source "$(dirname "$0")/common.sh"

set -euo pipefail

# === Colors ===
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
GRAY='\033[0;90m'
BOLD='\033[1m'
NC='\033[0m' # No Color

# === Log File Setup ===
LOG_DIR="${LOG_DIR:-/tmp/surface-dial-tests}"
mkdir -p "$LOG_DIR"
LOG_FILE="$LOG_DIR/e2e_$(date +%Y%m%d_%H%M%S).log"
SUMMARY_FILE="$LOG_DIR/summary_$(date +%Y%m%d_%H%M%S).txt"

# === Counters ===
TESTS_PASSED=0
TESTS_FAILED=0
TESTS_SKIPPED=0
CURRENT_TEST=""
CURRENT_SUITE=""

# === Logging Functions ===
log() {
    local level="$1"
    shift
    local msg="$*"
    local timestamp
    timestamp=$(date '+%Y-%m-%d %H:%M:%S')

    # Write to log file (plain)
    echo "[$timestamp] [$level] $msg" >> "$LOG_FILE"

    # Write to console (colored)
    case "$level" in
        DEBUG) echo -e "${GRAY}[$timestamp] [DEBUG] $msg${NC}" ;;
        INFO)  echo -e "${BLUE}[$timestamp] [INFO]  $msg${NC}" ;;
        STEP)  echo -e "${CYAN}[$timestamp] [STEP]  $msg${NC}" ;;
        PASS)  echo -e "${GREEN}[$timestamp] [PASS]  $msg${NC}" ;;
        FAIL)  echo -e "${RED}[$timestamp] [FAIL]  $msg${NC}" ;;
        WARN)  echo -e "${YELLOW}[$timestamp] [WARN]  $msg${NC}" ;;
        *)     echo "[$timestamp] [$level] $msg" ;;
    esac
}

debug() { log DEBUG "$@"; }
info()  { log INFO "$@"; }
step()  { log STEP "$@"; }
pass()  { log PASS "$@"; }
fail()  { log FAIL "$@"; }
warn()  { log WARN "$@"; }

# === Test Management ===
suite_start() {
    CURRENT_SUITE="$1"
    echo ""
    info "=========================================="
    info "Starting test suite: $CURRENT_SUITE"
    info "=========================================="
}

test_start() {
    CURRENT_TEST="$1"
    step "Test: $CURRENT_TEST"
}

test_pass() {
    pass "✓ $CURRENT_TEST"
    ((TESTS_PASSED++)) || true
}

test_fail() {
    local reason="${1:-Test failed}"
    fail "✗ $CURRENT_TEST: $reason"
    ((TESTS_FAILED++)) || true
    echo "$CURRENT_SUITE::$CURRENT_TEST: FAILED - $reason" >> "$SUMMARY_FILE"
}

test_skip() {
    local reason="${1:-Skipped}"
    warn "⊘ $CURRENT_TEST: $reason"
    ((TESTS_SKIPPED++)) || true
}

# === Assertions ===
assert_eq() {
    local expected="$1"
    local actual="$2"
    local msg="${3:-Values should be equal}"

    if [[ "$expected" == "$actual" ]]; then
        debug "assert_eq passed: '$expected' == '$actual'"
        return 0
    else
        fail "$msg: expected '$expected', got '$actual'"
        return 1
    fi
}

assert_ne() {
    local expected="$1"
    local actual="$2"
    local msg="${3:-Values should not be equal}"

    if [[ "$expected" != "$actual" ]]; then
        debug "assert_ne passed: '$expected' != '$actual'"
        return 0
    else
        fail "$msg: values are equal '$expected'"
        return 1
    fi
}

assert_contains() {
    local haystack="$1"
    local needle="$2"
    local msg="${3:-Should contain substring}"

    if [[ "$haystack" == *"$needle"* ]]; then
        debug "assert_contains passed: found '$needle'"
        return 0
    else
        fail "$msg: '$needle' not found in output"
        return 1
    fi
}

assert_not_contains() {
    local haystack="$1"
    local needle="$2"
    local msg="${3:-Should not contain substring}"

    if [[ "$haystack" != *"$needle"* ]]; then
        debug "assert_not_contains passed: '$needle' not found"
        return 0
    else
        fail "$msg: '$needle' unexpectedly found in output"
        return 1
    fi
}

assert_file_exists() {
    local path="$1"
    local msg="${2:-File should exist}"

    if [[ -f "$path" ]]; then
        debug "assert_file_exists passed: $path"
        return 0
    else
        fail "$msg: file not found: $path"
        return 1
    fi
}

assert_dir_exists() {
    local path="$1"
    local msg="${2:-Directory should exist}"

    if [[ -d "$path" ]]; then
        debug "assert_dir_exists passed: $path"
        return 0
    else
        fail "$msg: directory not found: $path"
        return 1
    fi
}

assert_exit_code() {
    local expected="$1"
    local actual="$2"
    local msg="${3:-Exit code mismatch}"

    if [[ "$expected" == "$actual" ]]; then
        debug "assert_exit_code passed: $actual"
        return 0
    else
        fail "$msg: expected exit code $expected, got $actual"
        return 1
    fi
}

assert_success() {
    local exit_code="$1"
    local msg="${2:-Command should succeed}"

    if [[ "$exit_code" == "0" ]]; then
        debug "assert_success passed"
        return 0
    else
        fail "$msg: command failed with exit code $exit_code"
        return 1
    fi
}

assert_failure() {
    local exit_code="$1"
    local msg="${2:-Command should fail}"

    if [[ "$exit_code" != "0" ]]; then
        debug "assert_failure passed: exit code $exit_code"
        return 0
    else
        fail "$msg: command unexpectedly succeeded"
        return 1
    fi
}

assert_json_valid() {
    local json="$1"
    local msg="${2:-Should be valid JSON}"

    if echo "$json" | jq . >/dev/null 2>&1; then
        debug "assert_json_valid passed"
        return 0
    else
        fail "$msg: invalid JSON"
        return 1
    fi
}

assert_json_has_key() {
    local json="$1"
    local key="$2"
    local msg="${3:-JSON should have key}"

    if echo "$json" | jq -e ".$key" >/dev/null 2>&1; then
        debug "assert_json_has_key passed: .$key exists"
        return 0
    else
        fail "$msg: key .$key not found in JSON"
        return 1
    fi
}

assert_matches() {
    local text="$1"
    local pattern="$2"
    local msg="${3:-Should match pattern}"

    if [[ "$text" =~ $pattern ]]; then
        debug "assert_matches passed: matched '$pattern'"
        return 0
    else
        fail "$msg: '$text' doesn't match pattern '$pattern'"
        return 1
    fi
}

# === Cleanup ===
cleanup() {
    info "Cleaning up test environment..."
    # Kill any test daemons
    pkill -f "surface-dial.*test" 2>/dev/null || true
    # Remove temp configs (but preserve logs)
    rm -rf "${TEST_CONFIG_DIR:-/tmp/sd-test-config}" 2>/dev/null || true
}

trap cleanup EXIT

# === Summary ===
print_summary() {
    echo ""
    info "=========================================="
    info "Test Summary"
    info "=========================================="
    echo -e "${BOLD}Passed:${NC}  ${GREEN}$TESTS_PASSED${NC}"
    echo -e "${BOLD}Failed:${NC}  ${RED}$TESTS_FAILED${NC}"
    echo -e "${BOLD}Skipped:${NC} ${YELLOW}$TESTS_SKIPPED${NC}"
    info "Log file: $LOG_FILE"

    if [[ $TESTS_FAILED -gt 0 ]]; then
        echo ""
        fail "Some tests failed!"
        if [[ -f "$SUMMARY_FILE" ]]; then
            echo ""
            echo "Failed tests:"
            cat "$SUMMARY_FILE"
        fi
        return 1
    else
        echo ""
        pass "All tests passed!"
        return 0
    fi
}

# === Binary Path ===
# Look for binary in common locations
find_binary() {
    local locations=(
        "./target/release/surface-dial"
        "./target/debug/surface-dial"
        "../target/release/surface-dial"
        "../target/debug/surface-dial"
        "../../target/release/surface-dial"
        "../../target/debug/surface-dial"
    )

    for loc in "${locations[@]}"; do
        if [[ -x "$loc" ]]; then
            echo "$loc"
            return 0
        fi
    done

    # Try PATH
    if command -v surface-dial >/dev/null 2>&1; then
        echo "surface-dial"
        return 0
    fi

    return 1
}

BINARY="${BINARY:-$(find_binary || echo "./target/debug/surface-dial")}"
export TEST_CONFIG_DIR="${TEST_CONFIG_DIR:-/tmp/sd-test-config}"
export XDG_CONFIG_HOME="$TEST_CONFIG_DIR"
export HOME_OVERRIDE="$TEST_CONFIG_DIR"

# === Helper Functions ===

# Run a command and capture output, exit code
run_cmd() {
    local output
    local exit_code

    output=$("$@" 2>&1) && exit_code=0 || exit_code=$?

    echo "$output"
    return $exit_code
}

# Check if binary exists and is executable
check_binary() {
    if [[ ! -x "$BINARY" ]]; then
        fail "Binary not found or not executable: $BINARY"
        info "Please build with: cargo build --release"
        exit 1
    fi
    info "Using binary: $BINARY"
}

# Setup fresh test config directory
setup_test_config() {
    rm -rf "$TEST_CONFIG_DIR"
    mkdir -p "$TEST_CONFIG_DIR/surface-dial"
}

# Get the project root directory
get_project_root() {
    local script_dir
    script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
    echo "$(cd "$script_dir/../.." && pwd)"
}

# === Initialization ===
info "E2E Test Framework initialized"
info "Log file: $LOG_FILE"
info "Config dir: $TEST_CONFIG_DIR"
