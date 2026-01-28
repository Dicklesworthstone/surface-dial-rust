#!/usr/bin/env bash
# E2E Tests for Configuration System
# Surface Dial Volume Controller

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/common.sh"

check_binary
setup_test_config

suite_start "Configuration Tests"

# =============================================================================
# Default Config Creation Tests
# =============================================================================

test_start "Default config created on first run"
{
    rm -rf "$TEST_CONFIG_DIR"

    # Any command that reads config should create defaults
    run_cmd "$BINARY" config get volume.step_min >/dev/null

    config_file="$TEST_CONFIG_DIR/surface-dial/config.toml"
    assert_file_exists "$config_file" "Config file should be created"
    test_pass
} || test_fail "$output"

test_start "Default config directory structure"
{
    assert_dir_exists "$TEST_CONFIG_DIR/surface-dial"
    test_pass
} || test_fail

# =============================================================================
# Volume Config Tests
# =============================================================================

test_start "Volume step_min configurable (1-10)"
{
    for val in 1 5 10; do
        run_cmd "$BINARY" config set volume.step_min "$val"
        output=$(run_cmd "$BINARY" config get volume.step_min)
        assert_eq "$val" "$output" "step_min should be $val"
    done
    test_pass
} || test_fail "$output"

test_start "Volume step_min rejects out-of-range"
{
    run_cmd "$BINARY" config set volume.step_min 0 && exit_code=0 || exit_code=$?
    assert_failure "$exit_code"

    run_cmd "$BINARY" config set volume.step_min 11 && exit_code=0 || exit_code=$?
    assert_failure "$exit_code"
    test_pass
} || test_fail

test_start "Volume step_max configurable (5-20)"
{
    for val in 5 10 20; do
        run_cmd "$BINARY" config set volume.step_max "$val"
        output=$(run_cmd "$BINARY" config get volume.step_max)
        assert_eq "$val" "$output" "step_max should be $val"
    done
    test_pass
} || test_fail "$output"

test_start "Volume curve options"
{
    for curve in linear logarithmic; do
        run_cmd "$BINARY" config set volume.curve "$curve"
        output=$(run_cmd "$BINARY" config get volume.curve)
        assert_eq "$curve" "$output" "curve should be $curve"
    done
    test_pass
} || test_fail "$output"

# =============================================================================
# Sensitivity Config Tests
# =============================================================================

test_start "Sensitivity dead_zone configurable (0-10)"
{
    for val in 0 3 5 10; do
        run_cmd "$BINARY" config set sensitivity.dead_zone "$val"
        output=$(run_cmd "$BINARY" config get sensitivity.dead_zone)
        assert_eq "$val" "$output" "dead_zone should be $val"
    done
    test_pass
} || test_fail "$output"

test_start "Sensitivity multiplier configurable"
{
    run_cmd "$BINARY" config set sensitivity.multiplier 1.5
    output=$(run_cmd "$BINARY" config get sensitivity.multiplier)
    assert_eq "1.5" "$output"
    test_pass
} || test_fail "$output"

test_start "Sensitivity invert toggle"
{
    run_cmd "$BINARY" config set sensitivity.invert true
    output=$(run_cmd "$BINARY" config get sensitivity.invert)
    assert_eq "true" "$output"

    run_cmd "$BINARY" config set sensitivity.invert false
    output=$(run_cmd "$BINARY" config get sensitivity.invert)
    assert_eq "false" "$output"
    test_pass
} || test_fail "$output"

# =============================================================================
# Interaction Config Tests
# =============================================================================

test_start "Interaction double_click_ms configurable"
{
    run_cmd "$BINARY" config set interaction.double_click_ms 300
    output=$(run_cmd "$BINARY" config get interaction.double_click_ms)
    assert_eq "300" "$output"
    test_pass
} || test_fail "$output"

test_start "Interaction long_press_ms configurable"
{
    run_cmd "$BINARY" config set interaction.long_press_ms 800
    output=$(run_cmd "$BINARY" config get interaction.long_press_ms)
    assert_eq "800" "$output"
    test_pass
} || test_fail "$output"

# =============================================================================
# Daemon Config Tests
# =============================================================================

test_start "Daemon log_level configurable"
{
    for level in error warn info debug trace; do
        run_cmd "$BINARY" config set daemon.log_level "$level"
        output=$(run_cmd "$BINARY" config get daemon.log_level)
        assert_eq "$level" "$output" "log_level should be $level"
    done
    test_pass
} || test_fail "$output"

# =============================================================================
# Config File Persistence Tests
# =============================================================================

test_start "Config changes persist to file"
{
    run_cmd "$BINARY" config set volume.step_max 12

    config_file="$TEST_CONFIG_DIR/surface-dial/config.toml"
    content=$(cat "$config_file")
    assert_contains "$content" "step_max = 12"
    test_pass
} || test_fail "$content"

test_start "Config survives binary restart"
{
    run_cmd "$BINARY" config set sensitivity.dead_zone 7

    # Simulate "restart" by clearing any state
    sleep 0.1

    output=$(run_cmd "$BINARY" config get sensitivity.dead_zone)
    assert_eq "7" "$output"
    test_pass
} || test_fail "$output"

# =============================================================================
# Config Validation Tests
# =============================================================================

test_start "Config validation: step_min < step_max"
{
    # Set step_max high first
    run_cmd "$BINARY" config set volume.step_max 15

    # step_min should not exceed step_max
    run_cmd "$BINARY" config set volume.step_min 1
    output=$(run_cmd "$BINARY" config get volume.step_min)
    assert_eq "1" "$output"
    test_pass
} || test_fail "$output"

test_start "Config list shows all configurable keys"
{
    output=$(run_cmd "$BINARY" config list)

    # Should contain key sections
    assert_contains "$output" "volume.step_min"
    assert_contains "$output" "volume.step_max"
    assert_contains "$output" "sensitivity.dead_zone"
    assert_contains "$output" "sensitivity.multiplier"
    assert_contains "$output" "interaction.double_click_ms"
    assert_contains "$output" "daemon.log_level"
    test_pass
} || test_fail "$output"

# =============================================================================
# Config Path Tests
# =============================================================================

test_start "Config path command shows location"
{
    output=$(run_cmd "$BINARY" config path) && exit_code=0 || exit_code=$?
    assert_success "$exit_code"
    assert_contains "$output" "config.toml" || assert_contains "$output" "surface-dial"
    test_pass
} || test_fail "$output"

# =============================================================================
# Summary
# =============================================================================

print_summary
