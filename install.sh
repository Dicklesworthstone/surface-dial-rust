#!/usr/bin/env bash
#
# Surface Dial Volume Controller Installer
#
# One-line install:
#   curl -fsSL https://raw.githubusercontent.com/USER/surface-dial/main/install.sh | bash
#
# Options:
#   DEST=/path        Install directory (default: ~/.local/bin)
#   NO_AUTOSTART=1    Skip auto-start setup
#   VERSION=x.y.z     Install specific version (default: build from source)
#
set -euo pipefail

# Determine script directory (empty if piped from curl)
if [[ -n "${BASH_SOURCE[0]:-}" ]] && [[ -f "${BASH_SOURCE[0]}" ]]; then
    SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
else
    SCRIPT_DIR=""
fi

VERSION="1.0.0"
REPO="USER/surface-dial"

# =============================================================================
# Colors and Logging (DIAL-l4i giil-style)
# =============================================================================

# Check if we're in a terminal that supports colors
if [[ -t 1 ]] && [[ "${TERM:-}" != "dumb" ]]; then
    RED='\033[0;31m'
    GREEN='\033[0;32m'
    YELLOW='\033[1;33m'
    BLUE='\033[0;34m'
    CYAN='\033[0;36m'
    BOLD='\033[1m'
    NC='\033[0m'
else
    RED=''
    GREEN=''
    YELLOW=''
    BLUE=''
    CYAN=''
    BOLD=''
    NC=''
fi

log_info()  { echo -e "${GREEN}[installer]${NC} $1"; }
log_warn()  { echo -e "${YELLOW}[installer]${NC} $1"; }
log_error() { echo -e "${RED}[installer]${NC} $1"; }
log_step()  { echo -e "${BLUE}==>${NC} ${BOLD}$1${NC}"; }

# =============================================================================
# Platform Detection Module (DIAL-y5b)
# =============================================================================

detect_os() {
    case "$(uname -s)" in
        Darwin)              echo "macos" ;;
        Linux)               echo "linux" ;;
        MINGW*|MSYS*|CYGWIN*) echo "windows" ;;
        *)                   echo "unknown" ;;
    esac
}

detect_arch() {
    case "$(uname -m)" in
        x86_64|amd64)   echo "x86_64" ;;
        aarch64|arm64)  echo "aarch64" ;;
        armv7l)         echo "armv7" ;;
        *)              echo "unknown" ;;
    esac
}

detect_platform() {
    local os arch
    os="$(detect_os)"
    arch="$(detect_arch)"
    echo "${os}_${arch}"
}

# Binary name for current platform
get_binary_name() {
    local os arch
    os="$(detect_os)"
    arch="$(detect_arch)"

    if [[ "$os" == "windows" ]]; then
        echo "surface-dial-${os}-${arch}.exe"
    else
        echo "surface-dial-${os}-${arch}"
    fi
}

# =============================================================================
# Common Functions
# =============================================================================

print_header() {
    local platform
    platform="$(detect_platform)"
    echo
    echo -e "${CYAN}╔══════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${CYAN}║${NC}  ${BOLD}Surface Dial Volume Controller${NC}                              ${CYAN}║${NC}"
    printf "${CYAN}║${NC}  Version: %-6s  •  Platform: %-16s          ${CYAN}║${NC}\n" "${VERSION}" "${platform}"
    echo -e "${CYAN}╚══════════════════════════════════════════════════════════════╝${NC}"
    echo
}

print_usage() {
    cat << EOF
${BOLD}Surface Dial Volume Controller Installer${NC}

${BOLD}Quick Install:${NC}
  curl -fsSL https://raw.githubusercontent.com/${REPO}/main/install.sh | bash

${BOLD}Usage:${NC} $0 [OPTIONS]

${BOLD}Options:${NC}
  --detect-only     Print detected platform and exit
  --from-release    Download pre-built binary from GitHub releases
  --build-local     Build from source (requires Rust toolchain)
  --uninstall       Remove Surface Dial and its service
  --help            Show this help message

${BOLD}Environment Variables:${NC}
  DEST=/path        Install directory (default: ~/.local/bin)
  NO_AUTOSTART=1    Skip auto-start service setup
  VERSION=x.y.z     Install specific version

${BOLD}Examples:${NC}
  $0                     # Auto-detect: build if cargo available
  $0 --from-release      # Download pre-built binary
  $0 --build-local       # Build from source
  $0 --uninstall         # Remove installation
  DEST=/opt $0           # Install to /opt
EOF
}

check_rust_toolchain() {
    if command -v cargo &>/dev/null; then
        return 0
    else
        return 1
    fi
}

build_from_source() {
    log_step "Building release binary..."
    if [[ -z "$SCRIPT_DIR" ]]; then
        log_error "Cannot build from source when running via curl."
        log_info "Clone the repo first: git clone https://github.com/${REPO}"
        exit 1
    fi
    cd "$SCRIPT_DIR"
    cargo build --release
    log_info "Build complete."
}

# =============================================================================
# macOS Installation (DIAL-fas)
# =============================================================================

generate_macos_plist() {
    local binary_path="$1"
    local log_dir="$2"
    cat << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.surface-dial</string>

    <key>ProgramArguments</key>
    <array>
        <string>${binary_path}</string>
        <string>daemon</string>
    </array>

    <key>RunAtLoad</key>
    <true/>

    <key>KeepAlive</key>
    <dict>
        <key>SuccessfulExit</key>
        <false/>
    </dict>

    <key>StandardOutPath</key>
    <string>${log_dir}/stdout.log</string>

    <key>StandardErrorPath</key>
    <string>${log_dir}/stderr.log</string>

    <key>ProcessType</key>
    <string>Background</string>
</dict>
</plist>
EOF
}

install_macos() {
    local install_dir="${HOME}/.local/bin"
    local plist_dir="${HOME}/Library/LaunchAgents"
    local plist_file="${plist_dir}/com.surface-dial.plist"
    local log_dir="${HOME}/.local/share/surface-dial"
    local binary_path="${install_dir}/surface-dial"

    echo "Installing for macOS..."
    echo

    # Build or download binary
    if [[ "${INSTALL_MODE:-auto}" == "build" ]] || { [[ "${INSTALL_MODE:-auto}" == "auto" ]] && check_rust_toolchain; }; then
        build_from_source
        local src_binary="${SCRIPT_DIR}/target/release/surface-dial"
    else
        echo "Error: Pre-built release download not yet implemented."
        echo "Please install Rust and run with --build-local"
        exit 1
    fi

    # Stop existing service (if any)
    echo "Stopping existing service (if any)..."
    launchctl unload "$plist_file" 2>/dev/null || true
    pkill -f "surface-dial" 2>/dev/null || true
    sleep 1

    # Create directories
    echo "Creating directories..."
    mkdir -p "$install_dir" "$plist_dir" "$log_dir"

    # Install binary
    echo "Installing binary to: ${binary_path}"
    cp "$src_binary" "$binary_path"
    chmod +x "$binary_path"

    # Ad-hoc sign the binary (required for Apple Silicon)
    echo "Signing binary..."
    codesign --force --sign - "$binary_path" 2>/dev/null || true

    # Generate and install plist
    echo "Installing LaunchAgent..."
    generate_macos_plist "$binary_path" "$log_dir" > "$plist_file"

    # Permissions notice
    echo
    echo "┌─────────────────────────────────────────────────────────────┐"
    echo "│  macOS requires Input Monitoring permission for HID access │"
    echo "└─────────────────────────────────────────────────────────────┘"
    echo
    echo "When prompted, enable permissions for 'surface-dial' in:"
    echo "  System Settings → Privacy & Security → Input Monitoring"
    echo
    echo "If not prompted, manually add the binary:"
    echo "  ${binary_path}"
    echo

    # Load the service
    echo "Starting service..."
    launchctl load "$plist_file"

    # Verify it started
    sleep 1
    if launchctl list | grep -q "com.surface-dial"; then
        echo
        echo "=== Installation Complete ==="
        echo
        echo "The Surface Dial volume controller is now running."
        echo
        echo "Logs:     ${log_dir}/"
        echo "Binary:   ${binary_path}"
        echo "Config:   ~/Library/Application Support/surface-dial/config.toml"
        echo
        echo "Commands:"
        echo "  View logs:    tail -f ${log_dir}/stderr.log"
        echo "  Status:       launchctl list | grep surface-dial"
        echo "  Stop:         launchctl unload ${plist_file}"
        echo "  Start:        launchctl load ${plist_file}"
        echo "  Uninstall:    $0 --uninstall"
    else
        echo
        echo "Warning: Service may not have started. Check permissions."
        echo "Try: launchctl load ${plist_file}"
    fi
}

uninstall_macos() {
    local plist_file="${HOME}/Library/LaunchAgents/com.surface-dial.plist"
    local binary_path="${HOME}/.local/bin/surface-dial"

    echo "Uninstalling Surface Dial..."

    # Stop and remove service
    launchctl unload "$plist_file" 2>/dev/null || true
    rm -f "$plist_file"

    # Remove binary
    rm -f "$binary_path"

    echo
    echo "=== Uninstall Complete ==="
    echo
    echo "Removed:"
    echo "  - LaunchAgent: ${plist_file}"
    echo "  - Binary: ${binary_path}"
    echo
    echo "Preserved (user data):"
    echo "  - Config: ~/Library/Application Support/surface-dial/"
    echo "  - Logs:   ~/.local/share/surface-dial/"
    echo
    echo "To remove all data: rm -rf ~/.local/share/surface-dial ~/Library/Application\\ Support/surface-dial"
}

# =============================================================================
# Linux Installation (DIAL-ufq)
# =============================================================================

generate_systemd_service() {
    local binary_path="$1"
    local config_dir="$2"
    local log_dir="$3"
    cat << EOF
[Unit]
Description=Surface Dial Volume Controller
Documentation=https://github.com/USER/surface-dial
After=default.target bluetooth.target

[Service]
Type=simple
ExecStart=${binary_path} daemon
Restart=on-failure
RestartSec=5
Environment=RUST_LOG=info

# Hardening
NoNewPrivileges=yes
ProtectSystem=strict
ProtectHome=read-only
ReadWritePaths=${config_dir} ${log_dir}

[Install]
WantedBy=default.target
EOF
}

generate_udev_rule() {
    cat << 'EOF'
# Surface Dial HID access for non-root users
SUBSYSTEM=="hidraw", ATTRS{idVendor}=="045e", ATTRS{idProduct}=="091b", MODE="0666", GROUP="plugdev"
EOF
}

install_linux() {
    local install_dir="${HOME}/.local/bin"
    local service_dir="${HOME}/.config/systemd/user"
    local service_file="${service_dir}/surface-dial.service"
    local config_dir="${HOME}/.config/surface-dial"
    local log_dir="${HOME}/.local/share/surface-dial"
    local binary_path="${install_dir}/surface-dial"

    echo "Installing for Linux..."
    echo

    # Build or download binary
    if [[ "${INSTALL_MODE:-auto}" == "build" ]] || { [[ "${INSTALL_MODE:-auto}" == "auto" ]] && check_rust_toolchain; }; then
        build_from_source
        local src_binary="${SCRIPT_DIR}/target/release/surface-dial"
    else
        echo "Error: Pre-built release download not yet implemented."
        echo "Please install Rust and run with --build-local"
        exit 1
    fi

    # Stop existing service (if any)
    echo "Stopping existing service (if any)..."
    systemctl --user stop surface-dial.service 2>/dev/null || true
    systemctl --user disable surface-dial.service 2>/dev/null || true

    # Create directories
    echo "Creating directories..."
    mkdir -p "$install_dir" "$service_dir" "$config_dir" "$log_dir"

    # Install binary
    echo "Installing binary to: ${binary_path}"
    cp "$src_binary" "$binary_path"
    chmod +x "$binary_path"

    # Generate and install systemd service
    echo "Installing systemd user service..."
    generate_systemd_service "$binary_path" "$config_dir" "$log_dir" > "$service_file"

    # Reload systemd
    systemctl --user daemon-reload

    # Check HID permissions
    echo
    echo "Checking HID device permissions..."
    local hidraw_readable=false
    if ls /dev/hidraw* &>/dev/null; then
        # hidraw devices exist, check if any are readable
        for dev in /dev/hidraw*; do
            if [[ -r "$dev" ]]; then
                hidraw_readable=true
                break
            fi
        done
    fi
    if [[ "$hidraw_readable" == "false" ]]; then
        echo
        echo "┌─────────────────────────────────────────────────────────────┐"
        echo "│  HID permissions may need configuration                     │"
        echo "└─────────────────────────────────────────────────────────────┘"
        echo
        echo "Option 1: Add udev rule (recommended, requires sudo):"
        echo "  sudo tee /etc/udev/rules.d/99-surface-dial.rules << 'RULE'"
        generate_udev_rule
        echo "RULE"
        echo "  sudo udevadm control --reload-rules"
        echo "  sudo udevadm trigger"
        echo
        echo "Option 2: Add user to input group:"
        echo "  sudo usermod -aG input \$USER"
        echo "  # Log out and back in"
        echo
    fi

    # Enable and start service
    echo "Enabling and starting service..."
    systemctl --user enable surface-dial.service
    systemctl --user start surface-dial.service

    # Verify it started
    sleep 1
    if systemctl --user is-active --quiet surface-dial.service; then
        echo
        echo "=== Installation Complete ==="
        echo
        echo "The Surface Dial volume controller is now running."
        echo
        echo "Binary:   ${binary_path}"
        echo "Service:  ${service_file}"
        echo "Config:   ${config_dir}/config.toml"
        echo "Logs:     journalctl --user -u surface-dial -f"
        echo
        echo "Commands:"
        echo "  Status:     systemctl --user status surface-dial"
        echo "  Logs:       journalctl --user -u surface-dial -f"
        echo "  Stop:       systemctl --user stop surface-dial"
        echo "  Start:      systemctl --user start surface-dial"
        echo "  Uninstall:  $0 --uninstall"
        echo
        echo "Note: For auto-start without login, run:"
        echo "  loginctl enable-linger \$USER"
    else
        echo
        echo "Warning: Service may not have started."
        echo "Check status: systemctl --user status surface-dial"
        echo "Check logs:   journalctl --user -u surface-dial"
    fi
}

uninstall_linux() {
    local service_file="${HOME}/.config/systemd/user/surface-dial.service"
    local binary_path="${HOME}/.local/bin/surface-dial"

    echo "Uninstalling Surface Dial..."

    # Stop and disable service
    systemctl --user stop surface-dial.service 2>/dev/null || true
    systemctl --user disable surface-dial.service 2>/dev/null || true
    rm -f "$service_file"
    systemctl --user daemon-reload

    # Remove binary
    rm -f "$binary_path"

    echo
    echo "=== Uninstall Complete ==="
    echo
    echo "Removed:"
    echo "  - Service: ${service_file}"
    echo "  - Binary: ${binary_path}"
    echo
    echo "Preserved (user data):"
    echo "  - Config: ~/.config/surface-dial/"
    echo "  - Logs are in journald"
    echo
    echo "To remove udev rule (if installed):"
    echo "  sudo rm /etc/udev/rules.d/99-surface-dial.rules"
}

# =============================================================================
# Windows Installation (DIAL-wic)
# =============================================================================

install_windows() {
    local install_dir="${LOCALAPPDATA:-$HOME/AppData/Local}/surface-dial"
    local binary_path="${install_dir}/surface-dial.exe"
    local task_name="SurfaceDialController"

    echo "Installing for Windows..."
    echo

    # Check if running in proper environment
    if ! command -v powershell.exe &>/dev/null && ! command -v powershell &>/dev/null; then
        echo "Error: PowerShell not found."
        echo "Please run from PowerShell or Git Bash on Windows."
        exit 1
    fi

    # Build or download binary
    if [[ "${INSTALL_MODE:-auto}" == "build" ]] || { [[ "${INSTALL_MODE:-auto}" == "auto" ]] && check_rust_toolchain; }; then
        build_from_source
        local src_binary="${SCRIPT_DIR}/target/release/surface-dial.exe"
        if [[ ! -f "$src_binary" ]]; then
            src_binary="${SCRIPT_DIR}/target/release/surface-dial"
        fi
    else
        echo "Error: Pre-built release download not yet implemented."
        echo "Please install Rust and run with --build-local"
        exit 1
    fi

    # Create directory and copy binary
    echo "Installing binary to: ${install_dir}"
    mkdir -p "$install_dir"
    cp "$src_binary" "$binary_path"

    # Create Task Scheduler entry using PowerShell
    echo "Setting up auto-start via Task Scheduler..."
    local ps_script='
        $taskName = "SurfaceDialController"
        $exePath = "'"$binary_path"'"

        # Remove existing task if present
        Unregister-ScheduledTask -TaskName $taskName -Confirm:$false -ErrorAction SilentlyContinue

        # Create new task
        $action = New-ScheduledTaskAction -Execute $exePath -Argument "daemon"
        $trigger = New-ScheduledTaskTrigger -AtLogon
        $settings = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -ExecutionTimeLimit 0

        Register-ScheduledTask -TaskName $taskName -Action $action -Trigger $trigger -Settings $settings | Out-Null

        # Start immediately
        Start-ScheduledTask -TaskName $taskName

        Write-Host "Task Scheduler entry created: $taskName"
    '

    if command -v powershell.exe &>/dev/null; then
        powershell.exe -NoProfile -Command "$ps_script"
    else
        powershell -NoProfile -Command "$ps_script"
    fi

    echo
    echo "=== Installation Complete ==="
    echo
    echo "The Surface Dial volume controller is now running."
    echo
    echo "Binary:     ${binary_path}"
    echo "Auto-start: Task Scheduler (${task_name})"
    echo "Config:     %APPDATA%\\surface-dial\\config.toml"
    echo
    echo "Commands (PowerShell):"
    echo "  Status:     Get-ScheduledTask -TaskName ${task_name}"
    echo "  Stop:       Stop-ScheduledTask -TaskName ${task_name}"
    echo "  Start:      Start-ScheduledTask -TaskName ${task_name}"
    echo "  Uninstall:  $0 --uninstall"
}

uninstall_windows() {
    local install_dir="${LOCALAPPDATA:-$HOME/AppData/Local}/surface-dial"
    local binary_path="${install_dir}/surface-dial.exe"
    local task_name="SurfaceDialController"

    echo "Uninstalling Surface Dial..."

    # Remove Task Scheduler entry
    local ps_script='
        $taskName = "SurfaceDialController"
        Stop-ScheduledTask -TaskName $taskName -ErrorAction SilentlyContinue
        Unregister-ScheduledTask -TaskName $taskName -Confirm:$false -ErrorAction SilentlyContinue
        Write-Host "Task Scheduler entry removed"
    '

    if command -v powershell.exe &>/dev/null; then
        powershell.exe -NoProfile -Command "$ps_script"
    elif command -v powershell &>/dev/null; then
        powershell -NoProfile -Command "$ps_script"
    fi

    # Remove binary
    rm -f "$binary_path"
    rmdir "$install_dir" 2>/dev/null || true

    echo
    echo "=== Uninstall Complete ==="
    echo
    echo "Removed:"
    echo "  - Task: ${task_name}"
    echo "  - Binary: ${binary_path}"
    echo
    echo "Preserved (user data):"
    echo "  - Config: %APPDATA%\\surface-dial\\"
}

# =============================================================================
# Main Entry Point
# =============================================================================

main() {
    INSTALL_MODE="auto"
    ACTION="install"

    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --detect-only)
                detect_platform
                exit 0
                ;;
            --from-release)
                INSTALL_MODE="download"
                shift
                ;;
            --build-local)
                INSTALL_MODE="build"
                shift
                ;;
            --uninstall)
                ACTION="uninstall"
                shift
                ;;
            --help|-h)
                print_usage
                exit 0
                ;;
            *)
                echo "Unknown option: $1"
                print_usage
                exit 1
                ;;
        esac
    done

    print_header

    # Validate architecture is supported
    local arch
    arch="$(detect_arch)"
    if [[ "$arch" == "unknown" ]]; then
        echo "Error: Unsupported architecture: $(uname -m)"
        echo "Supported architectures: x86_64, aarch64 (arm64), armv7l"
        exit 1
    fi

    # Dispatch to platform-specific installer/uninstaller
    case "$(detect_os)" in
        macos)
            if [[ "$ACTION" == "uninstall" ]]; then
                uninstall_macos
            else
                install_macos
            fi
            ;;
        linux)
            if [[ "$ACTION" == "uninstall" ]]; then
                uninstall_linux
            else
                install_linux
            fi
            ;;
        windows)
            if [[ "$ACTION" == "uninstall" ]]; then
                uninstall_windows
            else
                install_windows
            fi
            ;;
        *)
            echo "Error: Unsupported platform: $(detect_platform)"
            echo "Supported platforms: macos, linux, windows"
            exit 1
            ;;
    esac
}

main "$@"
