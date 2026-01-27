#!/bin/bash
set -e

APP_PATH="/Applications/Surface Dial.app"
PLIST_NAME="com.surface-dial.volume-controller.plist"
LAUNCH_AGENTS_DIR="${HOME}/Library/LaunchAgents"

echo "=== Surface Dial Volume Controller Uninstaller ==="
echo

# Stop service if running
if launchctl list 2>/dev/null | grep -q "${PLIST_NAME%.plist}"; then
    echo "Stopping service..."
    launchctl unload "${LAUNCH_AGENTS_DIR}/${PLIST_NAME}" 2>/dev/null || true
fi

# Kill any running instances
pkill -f "surface-dial" 2>/dev/null || true

# Remove plist
if [ -f "${LAUNCH_AGENTS_DIR}/${PLIST_NAME}" ]; then
    echo "Removing LaunchAgent..."
    rm "${LAUNCH_AGENTS_DIR}/${PLIST_NAME}"
fi

# Remove app bundle
if [ -d "${APP_PATH}" ]; then
    echo "Removing app bundle..."
    sudo rm -rf "${APP_PATH}"
fi

# Remove old binary if exists
if [ -f "/usr/local/bin/surface-dial" ]; then
    echo "Removing old binary..."
    sudo rm "/usr/local/bin/surface-dial"
fi

echo
echo "=== Uninstallation Complete ==="
