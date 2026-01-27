#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
VERSION="1.0.0"

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
    echo "=== Surface Dial Volume Controller Installer ==="
    echo "Version: ${VERSION}"
    echo "Platform: $(detect_platform)"
    echo
}

print_usage() {
    cat << EOF
Usage: $0 [OPTIONS]

Options:
  --detect-only     Print detected platform and exit
  --from-release    Download pre-built binary from GitHub releases
  --build-local     Build from source (requires Rust toolchain)
  --uninstall       Remove Surface Dial and its service
  --help            Show this help message

Examples:
  $0                     # Auto-detect: build local if cargo available, else download
  $0 --detect-only       # Just print platform detection result
  $0 --from-release      # Download pre-built binary
  $0 --build-local       # Build from source
  $0 --uninstall         # Remove installation
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
    echo "Building release binary..."
    cd "$SCRIPT_DIR"
    cargo build --release
    echo "Build complete."
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
# Linux Installation (stub - implemented in DIAL-ufq)
# =============================================================================

install_linux() {
    echo "Linux installation..."
    echo
    echo "Error: Linux installer not yet implemented."
    echo "See issue DIAL-ufq: Linux installer with systemd user service"
    echo
    echo "For now, build manually:"
    echo "  cargo build --release"
    echo "  sudo cp target/release/surface-dial /usr/local/bin/"
    exit 1
}

# =============================================================================
# Windows Installation (stub - implemented in DIAL-wic)
# =============================================================================

install_windows() {
    echo "Windows installation..."
    echo
    echo "Error: Windows installer not yet implemented."
    echo "See issue DIAL-wic: Windows installer with Task Scheduler auto-start"
    echo
    echo "For now, build manually:"
    echo "  cargo build --release"
    echo "  # Copy target/release/surface-dial.exe to desired location"
    exit 1
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
                echo "Error: Linux uninstaller not yet implemented."
                exit 1
            fi
            install_linux
            ;;
        windows)
            if [[ "$ACTION" == "uninstall" ]]; then
                echo "Error: Windows uninstaller not yet implemented."
                exit 1
            fi
            install_windows
            ;;
        *)
            echo "Error: Unsupported platform: $(detect_platform)"
            echo "Supported platforms: macos, linux, windows"
            exit 1
            ;;
    esac
}

main "$@"
