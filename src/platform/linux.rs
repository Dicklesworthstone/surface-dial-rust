//! Linux platform implementation
//!
//! Supports both PipeWire (wpctl) and PulseAudio (pactl) for audio control.
//! Uses playerctl for media control and notify-send for notifications.

use super::*;
use std::process::Command;

/// Linux platform implementation
pub struct Linux {
    /// Whether PipeWire is available (vs PulseAudio)
    has_pipewire: bool,
}

impl Linux {
    /// Create a new Linux platform instance
    pub fn new() -> Self {
        let has_pipewire = Self::check_pipewire();
        Self { has_pipewire }
    }

    /// Check if PipeWire is available
    fn check_pipewire() -> bool {
        Command::new("wpctl")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Run a command and return stdout
    fn run_command(&self, cmd: &str, args: &[&str]) -> PlatformResult<String> {
        let output = Command::new(cmd)
            .args(args)
            .output()
            .map_err(|e| PlatformError::OperationFailed(format!("{}: {}", cmd, e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            return Err(PlatformError::OperationFailed(stderr));
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    /// Run a command, ignoring output
    fn run_command_quiet(&self, cmd: &str, args: &[&str]) -> PlatformResult<()> {
        let output = Command::new(cmd)
            .args(args)
            .output()
            .map_err(|e| PlatformError::OperationFailed(format!("{}: {}", cmd, e)))?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            Err(PlatformError::OperationFailed(stderr))
        }
    }

    /// Parse PipeWire volume output ("Volume: 0.65" -> 65)
    fn parse_pipewire_volume(&self, output: &str) -> PlatformResult<i32> {
        // Format: "Volume: 0.65"
        let vol_str = output
            .split(':')
            .nth(1)
            .ok_or_else(|| PlatformError::ParseError("Invalid wpctl output".to_string()))?
            .trim();

        let vol_float: f64 = vol_str
            .parse()
            .map_err(|_| PlatformError::ParseError(format!("Cannot parse: {}", vol_str)))?;

        Ok((vol_float * 100.0).round() as i32)
    }

    /// Parse PulseAudio volume output (varies by locale, but typically includes "XX%")
    fn parse_pulseaudio_volume(&self, output: &str) -> PlatformResult<i32> {
        // Look for percentage pattern
        for word in output.split_whitespace() {
            if word.ends_with('%') {
                if let Ok(vol) = word.trim_end_matches('%').parse::<i32>() {
                    return Ok(vol.clamp(0, 100));
                }
            }
        }
        Err(PlatformError::ParseError(
            "Cannot parse pactl output".to_string(),
        ))
    }
}

impl Default for Linux {
    fn default() -> Self {
        Self::new()
    }
}

impl Platform for Linux {
    fn get_volume(&self) -> PlatformResult<i32> {
        if self.has_pipewire {
            let output = self.run_command("wpctl", &["get-volume", "@DEFAULT_AUDIO_SINK@"])?;
            self.parse_pipewire_volume(&output)
        } else {
            let output = self.run_command("pactl", &["get-sink-volume", "@DEFAULT_SINK@"])?;
            self.parse_pulseaudio_volume(&output)
        }
    }

    fn set_volume(&self, vol: i32) -> PlatformResult<()> {
        let vol = vol.clamp(0, 100);
        if self.has_pipewire {
            let vol_str = format!("{}%", vol);
            self.run_command_quiet("wpctl", &["set-volume", "@DEFAULT_AUDIO_SINK@", &vol_str])
        } else {
            let vol_str = format!("{}%", vol);
            self.run_command_quiet("pactl", &["set-sink-volume", "@DEFAULT_SINK@", &vol_str])
        }
    }

    fn is_muted(&self) -> PlatformResult<bool> {
        if self.has_pipewire {
            let output = self.run_command("wpctl", &["get-volume", "@DEFAULT_AUDIO_SINK@"])?;
            // Output includes "[MUTED]" if muted
            Ok(output.contains("[MUTED]"))
        } else {
            let output = self.run_command("pactl", &["get-sink-mute", "@DEFAULT_SINK@"])?;
            Ok(output.to_lowercase().contains("yes"))
        }
    }

    fn toggle_mute(&self) -> PlatformResult<()> {
        if self.has_pipewire {
            self.run_command_quiet("wpctl", &["set-mute", "@DEFAULT_AUDIO_SINK@", "toggle"])
        } else {
            self.run_command_quiet("pactl", &["set-sink-mute", "@DEFAULT_SINK@", "toggle"])
        }
    }

    fn get_mic_volume(&self) -> PlatformResult<i32> {
        if self.has_pipewire {
            let output = self.run_command("wpctl", &["get-volume", "@DEFAULT_AUDIO_SOURCE@"])?;
            self.parse_pipewire_volume(&output)
        } else {
            let output = self.run_command("pactl", &["get-source-volume", "@DEFAULT_SOURCE@"])?;
            self.parse_pulseaudio_volume(&output)
        }
    }

    fn set_mic_volume(&self, vol: i32) -> PlatformResult<()> {
        let vol = vol.clamp(0, 100);
        if self.has_pipewire {
            let vol_str = format!("{}%", vol);
            self.run_command_quiet("wpctl", &["set-volume", "@DEFAULT_AUDIO_SOURCE@", &vol_str])
        } else {
            let vol_str = format!("{}%", vol);
            self.run_command_quiet("pactl", &["set-source-volume", "@DEFAULT_SOURCE@", &vol_str])
        }
    }

    fn is_mic_muted(&self) -> PlatformResult<bool> {
        if self.has_pipewire {
            let output = self.run_command("wpctl", &["get-volume", "@DEFAULT_AUDIO_SOURCE@"])?;
            Ok(output.contains("[MUTED]"))
        } else {
            let output = self.run_command("pactl", &["get-source-mute", "@DEFAULT_SOURCE@"])?;
            Ok(output.to_lowercase().contains("yes"))
        }
    }

    fn toggle_mic_mute(&self) -> PlatformResult<()> {
        if self.has_pipewire {
            self.run_command_quiet("wpctl", &["set-mute", "@DEFAULT_AUDIO_SOURCE@", "toggle"])
        } else {
            self.run_command_quiet("pactl", &["set-source-mute", "@DEFAULT_SOURCE@", "toggle"])
        }
    }

    fn send_key_down(&self, key: Key) -> PlatformResult<()> {
        // Use xdotool for X11 key simulation
        let key_name = match key {
            Key::F15 => "F15",
            Key::F16 => "F16",
            Key::F17 => "F17",
            Key::F18 => "F18",
            Key::F19 => "F19",
        };
        self.run_command_quiet("xdotool", &["keydown", key_name])
    }

    fn send_key_up(&self, key: Key) -> PlatformResult<()> {
        let key_name = match key {
            Key::F15 => "F15",
            Key::F16 => "F16",
            Key::F17 => "F17",
            Key::F18 => "F18",
            Key::F19 => "F19",
        };
        self.run_command_quiet("xdotool", &["keyup", key_name])
    }

    fn send_media_key(&self, key: MediaKey) -> PlatformResult<()> {
        // Use playerctl for media control
        let action = match key {
            MediaKey::PlayPause => "play-pause",
            MediaKey::NextTrack => "next",
            MediaKey::PrevTrack => "previous",
            MediaKey::VolumeUp => return self.set_volume(self.get_volume()? + 5),
            MediaKey::VolumeDown => return self.set_volume(self.get_volume()? - 5),
            MediaKey::Mute => return self.toggle_mute(),
        };
        self.run_command_quiet("playerctl", &[action])
    }

    fn list_output_devices(&self) -> PlatformResult<Vec<AudioDevice>> {
        // TODO: Implement full device enumeration
        // For PipeWire: wpctl status
        // For PulseAudio: pactl list sinks
        Ok(vec![AudioDevice {
            id: "default".to_string(),
            name: "Default Output".to_string(),
            device_type: DeviceType::Unknown,
            is_default: true,
        }])
    }

    fn get_default_output(&self) -> PlatformResult<AudioDevice> {
        Ok(AudioDevice {
            id: "default".to_string(),
            name: "Default Output".to_string(),
            device_type: DeviceType::Unknown,
            is_default: true,
        })
    }

    fn set_default_output(&self, _device_id: &str) -> PlatformResult<()> {
        // TODO: Implement with wpctl/pactl
        Err(PlatformError::NotImplemented)
    }

    fn list_input_devices(&self) -> PlatformResult<Vec<AudioDevice>> {
        Ok(vec![AudioDevice {
            id: "default".to_string(),
            name: "Default Input".to_string(),
            device_type: DeviceType::Unknown,
            is_default: true,
        }])
    }

    fn send_notification(&self, title: &str, body: &str, urgency: Urgency) -> PlatformResult<()> {
        let urgency_str = match urgency {
            Urgency::Low => "low",
            Urgency::Normal => "normal",
            Urgency::Critical => "critical",
        };
        self.run_command_quiet("notify-send", &["-u", urgency_str, title, body])
    }

    fn get_foreground_app(&self) -> PlatformResult<Option<AppInfo>> {
        // Use xdotool to get the active window
        let output = self.run_command("xdotool", &["getactivewindow", "getwindowname"])?;

        if output.is_empty() {
            return Ok(None);
        }

        Ok(Some(AppInfo {
            name: output,
            bundle_id: None,
            process_id: None,
            executable: None,
        }))
    }

    fn get_daemon_pid(&self) -> Option<u32> {
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
        // Check for systemd user service
        if let Some(home) = dirs::home_dir() {
            let service_path = home
                .join(".config")
                .join("systemd")
                .join("user")
                .join("surface-dial.service");
            service_path.exists()
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_linux() {
        let linux = Linux::new();
        let _ = linux;
    }

    #[test]
    fn test_default_linux() {
        let linux = Linux::default();
        let _ = linux;
    }

    #[test]
    fn test_parse_pipewire_volume() {
        let linux = Linux::new();

        // Normal volume
        assert_eq!(linux.parse_pipewire_volume("Volume: 0.65").unwrap(), 65);
        assert_eq!(linux.parse_pipewire_volume("Volume: 1.00").unwrap(), 100);
        assert_eq!(linux.parse_pipewire_volume("Volume: 0.00").unwrap(), 0);

        // Edge cases
        assert_eq!(linux.parse_pipewire_volume("Volume: 0.123").unwrap(), 12);
    }

    #[test]
    fn test_parse_pulseaudio_volume() {
        let linux = Linux::new();

        // Common pactl output formats
        assert_eq!(
            linux
                .parse_pulseaudio_volume("Volume: front-left: 65536 / 100%")
                .unwrap(),
            100
        );
        assert_eq!(
            linux
                .parse_pulseaudio_volume("Something 50% something")
                .unwrap(),
            50
        );
    }
}
