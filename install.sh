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
  --help            Show this help message

Examples:
  $0                     # Auto-detect: build local if cargo available, else download
  $0 --detect-only       # Just print platform detection result
  $0 --from-release      # Download pre-built binary
  $0 --build-local       # Build from source
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
# macOS Installation
# =============================================================================

install_macos() {
    local APP_NAME="Surface Dial.app"
    local APP_PATH="/Applications/${APP_NAME}"
    local BUNDLE_ID="com.surface-dial.volume-controller"
    local PLIST_NAME="com.surface-dial.volume-controller.plist"
    local LAUNCH_AGENTS_DIR="${HOME}/Library/LaunchAgents"

    echo "Installing for macOS..."
    echo

    # Build or download binary
    if [[ "${INSTALL_MODE:-auto}" == "build" ]] || { [[ "${INSTALL_MODE:-auto}" == "auto" ]] && check_rust_toolchain; }; then
        build_from_source
        BINARY_PATH="${SCRIPT_DIR}/target/release/surface-dial"
    else
        echo "Error: Pre-built release download not yet implemented."
        echo "Please install Rust and run with --build-local"
        exit 1
    fi

    echo "Stopping existing service (if any)..."
    launchctl unload "${LAUNCH_AGENTS_DIR}/${PLIST_NAME}" 2>/dev/null || true
    pkill -f "surface-dial" 2>/dev/null || true

    STAGE_DIR="$(mktemp -d)"
    trap 'rm -rf "$STAGE_DIR"' EXIT

    APP_STAGE="${STAGE_DIR}/${APP_NAME}"
    echo "Creating app bundle (staging): ${APP_STAGE}"
    mkdir -p "${APP_STAGE}/Contents/MacOS"
    mkdir -p "${APP_STAGE}/Contents/Resources"

    cp "$BINARY_PATH" "${APP_STAGE}/Contents/MacOS/surface-dial"
    chmod +x "${APP_STAGE}/Contents/MacOS/surface-dial"

    cat > "${APP_STAGE}/Contents/Info.plist" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleIdentifier</key>
    <string>${BUNDLE_ID}</string>
    <key>CFBundleName</key>
    <string>Surface Dial</string>
    <key>CFBundleDisplayName</key>
    <string>Surface Dial</string>
    <key>CFBundleExecutable</key>
    <string>surface-dial</string>
    <key>CFBundleVersion</key>
    <string>${VERSION}</string>
    <key>CFBundleShortVersionString</key>
    <string>${VERSION}</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>LSMinimumSystemVersion</key>
    <string>10.15</string>
    <key>LSUIElement</key>
    <true/>
    <key>NSHighResolutionCapable</key>
    <true/>
</dict>
</plist>
EOF

    echo "Signing app bundle..."
    SIGNING_IDENTITY="$(security find-identity -v -p codesigning 2>/dev/null | awk -F '"' '/\"/ { print $2; exit }' || true)"
    if [[ -z "${SIGNING_IDENTITY}" ]]; then
        CERT_NAME="Surface Dial Local Code Signing"
        if security find-identity -v -p codesigning 2>/dev/null | grep -Fq "${CERT_NAME}"; then
            SIGNING_IDENTITY="${CERT_NAME}"
        else
            echo "No code signing identity found; creating a local one (${CERT_NAME})..."
            CERT_DIR="${STAGE_DIR}/codesign"
            mkdir -p "${CERT_DIR}"
            cat > "${CERT_DIR}/openssl.cnf" << 'CONF'
[req]
distinguished_name = dn
x509_extensions = v3_req
prompt = no

[dn]
CN = Surface Dial Local Code Signing

[v3_req]
keyUsage = critical, digitalSignature
extendedKeyUsage = codeSigning
basicConstraints = critical, CA:FALSE
CONF

            openssl req -x509 -newkey rsa:2048 -nodes -days 3650 \
                -keyout "${CERT_DIR}/key.pem" \
                -out "${CERT_DIR}/cert.pem" \
                -config "${CERT_DIR}/openssl.cnf" >/dev/null 2>&1

            openssl pkcs12 -export -out "${CERT_DIR}/codesign.p12" \
                -inkey "${CERT_DIR}/key.pem" -in "${CERT_DIR}/cert.pem" \
                -passout pass: >/dev/null 2>&1

            security import "${CERT_DIR}/codesign.p12" \
                -k "${HOME}/Library/Keychains/login.keychain-db" \
                -P "" -T /usr/bin/codesign >/dev/null 2>&1 || true

            if security find-identity -v -p codesigning 2>/dev/null | grep -Fq "${CERT_NAME}"; then
                SIGNING_IDENTITY="${CERT_NAME}"
            fi
        fi
    fi

    if [[ -n "${SIGNING_IDENTITY}" ]]; then
        echo "Using signing identity: ${SIGNING_IDENTITY}"
        codesign --force --deep --sign "${SIGNING_IDENTITY}" --identifier "${BUNDLE_ID}" "${APP_STAGE}"
    else
        echo "Falling back to ad-hoc signing (permissions may need re-approval after updates)."
        codesign --force --deep --sign - --identifier "${BUNDLE_ID}" "${APP_STAGE}"
    fi

    echo "Installing app bundle to: ${APP_PATH}"
    sudo rm -rf "${APP_PATH}"
    sudo ditto "${APP_STAGE}" "${APP_PATH}"

    mkdir -p "${LAUNCH_AGENTS_DIR}"

    echo "Installing LaunchAgent..."
    if [[ ! -f "${SCRIPT_DIR}/${PLIST_NAME}" ]]; then
        echo "Error: LaunchAgent plist not found: ${SCRIPT_DIR}/${PLIST_NAME}"
        echo "Please ensure you're running from the project directory."
        exit 1
    fi
    cp "${SCRIPT_DIR}/${PLIST_NAME}" "${LAUNCH_AGENTS_DIR}/"

    echo
    echo "Requesting required macOS permissions (you must toggle them ON once)."
    /usr/bin/open -n -a "${APP_PATH}" --args --setup || true
    echo
    echo "Enable permissions for 'Surface Dial' in:"
    echo "  System Settings -> Privacy & Security -> Input Monitoring"
    echo "  System Settings -> Privacy & Security -> Accessibility"
    echo
    read -r -p "Press Enter once both toggles are enabled... " _

    echo "Starting service..."
    launchctl load "${LAUNCH_AGENTS_DIR}/${PLIST_NAME}"

    echo
    echo "=== Installation Complete ==="
    echo
    echo "The Surface Dial volume controller is now running."
    echo "Logs: /tmp/surface-dial.log"
    echo
    echo "Commands:"
    echo "  View logs:    tail -f /tmp/surface-dial.log"
    echo "  Stop:         launchctl unload ~/Library/LaunchAgents/${PLIST_NAME}"
    echo "  Start:        launchctl load ~/Library/LaunchAgents/${PLIST_NAME}"
    echo "  Uninstall:    ./uninstall.sh"
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

    # Dispatch to platform-specific installer
    case "$(detect_os)" in
        macos)
            install_macos
            ;;
        linux)
            install_linux
            ;;
        windows)
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
