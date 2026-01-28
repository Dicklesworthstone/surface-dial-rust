#!/usr/bin/env bash
# E2E Tests for CLI commands
# Surface Dial Volume Controller

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/common.sh"

check_binary
setup_test_config

suite_start "CLI Command Tests"

# =============================================================================
# Version and Help Tests
# =============================================================================

test_start "Version command shows version"
{
    output=$(run_cmd "$BINARY" --version) && exit_code=0 || exit_code=$?
    assert_success "$exit_code"
    assert_contains "$output" "surface-dial"
    test_pass
} || test_fail "$output"

test_start "Help command shows usage"
{
    output=$(run_cmd "$BINARY" --help) && exit_code=0 || exit_code=$?
    assert_success "$exit_code"
    assert_contains "$output" "Usage"
    assert_contains "$output" "daemon"
    assert_contains "$output" "config"
    test_pass
} || test_fail "$output"

test_start "Help for subcommands"
{
    for cmd in daemon config status; do
        output=$(run_cmd "$BINARY" "$cmd" --help) && exit_code=0 || exit_code=$?
        assert_success "$exit_code" "Help for $cmd"
        assert_contains "$output" "Usage" "Help for $cmd shows usage"
    done
    test_pass
} || test_fail "$output"

# =============================================================================
# Config Command Tests
# =============================================================================

test_start "Config list shows all keys"
{
    output=$(run_cmd "$BINARY" config list) && exit_code=0 || exit_code=$?
    assert_success "$exit_code"
    # Should contain major config sections
    assert_contains "$output" "volume"
    assert_contains "$output" "sensitivity"
    test_pass
} || test_fail "$output"

test_start "Config get returns value"
{
    output=$(run_cmd "$BINARY" config get volume.step_min) && exit_code=0 || exit_code=$?
    assert_success "$exit_code"
    # Default value should be 2
    assert_eq "2" "$output"
    test_pass
} || test_fail "$output"

test_start "Config set updates value"
{
    # Set a new value
    run_cmd "$BINARY" config set volume.step_max 10 && exit_code=0 || exit_code=$?
    assert_success "$exit_code"

    # Verify it was set
    output=$(run_cmd "$BINARY" config get volume.step_max) && exit_code=0 || exit_code=$?
    assert_eq "10" "$output"
    test_pass
} || test_fail "$output"

test_start "Config set rejects invalid values"
{
    # Try to set invalid value (step_min must be 1-10)
    output=$(run_cmd "$BINARY" config set volume.step_min 50) && exit_code=0 || exit_code=$?
    assert_failure "$exit_code"
    assert_contains "$output" "must be"
    test_pass
} || test_fail "$output"

test_start "Config reset restores defaults"
{
    # Change a value
    run_cmd "$BINARY" config set volume.step_max 15

    # Reset
    run_cmd "$BINARY" config reset && exit_code=0 || exit_code=$?
    assert_success "$exit_code"

    # Verify default is restored
    output=$(run_cmd "$BINARY" config get volume.step_max)
    assert_eq "8" "$output"
    test_pass
} || test_fail "$output"

test_start "Config get with invalid key fails"
{
    output=$(run_cmd "$BINARY" config get invalid.key.here) && exit_code=0 || exit_code=$?
    assert_failure "$exit_code"
    test_pass
} || test_fail "$output"

# =============================================================================
# Status Command Tests
# =============================================================================

test_start "Status command runs without daemon"
{
    output=$(run_cmd "$BINARY" status) && exit_code=0 || exit_code=$?
    assert_success "$exit_code"
    # Should show daemon is not running
    assert_contains "$output" "Daemon" || assert_contains "$output" "daemon"
    test_pass
} || test_fail "$output"

test_start "Status --json returns valid JSON"
{
    output=$(run_cmd "$BINARY" status --json) && exit_code=0 || exit_code=$?
    assert_success "$exit_code"
    assert_json_valid "$output"
    test_pass
} || test_fail "$output"

test_start "Status JSON has expected fields"
{
    output=$(run_cmd "$BINARY" status --json)
    assert_json_valid "$output"
    # Should have daemon status field
    assert_json_has_key "$output" "daemon" || assert_contains "$output" "daemon"
    test_pass
} || test_fail "$output"

# =============================================================================
# Exit Code Tests
# =============================================================================

test_start "Invalid subcommand returns error"
{
    run_cmd "$BINARY" invalid-subcommand-xyz && exit_code=0 || exit_code=$?
    assert_failure "$exit_code"
    test_pass
} || test_fail

test_start "Missing required args returns error"
{
    # config get without a key should fail
    run_cmd "$BINARY" config get && exit_code=0 || exit_code=$?
    assert_failure "$exit_code"
    test_pass
} || test_fail

# =============================================================================
# JSON Output Tests
# =============================================================================

test_start "Global --json flag works"
{
    output=$(run_cmd "$BINARY" --json status) && exit_code=0 || exit_code=$?
    assert_success "$exit_code"
    assert_json_valid "$output"
    test_pass
} || test_fail "$output"

# =============================================================================
# Summary
# =============================================================================

print_summary
