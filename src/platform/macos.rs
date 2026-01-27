//! macOS platform implementation
//!
//! Uses osascript (AppleScript) for volume control and key simulation.
//! This approach is simple and reliable, though it spawns subprocesses.

use super::*;
use std::process::Command;

/// macOS platform implementation
pub struct MacOS;

impl MacOS {
    /// Create a new macOS platform instance
    pub fn new() -> Self {
        Self
    }

    /// Run an osascript command and return stdout
    fn osascript(&self, script: &str) -> PlatformResult<String> {
        let output = Command::new("osascript")
            .args(["-e", script])
            .output()
            .map_err(|e| PlatformError::OperationFailed(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            return Err(PlatformError::OperationFailed(stderr));
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    /// Run an osascript command, ignoring output
    fn osascript_quiet(&self, script: &str) -> PlatformResult<()> {
        let output = Command::new("osascript")
            .args(["-e", script])
            .output()
            .map_err(|e| PlatformError::OperationFailed(e.to_string()))?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            Err(PlatformError::OperationFailed(stderr))
        }
    }

    /// Parse a volume percentage from osascript output
    fn parse_volume(&self, output: &str) -> PlatformResult<i32> {
        output
            .parse()
            .map_err(|_| PlatformError::ParseError(format!("Cannot parse volume: '{}'", output)))
    }

    /// Get the macOS keycode for a function key
    fn key_to_keycode(key: Key) -> u8 {
        match key {
            Key::F15 => 113,
            Key::F16 => 106,
            Key::F17 => 64,
            Key::F18 => 79,
            Key::F19 => 80,
        }
    }
}

impl Default for MacOS {
    fn default() -> Self {
        Self::new()
    }
}

impl Platform for MacOS {
    fn get_volume(&self) -> PlatformResult<i32> {
        let output = self.osascript("output volume of (get volume settings)")?;
        self.parse_volume(&output)
    }

    fn set_volume(&self, vol: i32) -> PlatformResult<()> {
        let vol = vol.clamp(0, 100);
        self.osascript_quiet(&format!("set volume output volume {}", vol))
    }

    fn is_muted(&self) -> PlatformResult<bool> {
        let output = self.osascript("output muted of (get volume settings)")?;
        Ok(output.to_lowercase() == "true")
    }

    fn toggle_mute(&self) -> PlatformResult<()> {
        self.osascript_quiet(
            "set volume output muted not (output muted of (get volume settings))",
        )
    }

    fn get_mic_volume(&self) -> PlatformResult<i32> {
        let output = self.osascript("input volume of (get volume settings)")?;
        self.parse_volume(&output)
    }

    fn set_mic_volume(&self, vol: i32) -> PlatformResult<()> {
        let vol = vol.clamp(0, 100);
        self.osascript_quiet(&format!("set volume input volume {}", vol))
    }

    fn is_mic_muted(&self) -> PlatformResult<bool> {
        // macOS doesn't have a direct mic mute, check if volume is 0
        Ok(self.get_mic_volume()? == 0)
    }

    fn toggle_mic_mute(&self) -> PlatformResult<()> {
        let current = self.get_mic_volume()?;
        if current > 0 {
            self.set_mic_volume(0)
        } else {
            // Restore to 50% when unmuting
            self.set_mic_volume(50)
        }
    }

    fn send_key_down(&self, key: Key) -> PlatformResult<()> {
        let keycode = Self::key_to_keycode(key);
        let script = format!(
            r#"tell application "System Events" to key down {}"#,
            keycode
        );
        self.osascript_quiet(&script)
    }

    fn send_key_up(&self, key: Key) -> PlatformResult<()> {
        let keycode = Self::key_to_keycode(key);
        let script = format!(
            r#"tell application "System Events" to key up {}"#,
            keycode
        );
        self.osascript_quiet(&script)
    }

    fn send_media_key(&self, key: MediaKey) -> PlatformResult<()> {
        // macOS media key codes for "key code" command
        // These work with System Events and send the appropriate NX events
        let keycode = match key {
            MediaKey::PlayPause => 49,  // Space (common fallback)
            MediaKey::NextTrack => 124, // Right arrow (with modifiers)
            MediaKey::PrevTrack => 123, // Left arrow (with modifiers)
            MediaKey::VolumeUp => 72,   // F11 equivalent
            MediaKey::VolumeDown => 73, // F12 equivalent
            MediaKey::Mute => 74,       // F10 equivalent
        };

        // For media keys, we use a different approach - direct key code
        // The actual media keys have special NX keycodes
        let script = match key {
            MediaKey::PlayPause => {
                // Use Music/Spotify control as a more reliable method
                r#"
                tell application "System Events"
                    -- Try to play/pause the frontmost media app
                    key code 49 using {command down}
                end tell
                "#
                .to_string()
            }
            MediaKey::NextTrack => {
                r#"
                tell application "System Events"
                    key code 124 using {command down}
                end tell
                "#
                .to_string()
            }
            MediaKey::PrevTrack => {
                r#"
                tell application "System Events"
                    key code 123 using {command down}
                end tell
                "#
                .to_string()
            }
            _ => {
                format!(
                    r#"tell application "System Events" to key code {}"#,
                    keycode
                )
            }
        };

        self.osascript_quiet(&script)
    }

    fn list_output_devices(&self) -> PlatformResult<Vec<AudioDevice>> {
        // Use system_profiler to get audio devices
        // For a full implementation, we'd use CoreAudio APIs via coreaudio-rs
        let output = Command::new("system_profiler")
            .args(["SPAudioDataType", "-json"])
            .output()
            .map_err(|e| PlatformError::OperationFailed(e.to_string()))?;

        if !output.status.success() {
            return Err(PlatformError::AudioUnavailable(
                "Failed to query audio devices".to_string(),
            ));
        }

        // Parse the JSON output
        // This is a simplified parser - a full implementation would use serde_json
        let _json_str = String::from_utf8_lossy(&output.stdout);

        // For now, return a placeholder with just the built-in output
        // TODO: Implement full CoreAudio device enumeration
        Ok(vec![AudioDevice {
            id: "default".to_string(),
            name: "Built-in Output".to_string(),
            device_type: DeviceType::Speakers,
            is_default: true,
        }])
    }

    fn get_default_output(&self) -> PlatformResult<AudioDevice> {
        // TODO: Use CoreAudio to get the actual default device
        Ok(AudioDevice {
            id: "default".to_string(),
            name: "Built-in Output".to_string(),
            device_type: DeviceType::Speakers,
            is_default: true,
        })
    }

    fn set_default_output(&self, _device_id: &str) -> PlatformResult<()> {
        // TODO: Use CoreAudio to set the default output device
        // This requires the SwitchAudioSource utility or CoreAudio APIs
        Err(PlatformError::NotImplemented)
    }

    fn list_input_devices(&self) -> PlatformResult<Vec<AudioDevice>> {
        // TODO: Implement with CoreAudio
        Ok(vec![AudioDevice {
            id: "default".to_string(),
            name: "Built-in Microphone".to_string(),
            device_type: DeviceType::Unknown,
            is_default: true,
        }])
    }

    fn send_notification(&self, title: &str, body: &str, _urgency: Urgency) -> PlatformResult<()> {
        // Escape quotes in the strings
        let title = title.replace('"', r#"\""#);
        let body = body.replace('"', r#"\""#);

        let script = format!(r#"display notification "{}" with title "{}""#, body, title);
        self.osascript_quiet(&script)
    }

    fn get_foreground_app(&self) -> PlatformResult<Option<AppInfo>> {
        let script = r#"
            tell application "System Events"
                set frontApp to first application process whose frontmost is true
                set appName to name of frontApp
                try
                    set bundleId to bundle identifier of frontApp
                on error
                    set bundleId to ""
                end try
                return appName & "|" & bundleId
            end tell
        "#;

        let output = self.osascript(script)?;
        let parts: Vec<&str> = output.split('|').collect();

        if parts.is_empty() || parts[0].is_empty() {
            return Ok(None);
        }

        Ok(Some(AppInfo {
            name: parts[0].to_string(),
            bundle_id: parts.get(1).filter(|s| !s.is_empty()).map(|s| s.to_string()),
            process_id: None,
            executable: None,
        }))
    }

    fn get_daemon_pid(&self) -> Option<u32> {
        // Check if the daemon is running by looking for our process
        let output = Command::new("pgrep")
            .args(["-f", "surface-dial daemon"])
            .output()
            .ok()?;

        if output.status.success() {
            String::from_utf8_lossy(&output.stdout)
                .trim()
                .lines()
                .next()
                .and_then(|s| s.parse().ok())
        } else {
            None
        }
    }

    fn is_daemon_installed(&self) -> bool {
        // Check if the LaunchAgent plist exists
        if let Some(home) = dirs::home_dir() {
            let plist_path = home
                .join("Library")
                .join("LaunchAgents")
                .join("com.surface-dial.volume-controller.plist");
            plist_path.exists()
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_macos() {
        let macos = MacOS::new();
        let _ = macos; // Just ensure it compiles
    }

    #[test]
    fn test_default_macos() {
        let macos = MacOS::default();
        let _ = macos;
    }

    #[test]
    fn test_key_to_keycode() {
        assert_eq!(MacOS::key_to_keycode(Key::F15), 113);
        assert_eq!(MacOS::key_to_keycode(Key::F16), 106);
        assert_eq!(MacOS::key_to_keycode(Key::F17), 64);
        assert_eq!(MacOS::key_to_keycode(Key::F18), 79);
        assert_eq!(MacOS::key_to_keycode(Key::F19), 80);
    }

    // Integration tests that actually call osascript
    // These require the test to run on macOS with permissions

    #[test]
    #[ignore = "Requires macOS with permissions"]
    fn test_get_volume() {
        let macos = MacOS::new();
        let volume = macos.get_volume();
        assert!(volume.is_ok());
        let vol = volume.unwrap();
        assert!(vol >= 0 && vol <= 100);
    }

    #[test]
    #[ignore = "Requires macOS with permissions"]
    fn test_get_muted() {
        let macos = MacOS::new();
        let muted = macos.is_muted();
        assert!(muted.is_ok());
    }

    #[test]
    #[ignore = "Requires macOS with permissions"]
    fn test_get_mic_volume() {
        let macos = MacOS::new();
        let volume = macos.get_mic_volume();
        assert!(volume.is_ok());
        let vol = volume.unwrap();
        assert!(vol >= 0 && vol <= 100);
    }

    #[test]
    #[ignore = "Requires macOS with permissions"]
    fn test_get_foreground_app() {
        let macos = MacOS::new();
        let app = macos.get_foreground_app();
        assert!(app.is_ok());
        // There should always be a foreground app
        assert!(app.unwrap().is_some());
    }

    #[test]
    #[ignore = "Requires macOS with permissions"]
    fn test_list_output_devices() {
        let macos = MacOS::new();
        let devices = macos.list_output_devices();
        assert!(devices.is_ok());
        // There should be at least one output device
        assert!(!devices.unwrap().is_empty());
    }
}
