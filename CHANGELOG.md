# Changelog

All notable changes to **Surface Dial Volume Controller** are documented here.

This project has no tagged releases yet (current version: `0.1.0`). There are no GitHub Releases or git tags. The release workflow (`.github/workflows/release.yml`) is configured and ready to trigger on `v*` tags or manual dispatch, building for 5 platform targets. Entries below are organized by date and grouped by capability area. All links point to live commits at [Dicklesworthstone/surface-dial-rust](https://github.com/Dicklesworthstone/surface-dial-rust).

---

## [Unreleased] -- 0.1.0-dev

A Rust daemon that transforms the Microsoft Surface Dial (VID `045E`, PID `091B`) into a universal volume controller with adaptive acceleration, state-machine click detection, perceptual volume curves, and cross-platform support (macOS, Linux, Windows). ~15k lines of Rust across 8 modules, 302 Rust tests (unit + integration) and 32 shell-based E2E tests.

---

### 2026-02-25

#### Agent Tooling Documentation

- [Add cass (Cross-Agent Session Search) tool reference to AGENTS.md](https://github.com/Dicklesworthstone/surface-dial-rust/commit/640c93294957fd1cd522ba78b7c683187e5bc853) -- documents how to use the `cass` CLI for cross-session knowledge reuse across Claude Code, Codex, Cursor, Gemini, and ChatGPT agents. Includes usage examples for `--robot`/`--json` flags, field filtering, agent filtering, and recency limiting.

---

### 2026-02-21 -- 2026-02-22

#### License Update

- [Replace plain MIT license with MIT + OpenAI/Anthropic Rider](https://github.com/Dicklesworthstone/surface-dial-rust/commit/1534c9b2a3b3dd9d09ceacc69b8c9f5415b6e62a) -- restricts use by OpenAI, Anthropic, and their affiliates without express written permission from Jeffrey Emanuel.
- [Update README license badge and references to match](https://github.com/Dicklesworthstone/surface-dial-rust/commit/e7ddd6b5eb2b495bf065929acf7508a4d0562303).

#### Repository Assets

- [Add GitHub social preview image](https://github.com/Dicklesworthstone/surface-dial-rust/commit/cb210b3645551569a25b88db28efef72f10271aa) (1280x640 PNG) for consistent link previews when sharing the repository URL.

---

### 2026-02-15

#### Repository Assets

- [Add dial hardware illustration PNG](https://github.com/Dicklesworthstone/surface-dial-rust/commit/4f193dbac85262eb1703d3da3f01e9373f29eaa9) -- the full-resolution asset (1.7 MB) referenced by README but previously missing from the repository.

---

### 2026-01-27 -- Initial Development Day

The entire functional codebase was built on this date across 25 commits. Organized below by capability rather than chronological order.

#### Core Daemon and HID Integration

- [**Initial commit -- full Surface Dial volume controller daemon**](https://github.com/Dicklesworthstone/surface-dial-rust/commit/046ff5bec6d3ffa82805d28e39847bbe31a7e7d9) -- 8,311 lines across 38 files. This single commit delivers:
  - **HID device polling** for Surface Dial (`0x045E:0x091B`) via `hidapi`, parsing 3-byte HID reports into button state and rotation values
  - **Adaptive acceleration** mapping rotation speed to volume step size: <80ms between ticks = max step (8%), >400ms = min step (2%), interpolated in between
  - **State-machine click detection** with four patterns: single click (toggle mute), double click (switch to microphone mode for configurable duration), triple click (media play/pause), long press at 1s (send F15 key)
  - **Perceptual volume curves**: linear, logarithmic, and exponential with configurable steepness
  - **52-key TOML configuration system** with range validation and cross-field constraints (`step_min <= step_max`, `double_click_ms < triple_click_ms < long_press_ms`)
  - **Rotating file logger** with JSON mode, dual console/file output, configurable levels, and size-based rotation
  - **Full CLI** via clap: `daemon` (with `--foreground`, `--config`, `--log-level`), `status` (with `--watch`, `--json`, `--check`), `config` (get/set/reset/show/path subcommands)
  - **Platform abstraction layer**: macOS via `osascript`/AppleScript, Linux via `wpctl` (PipeWire) or `pactl` (PulseAudio), Windows via PowerShell + COM APIs
  - **8 debug/diagnostic binaries**: `debug_dial`, `debug_events`, `dial_reader`, `event_tap`, `hid_blocking`, `hid_direct`, `hid_poll`, `list_devices`
  - **GitHub Actions release workflow** (`release.yml`) building for 5 targets: macOS aarch64, macOS x86_64, Linux x86_64, Linux aarch64, Windows x86_64
  - **Unified `install.sh`** with platform auto-detection and `uninstall.sh`
  - **85 unit tests** across config, input, daemon, logging, platform, and CLI modules

#### Installer -- Cross-Platform Distribution

- [Rewrite macOS installer with launchd auto-start](https://github.com/Dicklesworthstone/surface-dial-rust/commit/d5e75bacd74fd1040e36e209fee920418d5838ad) -- no-sudo installation to `~/.local/bin/surface-dial`, dynamically generated plist (no external file dependency), ad-hoc signing for Apple Silicon compatibility, logs to `~/.local/share/surface-dial/`, added `--uninstall` flag to `install.sh`.
- [Complete cross-platform installer](https://github.com/Dicklesworthstone/surface-dial-rust/commit/804006a521be7ee591121f9661f757e618d1dfe3) -- adds Linux support (systemd user service generation, udev rule guidance for HID permissions, journald logging, `enable-linger` for auto-start without login), Windows support (Task Scheduler via PowerShell, toast notification integration), unified colored output with banner, `curl` one-liner support, and environment variables (`DEST`, `NO_AUTOSTART`, `VERSION`).

#### Bug Fixes

- [Fix GitHub Action name and cross-compilation](https://github.com/Dicklesworthstone/surface-dial-rust/commit/ba669d5f0838c990baf3a292e711f8fce2c0b1a0) -- correct nonexistent `dtolnay/rust-action` to `dtolnay/rust-toolchain`, add linker config for Linux ARM64 cross-compilation, consolidate redundant `apt-get update` calls, add missing plist file check and unsupported CPU architecture validation.
- [Fix HID permission check and banner alignment in `install.sh`](https://github.com/Dicklesworthstone/surface-dial-rust/commit/81e80d0a91d537a5ad8d00da6e0faf573cf5e414) -- previous `xargs` approach silently succeeded when no `/dev/hidraw*` devices existed, failing to show the warning; now uses explicit loop. Fixed misaligned box-drawing characters caused by variable-length platform strings using `printf` with fixed-width fields.
- [Fix MockHidDevice mutex contention](https://github.com/Dicklesworthstone/surface-dial-rust/commit/6bdf8fe035d096ed44a143b609e3aefdefa535a5) -- release mutex before sleeping in `read_timeout` to prevent contention in multi-threaded tests; only pop events when connected to prevent event loss across disconnect/reconnect cycles.
- [Fix code issues from review](https://github.com/Dicklesworthstone/surface-dial-rust/commit/5a1978bd426af63a40fc3f1a11902f0bccc91a58) -- add `prev_track` to valid media actions in config validation (daemon supported it but validation rejected it), add buffer bounds check in `get_feature_report()` to prevent panics with small buffers, remove unreachable code branch in log rotation loop, remove unused `Duration` import.
- [Fix click detection clarity and mic mute behavior](https://github.com/Dicklesworthstone/surface-dial-rust/commit/04eb2aa98a41dbfe3f92b2e56329689f21b5a557) -- remove redundant double-click time check that always passed (compared `now` against `last_release_time` which was just set to `now`); fix macOS `toggle_mic_mute` to remember previous volume before muting instead of hardcoded 50% restore, using `AtomicI32` for thread-safe state tracking.

#### Test Infrastructure -- Integration Testing

- [Add HID device abstraction and mock platform for integration testing](https://github.com/Dicklesworthstone/surface-dial-rust/commit/3f972da20c2a96e7819e26e4491dda60128d0062) -- introduces `HidDevice` trait with `RealHidDevice` wrapper and `MockHidDevice` (event queuing for simulated dial input), `MockPlatform` that records all audio/key operations for assertion. 45+ integration tests across three files:
  - `tests/hid_integration.rs` (9 tests) -- HID device simulation
  - `tests/input_integration.rs` (19 tests) -- click detection and rotation processing
  - `tests/config_integration.rs` (17 tests) -- configuration persistence and validation

#### Test Infrastructure -- Unit Tests

- [Add 36 unit tests to input module](https://github.com/Dicklesworthstone/surface-dial-rust/commit/25b71036074b24b2652546a77f3ab8bb65f1425b) -- `calculate_step` boundary conditions and interpolation, `RotationProcessor` accumulation/bidirectional/multiplier/extremes, `ClickDetector` state transitions and config updates, `ClickConfig`/`SensitivityConfig` defaults. Documents a known limitation: triple-click is unreachable in current implementation because double-click fires immediately and resets state.
- [Add 22 logging behavior tests](https://github.com/Dicklesworthstone/surface-dial-rust/commit/9b10f150afa18d8fde43c94e684188a10140aeab) -- file creation with parent directories, append mode, size tracking, plain text and JSON entry formats, TTY detection with color codes per level, log level filtering (console/file/combined), file rotation with oldest-file deletion, structured events (startup, shutdown, error), integration workflow simulation.
- [Add PID file management module with 24 tests](https://github.com/Dicklesworthstone/surface-dial-rust/commit/9fa709ec6df95fe83c500e08dacd89c21bee2d81) -- new `pidfile` module providing PID file creation with automatic parent directory creation, stale PID detection and cleanup, process liveness checking (Unix `kill -0`), automatic file cleanup on drop, persistent mode for testing, Unix permissions (0644), duplicate instance prevention via `AlreadyRunning` error, crash recovery by replacing stale PID files.
- [Expand daemon tests from 3 to 35](https://github.com/Dicklesworthstone/surface-dial-rust/commit/db1b94abfd380d1a6f61ad59e93f9ccf69822112) (11.7x increase, full suite grows from 116 to 143 tests) -- covers control mode equality/debug/clone, `DaemonStats` defaults, daemon creation/initial state, running flag with cross-thread shutdown, mode switching (volume/mic with rotation processor reset), config reload with state preservation, click result processing for all patterns, mic mode expiry, button state tracking, integration-style sequences (full double-click and triple-click flows), USB constant verification.
- [Add 14 signal handling behavioral tests](https://github.com/Dicklesworthstone/surface-dial-rust/commit/6ddd516ba1574f2f781badfa3b7d41ea106ac651) -- SIGTERM/SIGINT graceful shutdown via running flag, SIGHUP config reload while running, cross-thread signal delivery, idempotent multiple shutdown signals, stats preservation on shutdown, click/rotation processor config updates on reload, mic mode and connected state on shutdown, uptime calculation.
- [Add 14 error recovery behavioral tests](https://github.com/Dicklesworthstone/surface-dial-rust/commit/bf8a050158ed9e5c5f994574ca0dc7524f5595df) -- device disconnect flag behavior, state preservation through disconnect/reconnect cycles, mode preservation (volume/mic), button state handling on mid-press disconnect, click/rotation processing works after reconnect, stats integrity under error conditions, config reload during error state, running flag unaffected by device errors, mic mode timer continues during disconnect, recovery after many error cycles.
- [Add 13 daemon lifecycle tests](https://github.com/Dicklesworthstone/surface-dial-rust/commit/a45e747ca9d28634da5f4a98f818890e72ce30d9) -- start/stop/restart lifecycle verification, atomic compare-and-swap single-instance prevention, config persistence throughout lifecycle, shutdown from any state (running, mic mode, disconnected), uptime calculation from `start_time`, processing stop after shutdown signal.

#### Test Infrastructure -- E2E Framework

- [Add E2E test framework with shell-based logging infrastructure](https://github.com/Dicklesworthstone/surface-dial-rust/commit/bc62cdadd8cb55faff96aad2ac2dad43ca4e9e08) -- new `tests/e2e/` directory with:
  - `common.sh` -- shared logging framework with colored output (DEBUG/INFO/STEP/PASS/FAIL/WARN), file logging with timestamps, 20+ assertion functions (`eq`, `contains`, `file_exists`, `json_valid`, etc.), test suite management, automatic cleanup on exit, summary reporting
  - `test_cli.sh` -- 15 CLI command tests (version, help, config get/set/reset/list, status plain and JSON, exit code verification)
  - `test_config.sh` -- 17 configuration tests (default config creation, volume/sensitivity/interaction settings, persistence, validation)
  - `run_all.sh` -- test runner with automatic discovery, per-suite and final summaries, selective execution

#### Documentation

- [Add comprehensive README and MIT LICENSE](https://github.com/Dicklesworthstone/surface-dial-rust/commit/eaeff78dfd89f86e5258371f2bcb90fa3389fb84) -- 717-line README with TL;DR and feature comparison table, design philosophy (5 core principles: zero-latency feel, state-machine click detection, perceptual volume curves, platform abstraction without FFI, configuration as data), comparison vs alternatives (SteerMouse, Karabiner, USB Overdrive), installation guide for macOS/Linux with service setup (launchd/systemd), complete CLI reference, full 52-key configuration reference with TOML comments, ASCII architecture and data flow diagrams, troubleshooting guide, limitations table, 7-item FAQ, contribution policy.
- [Add illustration to README](https://github.com/Dicklesworthstone/surface-dial-rust/commit/3f364975a578ee25bf79aeee40684481d257de7c) -- convert `dial_illustration.png` (1.6 MB) to webp (116 KB), replace ASCII art hero section with centered image.

#### Beads Metadata (no functional changes)

- [`bd sync` at 16:00:17](https://github.com/Dicklesworthstone/surface-dial-rust/commit/ef80c433ab1181341f7c562ac89dea471ea53208), [`bd sync` at 16:10:25](https://github.com/Dicklesworthstone/surface-dial-rust/commit/3d827f139263b79769120ad39ec7a110c3e5eb7d), [`bd sync` at 17:53:08](https://github.com/Dicklesworthstone/surface-dial-rust/commit/a1c64cca0b01b260685839853f23c205f32dcc31) -- beads issue tracker synchronization.
- [Close DIAL-7rm, DIAL-dax, DIAL-blk](https://github.com/Dicklesworthstone/surface-dial-rust/commit/3d2baf23bdd22c79d98a2770cdfe34509d8cfa0b) -- recognize pre-existing status command, config CLI, and 85 unit tests.
- [Close DIAL-m9g](https://github.com/Dicklesworthstone/surface-dial-rust/commit/4d8609f211c7070bc1d6b9ed519f51efefd55043) -- GitHub Actions workflow already existed in initial commit.
- [Close DIAL-ifk](https://github.com/Dicklesworthstone/surface-dial-rust/commit/806185e07122c1a13830c5fb5dc8488f29c70e9e) -- integration test infrastructure delivered.

---

## Test Coverage Summary

| Layer | File(s) | Count |
|-------|---------|-------|
| Config unit | `src/config.rs` | 34 |
| Daemon unit | `src/daemon.rs` | 76 |
| Input unit | `src/input/mod.rs` | 36 |
| Logging unit | `src/logging/mod.rs` | 31 |
| PID file unit | `src/pidfile.rs` | 24 |
| HID mock unit | `src/hid/mock.rs` | 10 |
| HID module unit | `src/hid/mod.rs` | 5 |
| CLI unit | `src/cli/{mod,config_cmd,status_cmd,daemon_cmd}.rs` | 22 |
| Platform unit | `src/platform/{mod,macos,linux,windows}.rs` | 19 |
| Config integration | `tests/config_integration.rs` | 17 |
| Input integration | `tests/input_integration.rs` | 19 |
| HID integration | `tests/hid_integration.rs` | 9 |
| **Rust subtotal** | | **302** |
| E2E CLI (shell) | `tests/e2e/test_cli.sh` | 15 |
| E2E Config (shell) | `tests/e2e/test_config.sh` | 17 |
| **E2E subtotal** | | **32** |
| **Grand total** | | **334** |

---

## Commit Index

Full commit history (newest first) with direct links:

| Date | Hash | Summary |
|------|------|---------|
| 2026-02-25 | [`640c932`](https://github.com/Dicklesworthstone/surface-dial-rust/commit/640c93294957fd1cd522ba78b7c683187e5bc853) | docs(AGENTS.md): add cass tool reference |
| 2026-02-22 | [`e7ddd6b`](https://github.com/Dicklesworthstone/surface-dial-rust/commit/e7ddd6b5eb2b495bf065929acf7508a4d0562303) | docs: update README license references |
| 2026-02-21 | [`1534c9b`](https://github.com/Dicklesworthstone/surface-dial-rust/commit/1534c9b2a3b3dd9d09ceacc69b8c9f5415b6e62a) | chore: update license to MIT + OpenAI/Anthropic Rider |
| 2026-02-21 | [`cb210b3`](https://github.com/Dicklesworthstone/surface-dial-rust/commit/cb210b3645551569a25b88db28efef72f10271aa) | chore: add GitHub social preview image |
| 2026-02-15 | [`4f193db`](https://github.com/Dicklesworthstone/surface-dial-rust/commit/4f193dbac85262eb1703d3da3f01e9373f29eaa9) | Add dial hardware illustration asset |
| 2026-01-27 | [`3f36497`](https://github.com/Dicklesworthstone/surface-dial-rust/commit/3f364975a578ee25bf79aeee40684481d257de7c) | Add illustration to README |
| 2026-01-27 | [`bc62cda`](https://github.com/Dicklesworthstone/surface-dial-rust/commit/bc62cdadd8cb55faff96aad2ac2dad43ca4e9e08) | Add E2E test framework |
| 2026-01-27 | [`a45e747`](https://github.com/Dicklesworthstone/surface-dial-rust/commit/a45e747ca9d28634da5f4a98f818890e72ce30d9) | Add 13 daemon lifecycle tests |
| 2026-01-27 | [`bf8a050`](https://github.com/Dicklesworthstone/surface-dial-rust/commit/bf8a050158ed9e5c5f994574ca0dc7524f5595df) | Add 14 error recovery tests |
| 2026-01-27 | [`6ddd516`](https://github.com/Dicklesworthstone/surface-dial-rust/commit/6ddd516ba1574f2f781badfa3b7d41ea106ac651) | Add 14 signal handling tests |
| 2026-01-27 | [`9fa709e`](https://github.com/Dicklesworthstone/surface-dial-rust/commit/9fa709ec6df95fe83c500e08dacd89c21bee2d81) | Add PID file management module (24 tests) |
| 2026-01-27 | [`9b10f15`](https://github.com/Dicklesworthstone/surface-dial-rust/commit/9b10f150afa18d8fde43c94e684188a10140aeab) | Add 22 logging behavior tests |
| 2026-01-27 | [`25b7103`](https://github.com/Dicklesworthstone/surface-dial-rust/commit/25b71036074b24b2652546a77f3ab8bb65f1425b) | Add 36 input module unit tests |
| 2026-01-27 | [`a1c64cc`](https://github.com/Dicklesworthstone/surface-dial-rust/commit/a1c64cca0b01b260685839853f23c205f32dcc31) | bd sync |
| 2026-01-27 | [`db1b94a`](https://github.com/Dicklesworthstone/surface-dial-rust/commit/db1b94abfd380d1a6f61ad59e93f9ccf69822112) | Expand daemon tests from 3 to 35 |
| 2026-01-27 | [`3d827f1`](https://github.com/Dicklesworthstone/surface-dial-rust/commit/3d827f139263b79769120ad39ec7a110c3e5eb7d) | bd sync |
| 2026-01-27 | [`eaeff78`](https://github.com/Dicklesworthstone/surface-dial-rust/commit/eaeff78dfd89f86e5258371f2bcb90fa3389fb84) | Add comprehensive README.md and MIT LICENSE |
| 2026-01-27 | [`ef80c43`](https://github.com/Dicklesworthstone/surface-dial-rust/commit/ef80c433ab1181341f7c562ac89dea471ea53208) | bd sync |
| 2026-01-27 | [`04eb2aa`](https://github.com/Dicklesworthstone/surface-dial-rust/commit/04eb2aa98a41dbfe3f92b2e56329689f21b5a557) | fix: click detection clarity and mic mute behavior |
| 2026-01-27 | [`5a1978b`](https://github.com/Dicklesworthstone/surface-dial-rust/commit/5a1978bd426af63a40fc3f1a11902f0bccc91a58) | fix: code issues found in review |
| 2026-01-27 | [`6bdf8fe`](https://github.com/Dicklesworthstone/surface-dial-rust/commit/6bdf8fe035d096ed44a143b609e3aefdefa535a5) | fix: MockHidDevice mutex contention |
| 2026-01-27 | [`806185e`](https://github.com/Dicklesworthstone/surface-dial-rust/commit/806185e07122c1a13830c5fb5dc8488f29c70e9e) | chore: close DIAL-ifk |
| 2026-01-27 | [`3f972da`](https://github.com/Dicklesworthstone/surface-dial-rust/commit/3f972da20c2a96e7819e26e4491dda60128d0062) | feat: integration test infrastructure |
| 2026-01-27 | [`81e80d0`](https://github.com/Dicklesworthstone/surface-dial-rust/commit/81e80d0a91d537a5ad8d00da6e0faf573cf5e414) | fix: HID permission check and banner alignment |
| 2026-01-27 | [`4d8609f`](https://github.com/Dicklesworthstone/surface-dial-rust/commit/4d8609f211c7070bc1d6b9ed519f51efefd55043) | chore: close DIAL-m9g |
| 2026-01-27 | [`804006a`](https://github.com/Dicklesworthstone/surface-dial-rust/commit/804006a521be7ee591121f9661f757e618d1dfe3) | feat: cross-platform installer |
| 2026-01-27 | [`3d2baf2`](https://github.com/Dicklesworthstone/surface-dial-rust/commit/3d2baf23bdd22c79d98a2770cdfe34509d8cfa0b) | chore: close DIAL-7rm, DIAL-dax, DIAL-blk |
| 2026-01-27 | [`d5e75ba`](https://github.com/Dicklesworthstone/surface-dial-rust/commit/d5e75bacd74fd1040e36e209fee920418d5838ad) | feat(macos): simplified installer with launchd |
| 2026-01-27 | [`ba669d5`](https://github.com/Dicklesworthstone/surface-dial-rust/commit/ba669d5f0838c990baf3a292e711f8fce2c0b1a0) | fix: GitHub Action name and error handling |
| 2026-01-27 | [`046ff5b`](https://github.com/Dicklesworthstone/surface-dial-rust/commit/046ff5bec6d3ffa82805d28e39847bbe31a7e7d9) | feat: initial commit -- full daemon |

---

## Known Limitations

- No haptic feedback (requires Windows-only Surface SDK)
- No radial menu (would need full GUI framework)
- Bluetooth can lag (OS stack issue; USB dongle recommended)
- Battery monitoring via polling only (5-minute interval; HID spec does not push)
- No per-application volume profiles yet
- Triple-click detection has a known code-level limitation: double-click fires immediately and resets state, making triple-click unreachable in the current implementation (documented in input module tests)
