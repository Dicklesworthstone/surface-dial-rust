//! Platform abstraction module for cross-platform audio control
//!
//! This module provides a trait-based abstraction for platform-specific operations
//! like volume control, key simulation, and audio device management.
//!
//! The `Platform` trait defines the interface, and each platform (macOS, Linux, Windows)
//! provides its own implementation.

use thiserror::Error;

#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "windows")]
mod windows;

// Re-export the current platform's implementation
#[cfg(target_os = "macos")]
pub use macos::MacOS as CurrentPlatform;

#[cfg(target_os = "linux")]
pub use linux::Linux as CurrentPlatform;

#[cfg(target_os = "windows")]
pub use windows::Windows as CurrentPlatform;

/// Result type for platform operations
pub type PlatformResult<T> = Result<T, PlatformError>;

/// Errors that can occur during platform operations
#[derive(Debug, Error)]
pub enum PlatformError {
    /// Audio system is not available or not responding
    #[error("Audio system unavailable: {0}")]
    AudioUnavailable(String),

    /// Operation requires permissions that haven't been granted
    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    /// Requested device was not found
    #[error("Device not found: {0}")]
    DeviceNotFound(String),

    /// General operation failure
    #[error("Operation failed: {0}")]
    OperationFailed(String),

    /// Feature is not implemented on this platform
    #[error("Not implemented on this platform")]
    NotImplemented,

    /// Failed to parse output from system command
    #[error("Parse error: {0}")]
    ParseError(String),
}

/// Core platform abstraction trait
///
/// All platform-specific operations are defined here. Each platform provides
/// its own implementation of this trait.
pub trait Platform: Send + Sync {
    // === Audio Control ===

    /// Get current system volume (0-100)
    fn get_volume(&self) -> PlatformResult<i32>;

    /// Set system volume (0-100)
    fn set_volume(&self, vol: i32) -> PlatformResult<()>;

    /// Check if system audio is muted
    fn is_muted(&self) -> PlatformResult<bool>;

    /// Toggle system mute
    fn toggle_mute(&self) -> PlatformResult<()>;

    /// Get current microphone volume (0-100)
    fn get_mic_volume(&self) -> PlatformResult<i32>;

    /// Set microphone volume (0-100)
    fn set_mic_volume(&self, vol: i32) -> PlatformResult<()>;

    /// Check if microphone is muted
    fn is_mic_muted(&self) -> PlatformResult<bool>;

    /// Toggle microphone mute
    fn toggle_mic_mute(&self) -> PlatformResult<()>;

    // === Key Simulation ===

    /// Send key down event
    fn send_key_down(&self, key: Key) -> PlatformResult<()>;

    /// Send key up event
    fn send_key_up(&self, key: Key) -> PlatformResult<()>;

    /// Send media key (play/pause, next, prev)
    fn send_media_key(&self, key: MediaKey) -> PlatformResult<()>;

    // === Audio Device Management ===

    /// List all available output devices
    fn list_output_devices(&self) -> PlatformResult<Vec<AudioDevice>>;

    /// Get current default output device
    fn get_default_output(&self) -> PlatformResult<AudioDevice>;

    /// Set default output device
    fn set_default_output(&self, device_id: &str) -> PlatformResult<()>;

    /// List all available input devices
    fn list_input_devices(&self) -> PlatformResult<Vec<AudioDevice>>;

    // === Notifications ===

    /// Send a system notification
    fn send_notification(
        &self,
        title: &str,
        body: &str,
        urgency: Urgency,
    ) -> PlatformResult<()>;

    // === Daemon Management (optional) ===

    /// Get daemon PID if running
    fn get_daemon_pid(&self) -> Option<u32> {
        None
    }

    /// Check if daemon is installed as system service
    fn is_daemon_installed(&self) -> bool {
        false
    }

    /// Install daemon as system service
    fn install_daemon(&self) -> PlatformResult<()> {
        Err(PlatformError::NotImplemented)
    }

    /// Uninstall daemon from system service
    fn uninstall_daemon(&self) -> PlatformResult<()> {
        Err(PlatformError::NotImplemented)
    }

    // === Foreground App Detection (for per-app volume) ===

    /// Get info about the currently focused application
    fn get_foreground_app(&self) -> PlatformResult<Option<AppInfo>>;

    /// Get volume for a specific app (if supported)
    fn get_app_volume(&self, _app: &AppInfo) -> PlatformResult<Option<i32>> {
        Err(PlatformError::NotImplemented)
    }

    /// Set volume for a specific app (if supported)
    fn set_app_volume(&self, _app: &AppInfo, _vol: i32) -> PlatformResult<()> {
        Err(PlatformError::NotImplemented)
    }
}

/// Function keys that can be simulated
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    F15,
    F16,
    F17,
    F18,
    F19,
}

/// Media keys that can be simulated
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaKey {
    PlayPause,
    NextTrack,
    PrevTrack,
    VolumeUp,
    VolumeDown,
    Mute,
}

/// Represents an audio device (input or output)
#[derive(Debug, Clone)]
pub struct AudioDevice {
    /// Unique identifier for the device
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Type of device (speakers, headphones, etc.)
    pub device_type: DeviceType,
    /// Whether this is the current default device
    pub is_default: bool,
}

/// Type of audio device
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceType {
    Speakers,
    Headphones,
    Hdmi,
    Usb,
    Bluetooth,
    Unknown,
}

/// Notification urgency level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Urgency {
    Low,
    Normal,
    Critical,
}

/// Information about a running application
#[derive(Debug, Clone)]
pub struct AppInfo {
    /// Application name
    pub name: String,
    /// Bundle identifier (macOS) or package name
    pub bundle_id: Option<String>,
    /// Process ID
    pub process_id: Option<u32>,
    /// Path to executable
    pub executable: Option<String>,
}

/// Get the current platform name at runtime
pub fn current_platform_name() -> &'static str {
    #[cfg(target_os = "macos")]
    return "macos";
    #[cfg(target_os = "linux")]
    return "linux";
    #[cfg(target_os = "windows")]
    return "windows";
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    return "unknown";
}

/// Create a new instance of the current platform
pub fn new_platform() -> CurrentPlatform {
    CurrentPlatform::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_name() {
        let name = current_platform_name();
        assert!(!name.is_empty());
        #[cfg(target_os = "macos")]
        assert_eq!(name, "macos");
        #[cfg(target_os = "linux")]
        assert_eq!(name, "linux");
        #[cfg(target_os = "windows")]
        assert_eq!(name, "windows");
    }

    #[test]
    fn test_new_platform() {
        let platform = new_platform();
        // Just ensure it compiles and doesn't panic
        let _ = platform;
    }

    #[test]
    fn test_device_type_equality() {
        assert_eq!(DeviceType::Speakers, DeviceType::Speakers);
        assert_ne!(DeviceType::Speakers, DeviceType::Headphones);
    }

    #[test]
    fn test_key_equality() {
        assert_eq!(Key::F15, Key::F15);
        assert_ne!(Key::F15, Key::F16);
    }

    #[test]
    fn test_media_key_equality() {
        assert_eq!(MediaKey::PlayPause, MediaKey::PlayPause);
        assert_ne!(MediaKey::PlayPause, MediaKey::NextTrack);
    }
}
