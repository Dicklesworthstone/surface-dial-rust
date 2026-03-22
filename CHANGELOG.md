# Changelog

All notable changes to Surface Dial Volume Controller are documented here.

This project has no tagged releases yet (current version: `0.1.0`). Entries are organized by date and capability area. All links point to the live repository at [Dicklesworthstone/surface-dial-rust](https://github.com/Dicklesworthstone/surface-dial-rust).

---

## [Unreleased] — 0.1.0-dev

### Project Overview

A Rust daemon that transforms the Microsoft Surface Dial (VID `045E`, PID `091B`) into a universal volume controller with adaptive acceleration, multi-click detection, and cross-platform support (macOS, Linux, Windows). 322 tests across unit, integration, and E2E layers. ~16k lines of Rust.

---

### 2026-02-25

#### Documentation

- Add [cass](https://github.com/Dicklesworthstone/surface-dial-rust/commit/640c93294957fd1cd522ba78b7c683187e5bc853) (Cross-Agent Session Search) tool reference to AGENTS.md for cross-session knowledge reuse across Claude Code, Codex, Cursor, Gemini, and ChatGPT agents.

---

### 2026-02-21 — 2026-02-22

#### License

- [Update license](https://github.com/Dicklesworthstone/surface-dial-rust/commit/1534c9b2a3b3dd9d09ceacc69b8c9f5415b6e62a) from plain MIT to MIT with OpenAI/Anthropic Rider (restricts use by OpenAI, Anthropic, and affiliates without express written permission from Jeffrey Emanuel).
- [Update README](https://github.com/Dicklesworthstone/surface-dial-rust/commit/e7ddd6b5eb2b495bf065929acf7508a4d0562303) license references to match.

#### Assets

- [Add GitHub social preview image](https://github.com/Dicklesworthstone/surface-dial-rust/commit/cb210b3645551569a25b88db28efef72f10271aa) (1280x640) for repository link previews.

---

### 2026-02-15

#### Assets

- [Add dial hardware illustration](https://github.com/Dicklesworthstone/surface-dial-rust/commit/4f193dbac85262eb1703d3da3f01e9373f29eaa9) PNG asset previously missing from the repository, completing the README visuals.

---

### 2026-01-27 — Initial Development Day

The entire functional codebase was built on this date across 25 commits. Organized below by capability rather than strict chronological order.

#### Core Daemon — HID Polling, Volume Control, Click Detection

- [**Initial commit**](https://github.com/Dicklesworthstone/surface-dial-rust/commit/046ff5bec6d3ffa82805d28e39847bbe31a7e7d9) — 8,311 lines across 38 files. Full Surface Dial volume controller daemon with:
  - HID device polling for Surface Dial (`0x045E:0x091B`)
  - Adaptive acceleration mapping rotation speed to volume step size (2-8%)
  - State-machine click detection: single (mute), double (mic mode), triple (media play/pause), long-press (F15 key)
  - Perceptual volume curves: linear, logarithmic, exponential
  - 52-key TOML configuration system with validation and cross-field constraints
  - Rotating file logger with JSON mode, dual console/file output
  - CLI with `daemon`, `status` (including `--watch` and `--json`), and `config` (get/set/reset/show/path) subcommands
  - Platform abstraction: macOS via `osascript`/AppleScript, Linux via `wpctl`/`pactl`, Windows via PowerShell + COM (stubs)
  - 8 debug/diagnostic binaries (`debug_dial`, `debug_events`, `dial_reader`, `event_tap`, `hid_blocking`, `hid_direct`, `hid_poll`, `list_devices`)
  - GitHub Actions release workflow for 5 cross-platform targets
  - Unified `install.sh` with platform auto-detection

#### Bug Fixes

- [Fix GitHub Action name and cross-compilation](https://github.com/Dicklesworthstone/surface-dial-rust/commit/ba669d5f0838c990baf3a292e711f8fce2c0b1a0) — correct `dtolnay/rust-action` to `dtolnay/rust-toolchain`, add ARM64 linker config, validate unsupported CPU architectures.
- [Fix HID permission check and banner alignment](https://github.com/Dicklesworthstone/surface-dial-rust/commit/81e80d0a91d537a5ad8d00da6e0faf573cf5e414) in `install.sh` — the previous `xargs` approach silently succeeded when no `/dev/hidraw*` devices existed; now uses explicit loop. Fixed misaligned box-drawing characters caused by variable-length platform strings.
- [Fix MockHidDevice mutex contention](https://github.com/Dicklesworthstone/surface-dial-rust/commit/6bdf8fe035d096ed44a143b609e3aefdefa535a5) — release mutex before sleeping in `read_timeout`; prevent event loss across disconnect/reconnect cycles by only popping events when connected.
- [Fix code issues from review](https://github.com/Dicklesworthstone/surface-dial-rust/commit/5a1978bd426af63a40fc3f1a11902f0bccc91a58) — add `prev_track` to valid media actions (was being rejected despite daemon support), add buffer bounds check in `get_feature_report()` to prevent panics, remove unreachable code in log rotation, remove unused import.
- [Fix click detection and mic mute behavior](https://github.com/Dicklesworthstone/surface-dial-rust/commit/04eb2aa98a41dbfe3f92b2e56329689f21b5a557) — remove redundant double-click time check that always passed; fix `toggle_mic_mute` on macOS to remember previous volume before muting (was hardcoded to restore 50%) using `AtomicI32` for thread-safe state tracking.

#### Installer — Cross-Platform

- [Rewrite macOS installer](https://github.com/Dicklesworthstone/surface-dial-rust/commit/d5e75bacd74fd1040e36e209fee920418d5838ad) — no-sudo installation to `~/.local/bin`, dynamic plist generation, ad-hoc signing for Apple Silicon, `--uninstall` flag.
- [Complete cross-platform installer](https://github.com/Dicklesworthstone/surface-dial-rust/commit/804006a521be7ee591121f9661f757e618d1dfe3) — Linux support (systemd user service, udev rule guidance, journald logging, `enable-linger`), Windows support (Task Scheduler via PowerShell, toast notifications), unified installer with colored output, `curl` one-liner support, and environment variables (`DEST`, `NO_AUTOSTART`, `VERSION`).

#### Integration Test Infrastructure

- [Add HID device abstraction and mock platform](https://github.com/Dicklesworthstone/surface-dial-rust/commit/3f972da20c2a96e7819e26e4491dda60128d0062) — `HidDevice` trait with `RealHidDevice` wrapper and `MockHidDevice` (event queuing), `MockPlatform` recording all audio/key operations, 45+ integration tests across `hid_integration.rs`, `input_integration.rs`, `config_integration.rs`. Enables full testing without physical hardware.

#### Unit Tests — Input Module

- [Add 36 unit tests to input module](https://github.com/Dicklesworthstone/surface-dial-rust/commit/25b71036074b24b2652546a77f3ab8bb65f1425b) — tests for `calculate_step` boundary conditions and interpolation, `RotationProcessor` accumulation/multiplier/extremes, `ClickDetector` state transitions and config updates, `ClickConfig`/`SensitivityConfig` defaults. Documents a known limitation: triple-click is unreachable because double-click fires immediately and resets state.

#### Unit Tests — Logging Module

- [Add 22 logging behavior tests](https://github.com/Dicklesworthstone/surface-dial-rust/commit/9b10f150afa18d8fde43c94e684188a10140aeab) — file creation with parent directories, append mode, size tracking, plain text and JSON entry formats, TTY detection with color codes, log level filtering (console/file/combined), file rotation with oldest-file deletion, structured events (startup, shutdown, error), integration workflow simulation.

#### PID File Management

- [Add pidfile module with 24 tests](https://github.com/Dicklesworthstone/surface-dial-rust/commit/9fa709ec6df95fe83c500e08dacd89c21bee2d81) — PID file creation with automatic parent directory creation, stale PID detection and cleanup, process liveness checking (Unix `kill -0`), automatic file cleanup on drop, persistent mode for testing, Unix permissions (0644), duplicate instance prevention (`AlreadyRunning` error), crash recovery.

#### Unit Tests — Daemon Module

- [Expand daemon tests from 3 to 35](https://github.com/Dicklesworthstone/surface-dial-rust/commit/db1b94abfd380d1a6f61ad59e93f9ccf69822112) (11.7x increase, full suite 116 to 143 tests) — control mode equality/debug/clone, `DaemonStats` defaults, daemon creation and initial state, running flag and cross-thread shutdown, mode switching (volume/mic with rotation processor reset), config reload with state preservation, click result processing (single/double/triple), mic mode expiry, button state tracking, integration-style sequences (full double-click, triple-click), USB constant verification.
- [Add 14 signal handling tests](https://github.com/Dicklesworthstone/surface-dial-rust/commit/6ddd516ba1574f2f781badfa3b7d41ea106ac651) — SIGTERM/SIGINT graceful shutdown, SIGHUP config reload, cross-thread signal delivery, idempotent multiple shutdown signals, stats preservation, processor config updates on reload.
- [Add 14 error recovery tests](https://github.com/Dicklesworthstone/surface-dial-rust/commit/bf8a050158ed9e5c5f994574ca0dc7524f5595df) — device disconnect flag behavior, state preservation through disconnect/reconnect cycles, mode preservation, button state on mid-press disconnect, reconnect processing, stats integrity, config reload during error state, mic mode timer during disconnect, many-cycle recovery.
- [Add 13 daemon lifecycle tests](https://github.com/Dicklesworthstone/surface-dial-rust/commit/a45e747ca9d28634da5f4a98f818890e72ce30d9) — start/stop/restart lifecycle, atomic compare-and-swap single-instance prevention, config persistence throughout lifecycle, shutdown from any state, uptime calculation, processing stop after shutdown signal.

#### E2E Test Framework

- [Add E2E test framework](https://github.com/Dicklesworthstone/surface-dial-rust/commit/bc62cdadd8cb55faff96aad2ac2dad43ca4e9e08) with shell-based logging infrastructure — `common.sh` with colored output (DEBUG/INFO/STEP/PASS/FAIL/WARN), 20+ assertion functions, automatic cleanup on exit, summary reporting; `test_cli.sh` (14 tests for version, help, config get/set/reset/list, status, exit codes); `test_config.sh` (17 tests for defaults, persistence, validation); `run_all.sh` test runner with automatic discovery and selective execution.

#### Documentation

- [Add comprehensive README and MIT LICENSE](https://github.com/Dicklesworthstone/surface-dial-rust/commit/eaeff78dfd89f86e5258371f2bcb90fa3389fb84) — 717-line README with TL;DR, feature comparison table, design philosophy (5 core principles), architecture diagrams, complete CLI reference, full 52-key configuration reference, troubleshooting guide, limitations, FAQ, contribution policy.
- [Add illustration to README](https://github.com/Dicklesworthstone/surface-dial-rust/commit/3f364975a578ee25bf79aeee40684481d257de7c) — convert `dial_illustration.png` (1.6 MB) to webp (116 KB), replace ASCII art hero section with centered image.

#### Beads Syncs (metadata-only)

- [`bd sync` at 16:00:17](https://github.com/Dicklesworthstone/surface-dial-rust/commit/ef80c433ab1181341f7c562ac89dea471ea53208), [`bd sync` at 16:10:25](https://github.com/Dicklesworthstone/surface-dial-rust/commit/3d827f139263b79769120ad39ec7a110c3e5eb7d), [`bd sync` at 17:53:08](https://github.com/Dicklesworthstone/surface-dial-rust/commit/a1c64cca0b01b260685839853f23c205f32dcc31) — beads issue tracker synchronization (no functional changes).

#### Beads Closures (metadata-only)

- [Close DIAL-7rm, DIAL-dax, DIAL-blk](https://github.com/Dicklesworthstone/surface-dial-rust/commit/3d2baf23bdd22c79d98a2770cdfe34509d8cfa0b) — recognize pre-existing status command, config CLI, and 85 unit tests.
- [Close DIAL-m9g](https://github.com/Dicklesworthstone/surface-dial-rust/commit/4d8609f211c7070bc1d6b9ed519f51efefd55043) — GitHub Actions workflow already created in initial commit.
- [Close DIAL-ifk](https://github.com/Dicklesworthstone/surface-dial-rust/commit/806185e07122c1a13830c5fb5dc8488f29c70e9e) — integration test infrastructure delivered.

---

## Test Coverage Summary

| Layer | File(s) | Test Count |
|-------|---------|------------|
| Config unit tests | `src/config.rs` | 34 |
| Input unit tests | `src/input/mod.rs` | 36 |
| Logging unit tests | `src/logging/mod.rs` | 31 |
| Daemon unit tests | `src/daemon.rs` | 76 |
| PID file unit tests | `src/pidfile.rs` | 24 |
| HID mock unit tests | `src/hid/mock.rs` | 10 |
| HID module unit tests | `src/hid/mod.rs` | 5 |
| Platform unit tests | `src/platform/*.rs` | 30 |
| CLI unit tests | `src/cli/*.rs` | 22 |
| HID integration tests | `tests/hid_integration.rs` | 9 |
| Input integration tests | `tests/input_integration.rs` | 19 |
| Config integration tests | `tests/config_integration.rs` | 17 |
| E2E tests (shell) | `tests/e2e/test_cli.sh`, `tests/e2e/test_config.sh` | 31 |
| **Total** | | **344** |

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
| 2026-01-27 | [`bc62cda`](https://github.com/Dicklesworthstone/surface-dial-rust/commit/bc62cdadd8cb55faff96aad2ac2dad43ca4e9e08) | Add E2E test framework with logging infrastructure |
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
| 2026-01-27 | [`046ff5b`](https://github.com/Dicklesworthstone/surface-dial-rust/commit/046ff5bec6d3ffa82805d28e39847bbe31a7e7d9) | feat: initial commit — full daemon |

---

## Known Limitations (from README)

- No haptic feedback (requires Windows-only Surface SDK)
- No radial menu (would need full GUI framework)
- Bluetooth can lag (OS stack issue; USB dongle recommended)
- Battery via polling only (5-minute interval; HID spec does not push)
- No per-app volume profiles yet
- Triple-click detection has a known code-level limitation where double-click fires immediately and resets state (documented in input module tests)
