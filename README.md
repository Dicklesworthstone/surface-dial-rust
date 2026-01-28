<div align="center">
  <img src="dial_illustration.webp" alt="Surface Dial Volume Controller - Turn your Microsoft Surface Dial into a premium volume knob for any OS">
</div>

# Surface Dial Volume Controller

<div align="center">

**Turn your Microsoft Surface Dial into a premium volume knob for any OS**

[![Rust](https://img.shields.io/badge/rust-1.75+-orange.svg)](https://www.rust-lang.org)
[![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Linux%20%7C%20Windows-blue.svg)](#platform-support)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)

[Installation](#installation) · [Quick Start](#quick-start) · [Configuration](#configuration) · [Commands](#commands)

</div>

---

## TL;DR

**The Problem:** The Surface Dial is a beautifully engineered $100 haptic controller that Microsoft abandoned outside of Windows—leaving macOS and Linux users with expensive paperweights.

**The Solution:** A lightweight Rust daemon that transforms your Surface Dial into a universal volume controller with intelligent click detection, adaptive acceleration, and per-application awareness.

**Why Surface Dial Volume Controller?**

| Feature | This Tool | Native Windows | Generic HID |
|---------|-----------|----------------|-------------|
| Cross-platform | macOS, Linux, Windows | Windows only | Varies |
| Volume control | Adaptive acceleration | Basic | None |
| Click patterns | Single/Double/Triple/Long | Limited | None |
| Mic control | Double-click mode | No | No |
| Media keys | Triple-click | No | No |
| Custom curves | Linear/Log/Exponential | No | No |
| Dead zone | Configurable | No | No |
| Hot reload | SIGHUP | No | No |
| Dependencies | Zero runtime deps | N/A | Varies |

---

## Quick Example

```bash
# Install and run
cargo install --path .
surface-dial

# In another terminal, check status
surface-dial status
╭─ Surface Dial Status ────────────────────────────────────────╮
│ Device: Connected (Microsoft Surface Dial)                   │
│ Volume: 65% [████████████████░░░░░░░░] (unmuted)             │
│ Mode: Volume Control                                         │
│ Battery: 78%                                                 │
╰──────────────────────────────────────────────────────────────╯

# Configure for your workflow
surface-dial config set volume.step_max 12      # Faster rotation
surface-dial config set sensitivity.invert true  # Reverse direction

# Watch live status updates
surface-dial status --watch
```

**What happens when you use it:**

| Action | Result |
|--------|--------|
| Rotate clockwise | Volume up (2-8% based on speed) |
| Rotate counter-clockwise | Volume down |
| Single click | Toggle mute |
| Double click | Switch to microphone mode (10 sec) |
| Triple click | Play/Pause media |
| Long press (1s) | Send F15 key (bind to anything) |

---

## Design Philosophy

### 1. Zero-Latency Feel

Volume control must feel instantaneous. We use adaptive acceleration that maps rotation speed to volume steps—slow rotation for precise adjustments, fast rotation to sweep across the range:

```
Rotation Speed → Volume Step
─────────────────────────────
< 80ms between ticks   → +8% (fast sweeping)
80-400ms between ticks → +2-8% (interpolated)
> 400ms between ticks  → +2% (precise control)
```

### 2. State Machine Click Detection

Multi-click detection is notoriously tricky. We use a proper state machine with three timing windows:

```
┌─────────────────────────────────────────────────────────────┐
│                    Click Detection FSM                       │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  [Idle] ──button_down──→ [Pressed] ──button_up──→ [Wait]    │
│                              │                       │      │
│                              │                       │      │
│                         (1000ms)               (400ms)      │
│                              ↓                       ↓      │
│                        LongPress              SingleClick   │
│                                                             │
│  [Wait] ──button_down──→ [Click2] ──button_up──→ DoubleClick│
│                                                             │
│  [Click2] ──button_down──→ [Click3] ──button_up──→ TripleClick│
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

Single clicks are **deferred** until the double-click window expires—preventing false positives.

### 3. Perceptual Volume Curves

Human hearing is logarithmic. A linear volume slider feels wrong—50% sounds nearly full. We default to a logarithmic curve that matches perception:

```
Volume Curves
─────────────
         │                          ╱
         │                        ╱
Perceived│                     ╱╱     ← Logarithmic (default)
Volume   │                  ╱╱╱
         │              ╱╱╱╱
         │         ╱╱╱╱╱
         │    ╱╱╱╱╱╱.................. ← Linear
         │╱╱╱╱
         └────────────────────────────
                  Dial Position
```

### 4. Platform Abstraction Without FFI

Instead of complex native bindings, we use platform command-line tools:
- **macOS:** `osascript` (AppleScript) for volume, System Events for keys
- **Linux:** `wpctl` (PipeWire) or `pactl` (PulseAudio) for audio
- **Windows:** Native audio APIs

This means zero native dependencies, easy cross-compilation, and debuggable subprocess calls.

### 5. Configuration as Data

All 52 configuration options are validated with ranges and cross-field constraints:

```rust
// Example: These constraints are enforced
double_click_ms < triple_click_ms < long_press_ms
step_min <= step_max
0.1 <= multiplier <= 5.0
```

---

## Comparison vs Alternatives

| Feature | surface-dial | SteerMouse | Karabiner | USB Overdrive |
|---------|--------------|------------|-----------|---------------|
| Surface Dial support | Native | Partial | No | No |
| Volume control | Adaptive | Basic | Manual | Basic |
| Click patterns | 4 types | 2 types | Many | 2 types |
| Config file | TOML | Plist | JSON | Plist |
| Open source | Yes | No | Yes | No |
| Price | Free | $20 | Free | $20 |
| Cross-platform | Yes | macOS | macOS | macOS |
| Daemon mode | Yes | Yes | Yes | Yes |
| CLI interface | Full | None | None | None |

---

## Installation

### From Source (Recommended)

```bash
# Clone and build
git clone https://github.com/yourusername/surface-dial-rust
cd surface-dial-rust
cargo build --release

# Install to ~/.cargo/bin
cargo install --path .

# Verify installation
surface-dial --version
```

### Prerequisites

**macOS:**
```bash
# Grant Accessibility permissions (System Preferences → Security & Privacy)
# Required for key simulation
```

**Linux:**
```bash
# Install HID libraries
sudo apt install libhidapi-dev  # Debian/Ubuntu
sudo pacman -S hidapi           # Arch

# Add udev rule for non-root access
echo 'SUBSYSTEM=="hidraw", ATTRS{idVendor}=="045e", ATTRS{idProduct}=="091b", MODE="0666"' | \
  sudo tee /etc/udev/rules.d/99-surface-dial.rules
sudo udevadm control --reload-rules
```

### Run as Service

**macOS (launchd):**
```bash
# Install LaunchAgent
cat > ~/Library/LaunchAgents/com.surface-dial.daemon.plist << 'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.surface-dial.daemon</string>
    <key>ProgramArguments</key>
    <array>
        <string>/Users/YOUR_USERNAME/.cargo/bin/surface-dial</string>
        <string>daemon</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
</dict>
</plist>
EOF

# Load the service
launchctl load ~/Library/LaunchAgents/com.surface-dial.daemon.plist
```

**Linux (systemd):**
```bash
# Create user service
mkdir -p ~/.config/systemd/user
cat > ~/.config/systemd/user/surface-dial.service << 'EOF'
[Unit]
Description=Surface Dial Volume Controller
After=graphical-session.target

[Service]
ExecStart=%h/.cargo/bin/surface-dial daemon
Restart=always

[Install]
WantedBy=default.target
EOF

# Enable and start
systemctl --user enable surface-dial
systemctl --user start surface-dial
```

---

## Quick Start

### 1. Connect Your Surface Dial

Pair via Bluetooth or plug in the USB dongle. The LED should pulse.

### 2. Run the Daemon

```bash
# Foreground (see logs)
surface-dial daemon --foreground

# Background (normal usage)
surface-dial daemon
```

### 3. Test Controls

| Action | Expected Result |
|--------|-----------------|
| Rotate right | System volume increases |
| Rotate left | System volume decreases |
| Click once | Volume mutes/unmutes |
| Double-click | "Mic mode" message, rotation controls mic |
| Triple-click | Media play/pause |

### 4. Customize

```bash
# See all settings
surface-dial config show

# Common customizations
surface-dial config set volume.step_max 10        # Bigger steps
surface-dial config set interaction.long_press_ms 500  # Faster long-press
surface-dial config set sensitivity.invert true   # Reverse direction
```

---

## Commands

### `surface-dial daemon`

Run the main daemon process.

```bash
surface-dial daemon [OPTIONS]

Options:
  -c, --config <FILE>    Use custom config file
  --foreground           Run in foreground (don't daemonize)
  --log-level <LEVEL>    Override log level [error|warn|info|debug|trace]
  --no-log-file          Disable file logging

Examples:
  surface-dial daemon --foreground --log-level debug
  surface-dial daemon --config ~/my-config.toml
```

### `surface-dial status`

Check daemon and device status.

```bash
surface-dial status [OPTIONS]

Options:
  -d, --detailed    Show extended information
  -c, --check       Exit with code 0 if running, 1 if not
  -w, --watch       Continuous updates (1/sec)
  --json            Output as JSON

Examples:
  surface-dial status --watch
  surface-dial status --json | jq '.volume'
```

### `surface-dial config`

Manage configuration.

```bash
surface-dial config <SUBCOMMAND>

Subcommands:
  show              Display all settings
  get <KEY>         Get single value
  set <KEY> <VAL>   Set single value
  reset             Reset to defaults
  path              Show config file location

Examples:
  surface-dial config show
  surface-dial config get volume.step_max
  surface-dial config set volume.curve logarithmic
  surface-dial config reset --section volume --force
```

---

## Configuration

### Config File Location

| Platform | Path |
|----------|------|
| macOS | `~/Library/Application Support/surface-dial/config.toml` |
| Linux | `~/.config/surface-dial/config.toml` |
| Windows | `%APPDATA%\surface-dial\config.toml` |

### Full Configuration Reference

```toml
# =============================================================================
# SURFACE DIAL CONFIGURATION
# =============================================================================
# All values shown are defaults. Delete any line to use the default.

# -----------------------------------------------------------------------------
# Volume Control
# -----------------------------------------------------------------------------
[volume]
step_min = 2              # Minimum step for slow rotation (1-20)
step_max = 8              # Maximum step for fast rotation (1-20)
curve = "logarithmic"     # Volume curve: linear | logarithmic | exponential
curve_steepness = 2.0     # Curve steepness (1.0-20.0, higher = steeper)

# -----------------------------------------------------------------------------
# Microphone Control (activated by double-click)
# -----------------------------------------------------------------------------
[microphone]
step_min = 3              # Minimum mic step (1-20)
step_max = 10             # Maximum mic step (1-20)
mode_duration = 10        # Seconds in mic mode before auto-return (1-60)
curve = "linear"          # Mic curve (linear recommended for voice)
curve_steepness = 2.0

# -----------------------------------------------------------------------------
# Acceleration (maps rotation speed to step size)
# -----------------------------------------------------------------------------
[acceleration]
fast_ms = 80              # Below this = max step (10-500ms)
slow_ms = 400             # Above this = min step (100-2000ms)

# -----------------------------------------------------------------------------
# Interaction Timing
# -----------------------------------------------------------------------------
[interaction]
double_click_ms = 400     # Max gap for double-click (100-800ms)
triple_click_ms = 600     # Max gap for triple-click (200-1000ms)
long_press_ms = 1000      # Hold duration for long-press (500-3000ms)

# -----------------------------------------------------------------------------
# Sensitivity
# -----------------------------------------------------------------------------
[sensitivity]
dead_zone = 0             # Ignore rotations smaller than this (0-10)
multiplier = 1.0          # Scale all rotation (0.1-5.0)
invert = false            # Reverse rotation direction
preset = "default"        # Preset: default | accessibility | precision | fast

# -----------------------------------------------------------------------------
# On-Screen Display
# -----------------------------------------------------------------------------
[osd]
enabled = true
position = "center-bottom"  # center-bottom | center | top-right | bottom-left
size = "medium"             # small | medium | large
timeout_ms = 1500           # How long OSD stays visible
opacity = 0.9               # OSD transparency (0.0-1.0)
show_icon = true
show_percentage = true

# -----------------------------------------------------------------------------
# Battery Monitoring
# -----------------------------------------------------------------------------
[battery]
enabled = true
poll_interval_seconds = 300
show_in_status = true
show_in_osd = true
warning_thresholds = [20, 10, 5]  # Low battery notifications

# -----------------------------------------------------------------------------
# Media Control
# -----------------------------------------------------------------------------
[media_control]
enabled = true
triple_click_action = "play_pause"  # play_pause | next_track | prev_track

# -----------------------------------------------------------------------------
# Audio Feedback
# -----------------------------------------------------------------------------
[audio_feedback]
enabled = false           # Play sounds on actions
volume = 0.3              # Feedback volume (0.0-1.0)
tick = true               # Sound on rotation
boundary = true           # Sound at 0% and 100%
mode_change = true        # Sound when switching modes
mute = true               # Sound when muting

# -----------------------------------------------------------------------------
# Event Hooks
# -----------------------------------------------------------------------------
[events]
enabled = false
debounce_ms = 500
scripts_dir = "~/.config/surface-dial/scripts"

# -----------------------------------------------------------------------------
# Daemon Settings
# -----------------------------------------------------------------------------
[daemon]
log_level = "info"        # error | warn | info | debug | trace
log_file_enabled = true
log_max_size_mb = 10      # Max log file size before rotation
log_keep_files = 3        # Number of rotated logs to keep
log_json = false          # Output logs as JSON
```

### Sensitivity Presets

| Preset | Dead Zone | Multiplier | Use Case |
|--------|-----------|------------|----------|
| `default` | 0 | 1.0 | Normal use |
| `accessibility` | 3 | 0.7 | Reduced sensitivity for tremor |
| `precision` | 1 | 0.5 | Fine audio mixing |
| `fast` | 0 | 2.0 | Quick adjustments |

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                         SURFACE DIAL DAEMON                          │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  ┌──────────────┐     ┌──────────────┐     ┌──────────────┐        │
│  │   HID Layer  │     │  Input Layer │     │Platform Layer│        │
│  ├──────────────┤     ├──────────────┤     ├──────────────┤        │
│  │              │     │              │     │              │        │
│  │  hidapi      │────▶│ ClickDetector│────▶│   macOS      │        │
│  │  DialReport  │     │ RotationProc │     │   Linux      │        │
│  │  Mock Device │     │ Acceleration │     │   Windows    │        │
│  │              │     │              │     │   Mock       │        │
│  └──────────────┘     └──────────────┘     └──────────────┘        │
│         │                    │                    │                 │
│         │                    │                    │                 │
│         ▼                    ▼                    ▼                 │
│  ┌─────────────────────────────────────────────────────────┐       │
│  │                      DAEMON CORE                         │       │
│  ├─────────────────────────────────────────────────────────┤       │
│  │  • Event loop (10ms tick)                               │       │
│  │  • Mode management (volume/mic)                         │       │
│  │  • Volume calculation with curves                       │       │
│  │  • Graceful shutdown (SIGTERM/SIGINT)                   │       │
│  └─────────────────────────────────────────────────────────┘       │
│         │                    │                    │                 │
│         ▼                    ▼                    ▼                 │
│  ┌──────────────┐     ┌──────────────┐     ┌──────────────┐        │
│  │    Config    │     │   Logging    │     │     CLI      │        │
│  ├──────────────┤     ├──────────────┤     ├──────────────┤        │
│  │  TOML file   │     │  Rotating    │     │  status      │        │
│  │  52 options  │     │  JSON/text   │     │  config      │        │
│  │  Validation  │     │  Dual output │     │  daemon      │        │
│  └──────────────┘     └──────────────┘     └──────────────┘        │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

### Data Flow

```
Surface Dial (USB/Bluetooth)
        │
        ▼
┌───────────────────┐
│ HID Report (3 bytes)     │
│ [0x01][button][rotation] │
└───────────────────┘
        │
        ▼
┌───────────────────┐
│ DialReport::parse()      │
│ → button_pressed: bool   │
│ → rotation: i8 (-127..127)│
└───────────────────┘
        │
        ├─────────────────────────────┐
        ▼                             ▼
┌───────────────────┐     ┌───────────────────┐
│ ClickDetector            │     │ RotationProcessor        │
│ → Single/Double/Triple   │     │ → Dead zone filter       │
│ → Long press             │     │ → Multiplier             │
│ → State machine          │     │ → Inversion              │
└───────────────────┘     └───────────────────┘
        │                             │
        ▼                             ▼
┌───────────────────────────────────────────┐
│              calculate_step()                      │
│  elapsed_ms → interpolate(min_step, max_step)     │
└───────────────────────────────────────────┘
        │
        ▼
┌───────────────────┐
│ Platform::set_volume()   │
│ → osascript (macOS)      │
│ → wpctl/pactl (Linux)    │
└───────────────────┘
```

---

## Troubleshooting

### Device Not Found

```
Error: Surface Dial not found (VID:045E PID:091B)
```

**Fixes:**
1. Check Bluetooth pairing (re-pair if needed)
2. Try the USB dongle instead of Bluetooth
3. Linux: Check udev rules are installed
4. Linux: Run `sudo surface-dial` to test permissions

### Permission Denied (macOS)

```
Error: Failed to send key event
```

**Fix:** Grant Accessibility permissions:
1. System Preferences → Security & Privacy → Privacy
2. Select "Accessibility" in sidebar
3. Add `surface-dial` (or Terminal if running from there)

### Permission Denied (Linux)

```
Error: Failed to open HID device
```

**Fix:** Add udev rule and reload:
```bash
echo 'SUBSYSTEM=="hidraw", ATTRS{idVendor}=="045e", ATTRS{idProduct}=="091b", MODE="0666"' | \
  sudo tee /etc/udev/rules.d/99-surface-dial.rules
sudo udevadm control --reload-rules
# Unplug and replug the device
```

### Volume Not Changing

```
Rotation detected but volume unchanged
```

**Fixes:**
1. Check `surface-dial status` shows correct current volume
2. Verify audio output device is correct
3. macOS: Check System Preferences → Sound → Output
4. Linux: Check `wpctl status` or `pactl info`

### Double-Click Not Working

**Fix:** Adjust timing in config:
```bash
# If you click slowly
surface-dial config set interaction.double_click_ms 500

# If you click fast
surface-dial config set interaction.double_click_ms 300
```

---

## Limitations

**Honest about what this tool doesn't do:**

| Limitation | Reason | Workaround |
|------------|--------|------------|
| No haptic feedback | Requires Windows-only Surface SDK | Audio feedback option |
| No radial menu | Would need full GUI framework | Use long-press + external app |
| Bluetooth can lag | OS Bluetooth stack issue | Use USB dongle |
| Battery via polling | HID spec doesn't push battery | 5-minute poll interval |
| No per-app profiles | Complexity vs. value | Manual config switching |

---

## FAQ

**Q: Does this work with Surface Dial 2?**
A: Yes, both Surface Dial generations use the same HID protocol.

**Q: Can I use multiple Surface Dials?**
A: Currently only the first detected device is used. Multi-device support is planned.

**Q: Why AppleScript instead of native APIs?**
A: AppleScript has zero dependencies, works across all macOS versions, and is trivially debuggable. The ~5ms latency is imperceptible for volume control.

**Q: Can I control app-specific volume (e.g., just Spotify)?**
A: Not yet. Per-application volume control is on the roadmap.

**Q: How do I bind the long-press F15 key to something useful?**
A: Use your OS keyboard shortcuts:
- macOS: System Preferences → Keyboard → Shortcuts
- Linux: Your DE's keyboard settings
- Popular bindings: Screenshot, app launcher, play/pause

**Q: Why is microphone mode time-limited?**
A: To prevent accidentally leaving it in mic mode. The duration is configurable (1-60 seconds).

**Q: Does it work over Remote Desktop?**
A: The dial must be connected to the machine running the daemon. USB passthrough may work with some RDP clients.

---

## About Contributions

Please don't take this the wrong way, but I do not accept outside contributions for any of my projects. I simply don't have the mental bandwidth to review anything, and it's my name on the thing, so I'm responsible for any problems it causes; thus, the risk-reward is highly asymmetric from my perspective. I'd also have to worry about other "stakeholders," which seems unwise for tools I mostly make for myself for free. Feel free to submit issues, and even PRs if you want to illustrate a proposed fix, but know I won't merge them directly. Instead, I'll have Claude or Codex review submissions via `gh` and independently decide whether and how to address them. Bug reports in particular are welcome. Sorry if this offends, but I want to avoid wasted time and hurt feelings. I understand this isn't in sync with the prevailing open-source ethos that seeks community contributions, but it's the only way I can move at this velocity and keep my sanity.

---

## License

MIT License - See [LICENSE](LICENSE) for details.

---

<div align="center">

**[Back to Top](#surface-dial-volume-controller)**

</div>
