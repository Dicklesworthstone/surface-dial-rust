#!/bin/bash
set -e

# New installation paths (DIAL-fas)
NEW_BINARY="${HOME}/.local/bin/surface-dial"
NEW_PLIST="${HOME}/Library/LaunchAgents/com.surface-dial.plist"

# Old installation paths (legacy)
OLD_APP_PATH="/Applications/Surface Dial.app"
OLD_PLIST="${HOME}/Library/LaunchAgents/com.surface-dial.volume-controller.plist"
OLD_BINARY="/usr/local/bin/surface-dial"

echo "=== Surface Dial Volume Controller Uninstaller ==="
echo

# Stop services
echo "Stopping service..."
launchctl unload "$NEW_PLIST" 2>/dev/null || true
launchctl unload "$OLD_PLIST" 2>/dev/null || true
pkill -f "surface-dial" 2>/dev/null || true
sleep 1

# Remove new installation
if [[ -f "$NEW_PLIST" ]]; then
    echo "Removing LaunchAgent: ${NEW_PLIST}"
    rm -f "$NEW_PLIST"
fi

if [[ -f "$NEW_BINARY" ]]; then
    echo "Removing binary: ${NEW_BINARY}"
    rm -f "$NEW_BINARY"
fi

# Remove old installation (legacy)
if [[ -f "$OLD_PLIST" ]]; then
    echo "Removing legacy LaunchAgent: ${OLD_PLIST}"
    rm -f "$OLD_PLIST"
fi

if [[ -d "$OLD_APP_PATH" ]]; then
    echo "Removing legacy app bundle: ${OLD_APP_PATH}"
    sudo rm -rf "$OLD_APP_PATH"
fi

if [[ -f "$OLD_BINARY" ]]; then
    echo "Removing legacy binary: ${OLD_BINARY}"
    sudo rm -f "$OLD_BINARY"
fi

echo
echo "=== Uninstall Complete ==="
echo
echo "Removed all Surface Dial components."
echo
echo "Preserved (user data):"
echo "  - Config: ~/Library/Application Support/surface-dial/"
echo "  - Logs:   ~/.local/share/surface-dial/"
echo
echo "To remove all data:"
echo "  rm -rf ~/.local/share/surface-dial"
echo "  rm -rf ~/Library/Application\\ Support/surface-dial"
