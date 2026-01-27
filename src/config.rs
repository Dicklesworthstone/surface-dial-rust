//! Configuration system for Surface Dial volume controller.
//!
//! Provides TOML-based configuration with:
//! - Platform-appropriate paths via `dirs` crate
//! - Dot-notation get/set for all 50+ config keys
//! - Comprehensive validation with ranges and enums
//! - Default values matching existing hardcoded constants
//!
//! Config file locations:
//! - macOS: ~/Library/Application Support/surface-dial/config.toml
//! - Linux: ~/.config/surface-dial/config.toml
//! - Windows: %APPDATA%\surface-dial\config.toml

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Configuration errors
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Config file not found: {0}")]
    NotFound(PathBuf),

    #[error("Failed to read config: {0}")]
    ReadError(#[from] std::io::Error),

    #[error("Failed to parse config: {0}")]
    ParseError(#[from] toml::de::Error),

    #[error("Failed to serialize config: {0}")]
    SerializeError(#[from] toml::ser::Error),

    #[error("Unknown config key: {0}")]
    UnknownKey(String),

    #[error("Invalid value for {key}: {message}")]
    InvalidValue { key: String, message: String },

    #[error("Type mismatch for {key}: expected {expected}, got {got}")]
    TypeMismatch {
        key: String,
        expected: String,
        got: String,
    },
}

/// Root configuration structure
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Config {
    pub volume: VolumeConfig,
    pub microphone: MicrophoneConfig,
    pub acceleration: AccelerationConfig,
    pub interaction: InteractionConfig,
    pub sensitivity: SensitivityConfig,
    pub osd: OsdConfig,
    pub battery: BatteryConfig,
    pub device_switching: DeviceSwitchingConfig,
    pub media_control: MediaControlConfig,
    pub audio_feedback: AudioFeedbackConfig,
    pub events: EventsConfig,
    pub tray: TrayConfig,
    pub daemon: DaemonConfig,
}

/// Volume control settings
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct VolumeConfig {
    /// Minimum step size for slow rotation (1-20)
    pub step_min: i32,
    /// Maximum step size for fast rotation (1-20)
    pub step_max: i32,
    /// Volume curve type: linear, logarithmic, exponential, custom
    pub curve: String,
    /// Curve steepness for log/exp curves (1.0-20.0)
    pub curve_steepness: f64,
}

impl Default for VolumeConfig {
    fn default() -> Self {
        Self {
            step_min: 2,
            step_max: 8,
            curve: "logarithmic".to_string(),
            curve_steepness: 2.0,
        }
    }
}

/// Microphone control settings
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct MicrophoneConfig {
    /// Minimum step size for slow rotation (1-20)
    pub step_min: i32,
    /// Maximum step size for fast rotation (1-20)
    pub step_max: i32,
    /// Duration to stay in mic mode after double-click (1-60 seconds)
    pub mode_duration: i32,
    /// Volume curve type: linear, logarithmic, exponential, custom
    pub curve: String,
    /// Curve steepness for log/exp curves (1.0-20.0)
    pub curve_steepness: f64,
}

impl Default for MicrophoneConfig {
    fn default() -> Self {
        Self {
            step_min: 3,
            step_max: 10,
            mode_duration: 10,
            curve: "linear".to_string(),
            curve_steepness: 2.0,
        }
    }
}

/// Acceleration settings for rotation speed mapping
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct AccelerationConfig {
    /// Time threshold in ms below which max step is used (10-500)
    pub fast_ms: i32,
    /// Time threshold in ms above which min step is used (100-2000)
    pub slow_ms: i32,
}

impl Default for AccelerationConfig {
    fn default() -> Self {
        Self {
            fast_ms: 80,
            slow_ms: 400,
        }
    }
}

/// Interaction timing settings
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct InteractionConfig {
    /// Maximum time between clicks for double-click (100-800ms)
    pub double_click_ms: i32,
    /// Maximum time for triple-click sequence (200-1000ms)
    pub triple_click_ms: i32,
    /// Hold duration for long-press (500-3000ms)
    pub long_press_ms: i32,
}

impl Default for InteractionConfig {
    fn default() -> Self {
        Self {
            double_click_ms: 400,
            triple_click_ms: 600,
            long_press_ms: 1000,
        }
    }
}

/// Sensitivity settings
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct SensitivityConfig {
    /// Rotation units to ignore (0-10)
    pub dead_zone: i32,
    /// Sensitivity multiplier (0.1-5.0)
    pub multiplier: f64,
    /// Reverse rotation direction
    pub invert: bool,
    /// Preset name: default, accessibility, precision, fast
    pub preset: String,
}

impl Default for SensitivityConfig {
    fn default() -> Self {
        Self {
            dead_zone: 0,
            multiplier: 1.0,
            invert: false,
            preset: "default".to_string(),
        }
    }
}

/// On-Screen Display settings
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct OsdConfig {
    /// Enable OSD overlay
    pub enabled: bool,
    /// Position: center-bottom, center, top-right, top-left, bottom-right, bottom-left
    pub position: String,
    /// Size: small, medium, large
    pub size: String,
    /// Display timeout in ms (500-5000)
    pub timeout_ms: i32,
    /// Opacity (0.1-1.0)
    pub opacity: f64,
    /// Show volume/mute icon
    pub show_icon: bool,
    /// Show percentage number
    pub show_percentage: bool,
}

impl Default for OsdConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            position: "center-bottom".to_string(),
            size: "medium".to_string(),
            timeout_ms: 1500,
            opacity: 0.9,
            show_icon: true,
            show_percentage: true,
        }
    }
}

/// Battery monitoring settings
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct BatteryConfig {
    /// Enable battery monitoring
    pub enabled: bool,
    /// Poll interval in seconds (60-3600)
    pub poll_interval_seconds: i32,
    /// Show battery in status output
    pub show_in_status: bool,
    /// Show battery level in OSD
    pub show_in_osd: bool,
    /// Low battery warning thresholds
    pub warning_thresholds: Vec<i32>,
}

impl Default for BatteryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            poll_interval_seconds: 300,
            show_in_status: true,
            show_in_osd: true,
            warning_thresholds: vec![20, 10, 5],
        }
    }
}

/// Device switching settings
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct DeviceSwitchingConfig {
    /// Enable device switching
    pub enabled: bool,
    /// Trigger mode: long_press_rotate
    pub mode: String,
    /// Show device list in OSD
    pub show_in_osd: bool,
}

impl Default for DeviceSwitchingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            mode: "long_press_rotate".to_string(),
            show_in_osd: true,
        }
    }
}

/// Media control settings
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct MediaControlConfig {
    /// Enable media control
    pub enabled: bool,
    /// Triple-click action: play_pause, next_track
    pub triple_click_action: String,
}

impl Default for MediaControlConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            triple_click_action: "play_pause".to_string(),
        }
    }
}

/// Audio feedback settings
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct AudioFeedbackConfig {
    /// Enable audio feedback
    pub enabled: bool,
    /// Feedback volume (0.0-1.0)
    pub volume: f64,
    /// Play tick on rotation
    pub tick: bool,
    /// Play sound at volume boundaries
    pub boundary: bool,
    /// Play sound on mode change
    pub mode_change: bool,
    /// Play sound on mute toggle
    pub mute: bool,
}

impl Default for AudioFeedbackConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            volume: 0.3,
            tick: true,
            boundary: true,
            mode_change: true,
            mute: true,
        }
    }
}

/// Event hooks settings
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct EventsConfig {
    /// Enable event hooks
    pub enabled: bool,
    /// Debounce interval in ms (100-5000)
    pub debounce_ms: i32,
    /// Script hooks directory
    pub scripts_dir: Option<String>,
    /// Webhook configurations
    pub webhooks: Vec<WebhookConfig>,
}

impl Default for EventsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            debounce_ms: 500,
            scripts_dir: None,
            webhooks: vec![],
        }
    }
}

/// Webhook configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WebhookConfig {
    pub url: String,
    pub events: Vec<String>,
    pub method: Option<String>,
    pub headers: Option<std::collections::HashMap<String, String>>,
}

/// System tray settings
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct TrayConfig {
    /// Enable system tray icon
    pub enabled: bool,
    /// Show current volume in tray
    pub show_volume: bool,
    /// Show battery level in tray
    pub show_battery: bool,
    /// Show current mode in tray
    pub show_mode: bool,
}

impl Default for TrayConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            show_volume: true,
            show_battery: true,
            show_mode: true,
        }
    }
}

/// Daemon settings
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct DaemonConfig {
    /// Log level: error, warn, info, debug, trace
    pub log_level: String,
    /// Enable logging to file
    pub log_file_enabled: bool,
    /// Max log file size in MB (1-100)
    pub log_max_size_mb: i32,
    /// Number of rotated log files to keep (1-10)
    pub log_keep_files: i32,
    /// Use JSON format for log files (for machine parsing)
    pub log_json: bool,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            log_level: "info".to_string(),
            log_file_enabled: true,
            log_max_size_mb: 10,
            log_keep_files: 3,
            log_json: false,
        }
    }
}

impl Config {
    /// Get the platform-appropriate config file path.
    pub fn path() -> PathBuf {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("surface-dial");
        config_dir.join("config.toml")
    }

    /// Alias for path() for CLI compatibility
    pub fn config_path() -> PathBuf {
        Self::path()
    }

    /// Get the platform-appropriate data directory.
    pub fn data_dir() -> PathBuf {
        dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("surface-dial")
    }

    /// Load config from the default path, creating default if not found.
    pub fn load() -> Self {
        Self::load_from(Self::path()).unwrap_or_default()
    }

    /// Load config from a specific path.
    pub fn load_from(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        if !path.exists() {
            return Err(ConfigError::NotFound(path.to_path_buf()));
        }
        let content = fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    /// Save config to the default path.
    pub fn save(&self) -> Result<(), ConfigError> {
        self.save_to(Self::path())
    }

    /// Save config to a specific path.
    pub fn save_to(&self, path: impl AsRef<Path>) -> Result<(), ConfigError> {
        let path = path.as_ref();
        // Create parent directories if needed
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    /// Validate cross-field constraints.
    ///
    /// Returns a list of validation errors. Empty list means valid.
    ///
    /// Note: This should be called after `load_from()` to ensure the loaded
    /// config is valid. Individual field validation is done by `set()`, but
    /// cross-field constraints (like step_min <= step_max) are only checked here.
    pub fn validate(&self) -> Vec<String> {
        let mut errors = Vec::new();

        // Volume: step_min should be <= step_max
        if self.volume.step_min > self.volume.step_max {
            errors.push(format!(
                "volume.step_min ({}) must be <= volume.step_max ({})",
                self.volume.step_min, self.volume.step_max
            ));
        }

        // Microphone: step_min should be <= step_max
        if self.microphone.step_min > self.microphone.step_max {
            errors.push(format!(
                "microphone.step_min ({}) must be <= microphone.step_max ({})",
                self.microphone.step_min, self.microphone.step_max
            ));
        }

        // Acceleration: fast_ms should be < slow_ms
        if self.acceleration.fast_ms >= self.acceleration.slow_ms {
            errors.push(format!(
                "acceleration.fast_ms ({}) must be < acceleration.slow_ms ({})",
                self.acceleration.fast_ms, self.acceleration.slow_ms
            ));
        }

        // Interaction: double_click_ms should be < triple_click_ms
        if self.interaction.double_click_ms >= self.interaction.triple_click_ms {
            errors.push(format!(
                "interaction.double_click_ms ({}) must be < interaction.triple_click_ms ({})",
                self.interaction.double_click_ms, self.interaction.triple_click_ms
            ));
        }

        // Interaction: triple_click_ms should be < long_press_ms (otherwise triple-click triggers long-press)
        if self.interaction.triple_click_ms >= self.interaction.long_press_ms {
            errors.push(format!(
                "interaction.triple_click_ms ({}) must be < interaction.long_press_ms ({})",
                self.interaction.triple_click_ms, self.interaction.long_press_ms
            ));
        }

        // Validate enum values (catches invalid values from manually edited config files)
        let valid_curves = ["linear", "logarithmic", "exponential", "custom"];
        if !valid_curves.contains(&self.volume.curve.as_str()) {
            errors.push(format!(
                "volume.curve '{}' must be one of: {}",
                self.volume.curve,
                valid_curves.join(", ")
            ));
        }
        if !valid_curves.contains(&self.microphone.curve.as_str()) {
            errors.push(format!(
                "microphone.curve '{}' must be one of: {}",
                self.microphone.curve,
                valid_curves.join(", ")
            ));
        }

        let valid_presets = ["default", "accessibility", "precision", "fast"];
        if !valid_presets.contains(&self.sensitivity.preset.as_str()) {
            errors.push(format!(
                "sensitivity.preset '{}' must be one of: {}",
                self.sensitivity.preset,
                valid_presets.join(", ")
            ));
        }

        let valid_positions = ["center-bottom", "center", "top-right", "top-left", "bottom-right", "bottom-left"];
        if !valid_positions.contains(&self.osd.position.as_str()) {
            errors.push(format!(
                "osd.position '{}' must be one of: {}",
                self.osd.position,
                valid_positions.join(", ")
            ));
        }

        let valid_sizes = ["small", "medium", "large"];
        if !valid_sizes.contains(&self.osd.size.as_str()) {
            errors.push(format!(
                "osd.size '{}' must be one of: {}",
                self.osd.size,
                valid_sizes.join(", ")
            ));
        }

        let valid_switch_modes = ["long_press_rotate"];
        if !valid_switch_modes.contains(&self.device_switching.mode.as_str()) {
            errors.push(format!(
                "device_switching.mode '{}' must be one of: {}",
                self.device_switching.mode,
                valid_switch_modes.join(", ")
            ));
        }

        let valid_media_actions = ["play_pause", "next_track"];
        if !valid_media_actions.contains(&self.media_control.triple_click_action.as_str()) {
            errors.push(format!(
                "media_control.triple_click_action '{}' must be one of: {}",
                self.media_control.triple_click_action,
                valid_media_actions.join(", ")
            ));
        }

        let valid_log_levels = ["error", "warn", "info", "debug", "trace"];
        if !valid_log_levels.contains(&self.daemon.log_level.as_str()) {
            errors.push(format!(
                "daemon.log_level '{}' must be one of: {}",
                self.daemon.log_level,
                valid_log_levels.join(", ")
            ));
        }

        errors
    }

    /// Returns true if the config passes all cross-field validation.
    ///
    /// This is a convenience wrapper around `validate().is_empty()`.
    pub fn is_valid(&self) -> bool {
        self.validate().is_empty()
    }

    /// Get a list of all configurable keys.
    pub fn keys() -> Vec<&'static str> {
        vec![
            // Volume
            "volume.step_min",
            "volume.step_max",
            "volume.curve",
            "volume.curve_steepness",
            // Microphone
            "microphone.step_min",
            "microphone.step_max",
            "microphone.mode_duration",
            "microphone.curve",
            "microphone.curve_steepness",
            // Acceleration
            "acceleration.fast_ms",
            "acceleration.slow_ms",
            // Interaction
            "interaction.double_click_ms",
            "interaction.triple_click_ms",
            "interaction.long_press_ms",
            // Sensitivity
            "sensitivity.dead_zone",
            "sensitivity.multiplier",
            "sensitivity.invert",
            "sensitivity.preset",
            // OSD
            "osd.enabled",
            "osd.position",
            "osd.size",
            "osd.timeout_ms",
            "osd.opacity",
            "osd.show_icon",
            "osd.show_percentage",
            // Battery
            "battery.enabled",
            "battery.poll_interval_seconds",
            "battery.show_in_status",
            "battery.show_in_osd",
            // Device switching
            "device_switching.enabled",
            "device_switching.mode",
            "device_switching.show_in_osd",
            // Media control
            "media_control.enabled",
            "media_control.triple_click_action",
            // Audio feedback
            "audio_feedback.enabled",
            "audio_feedback.volume",
            "audio_feedback.tick",
            "audio_feedback.boundary",
            "audio_feedback.mode_change",
            "audio_feedback.mute",
            // Events
            "events.enabled",
            "events.debounce_ms",
            "events.scripts_dir",
            // Tray
            "tray.enabled",
            "tray.show_volume",
            "tray.show_battery",
            "tray.show_mode",
            // Daemon
            "daemon.log_level",
            "daemon.log_file_enabled",
            "daemon.log_max_size_mb",
            "daemon.log_keep_files",
            "daemon.log_json",
        ]
    }

    /// Get a config value by dot-notation key.
    pub fn get(&self, key: &str) -> Result<String, ConfigError> {
        match key {
            // Volume
            "volume.step_min" => Ok(self.volume.step_min.to_string()),
            "volume.step_max" => Ok(self.volume.step_max.to_string()),
            "volume.curve" => Ok(self.volume.curve.clone()),
            "volume.curve_steepness" => Ok(self.volume.curve_steepness.to_string()),
            // Microphone
            "microphone.step_min" => Ok(self.microphone.step_min.to_string()),
            "microphone.step_max" => Ok(self.microphone.step_max.to_string()),
            "microphone.mode_duration" => Ok(self.microphone.mode_duration.to_string()),
            "microphone.curve" => Ok(self.microphone.curve.clone()),
            "microphone.curve_steepness" => Ok(self.microphone.curve_steepness.to_string()),
            // Acceleration
            "acceleration.fast_ms" => Ok(self.acceleration.fast_ms.to_string()),
            "acceleration.slow_ms" => Ok(self.acceleration.slow_ms.to_string()),
            // Interaction
            "interaction.double_click_ms" => Ok(self.interaction.double_click_ms.to_string()),
            "interaction.triple_click_ms" => Ok(self.interaction.triple_click_ms.to_string()),
            "interaction.long_press_ms" => Ok(self.interaction.long_press_ms.to_string()),
            // Sensitivity
            "sensitivity.dead_zone" => Ok(self.sensitivity.dead_zone.to_string()),
            "sensitivity.multiplier" => Ok(self.sensitivity.multiplier.to_string()),
            "sensitivity.invert" => Ok(self.sensitivity.invert.to_string()),
            "sensitivity.preset" => Ok(self.sensitivity.preset.clone()),
            // OSD
            "osd.enabled" => Ok(self.osd.enabled.to_string()),
            "osd.position" => Ok(self.osd.position.clone()),
            "osd.size" => Ok(self.osd.size.clone()),
            "osd.timeout_ms" => Ok(self.osd.timeout_ms.to_string()),
            "osd.opacity" => Ok(self.osd.opacity.to_string()),
            "osd.show_icon" => Ok(self.osd.show_icon.to_string()),
            "osd.show_percentage" => Ok(self.osd.show_percentage.to_string()),
            // Battery
            "battery.enabled" => Ok(self.battery.enabled.to_string()),
            "battery.poll_interval_seconds" => Ok(self.battery.poll_interval_seconds.to_string()),
            "battery.show_in_status" => Ok(self.battery.show_in_status.to_string()),
            "battery.show_in_osd" => Ok(self.battery.show_in_osd.to_string()),
            // Device switching
            "device_switching.enabled" => Ok(self.device_switching.enabled.to_string()),
            "device_switching.mode" => Ok(self.device_switching.mode.clone()),
            "device_switching.show_in_osd" => Ok(self.device_switching.show_in_osd.to_string()),
            // Media control
            "media_control.enabled" => Ok(self.media_control.enabled.to_string()),
            "media_control.triple_click_action" => {
                Ok(self.media_control.triple_click_action.clone())
            }
            // Audio feedback
            "audio_feedback.enabled" => Ok(self.audio_feedback.enabled.to_string()),
            "audio_feedback.volume" => Ok(self.audio_feedback.volume.to_string()),
            "audio_feedback.tick" => Ok(self.audio_feedback.tick.to_string()),
            "audio_feedback.boundary" => Ok(self.audio_feedback.boundary.to_string()),
            "audio_feedback.mode_change" => Ok(self.audio_feedback.mode_change.to_string()),
            "audio_feedback.mute" => Ok(self.audio_feedback.mute.to_string()),
            // Events
            "events.enabled" => Ok(self.events.enabled.to_string()),
            "events.debounce_ms" => Ok(self.events.debounce_ms.to_string()),
            "events.scripts_dir" => Ok(self.events.scripts_dir.clone().unwrap_or_default()),
            // Tray
            "tray.enabled" => Ok(self.tray.enabled.to_string()),
            "tray.show_volume" => Ok(self.tray.show_volume.to_string()),
            "tray.show_battery" => Ok(self.tray.show_battery.to_string()),
            "tray.show_mode" => Ok(self.tray.show_mode.to_string()),
            // Daemon
            "daemon.log_level" => Ok(self.daemon.log_level.clone()),
            "daemon.log_file_enabled" => Ok(self.daemon.log_file_enabled.to_string()),
            "daemon.log_max_size_mb" => Ok(self.daemon.log_max_size_mb.to_string()),
            "daemon.log_keep_files" => Ok(self.daemon.log_keep_files.to_string()),
            "daemon.log_json" => Ok(self.daemon.log_json.to_string()),
            _ => Err(ConfigError::UnknownKey(key.to_string())),
        }
    }

    /// Set a config value by dot-notation key with validation.
    pub fn set(&mut self, key: &str, value: &str) -> Result<(), ConfigError> {
        match key {
            // Volume
            "volume.step_min" => {
                let v = parse_int(key, value, 1, 20)?;
                self.volume.step_min = v;
            }
            "volume.step_max" => {
                let v = parse_int(key, value, 1, 20)?;
                self.volume.step_max = v;
            }
            "volume.curve" => {
                validate_enum(key, value, &["linear", "logarithmic", "exponential", "custom"])?;
                self.volume.curve = value.to_string();
            }
            "volume.curve_steepness" => {
                let v = parse_float(key, value, 1.0, 20.0)?;
                self.volume.curve_steepness = v;
            }
            // Microphone
            "microphone.step_min" => {
                let v = parse_int(key, value, 1, 20)?;
                self.microphone.step_min = v;
            }
            "microphone.step_max" => {
                let v = parse_int(key, value, 1, 20)?;
                self.microphone.step_max = v;
            }
            "microphone.mode_duration" => {
                let v = parse_int(key, value, 1, 60)?;
                self.microphone.mode_duration = v;
            }
            "microphone.curve" => {
                validate_enum(key, value, &["linear", "logarithmic", "exponential", "custom"])?;
                self.microphone.curve = value.to_string();
            }
            "microphone.curve_steepness" => {
                let v = parse_float(key, value, 1.0, 20.0)?;
                self.microphone.curve_steepness = v;
            }
            // Acceleration
            "acceleration.fast_ms" => {
                let v = parse_int(key, value, 10, 500)?;
                self.acceleration.fast_ms = v;
            }
            "acceleration.slow_ms" => {
                let v = parse_int(key, value, 100, 2000)?;
                self.acceleration.slow_ms = v;
            }
            // Interaction
            "interaction.double_click_ms" => {
                let v = parse_int(key, value, 100, 800)?;
                self.interaction.double_click_ms = v;
            }
            "interaction.triple_click_ms" => {
                let v = parse_int(key, value, 200, 1000)?;
                self.interaction.triple_click_ms = v;
            }
            "interaction.long_press_ms" => {
                let v = parse_int(key, value, 500, 3000)?;
                self.interaction.long_press_ms = v;
            }
            // Sensitivity
            "sensitivity.dead_zone" => {
                let v = parse_int(key, value, 0, 10)?;
                self.sensitivity.dead_zone = v;
            }
            "sensitivity.multiplier" => {
                let v = parse_float(key, value, 0.1, 5.0)?;
                self.sensitivity.multiplier = v;
            }
            "sensitivity.invert" => {
                let v = parse_bool(key, value)?;
                self.sensitivity.invert = v;
            }
            "sensitivity.preset" => {
                validate_enum(key, value, &["default", "accessibility", "precision", "fast"])?;
                self.sensitivity.preset = value.to_string();
            }
            // OSD
            "osd.enabled" => {
                let v = parse_bool(key, value)?;
                self.osd.enabled = v;
            }
            "osd.position" => {
                validate_enum(
                    key,
                    value,
                    &[
                        "center-bottom",
                        "center",
                        "top-right",
                        "top-left",
                        "bottom-right",
                        "bottom-left",
                    ],
                )?;
                self.osd.position = value.to_string();
            }
            "osd.size" => {
                validate_enum(key, value, &["small", "medium", "large"])?;
                self.osd.size = value.to_string();
            }
            "osd.timeout_ms" => {
                let v = parse_int(key, value, 500, 5000)?;
                self.osd.timeout_ms = v;
            }
            "osd.opacity" => {
                let v = parse_float(key, value, 0.1, 1.0)?;
                self.osd.opacity = v;
            }
            "osd.show_icon" => {
                let v = parse_bool(key, value)?;
                self.osd.show_icon = v;
            }
            "osd.show_percentage" => {
                let v = parse_bool(key, value)?;
                self.osd.show_percentage = v;
            }
            // Battery
            "battery.enabled" => {
                let v = parse_bool(key, value)?;
                self.battery.enabled = v;
            }
            "battery.poll_interval_seconds" => {
                let v = parse_int(key, value, 60, 3600)?;
                self.battery.poll_interval_seconds = v;
            }
            "battery.show_in_status" => {
                let v = parse_bool(key, value)?;
                self.battery.show_in_status = v;
            }
            "battery.show_in_osd" => {
                let v = parse_bool(key, value)?;
                self.battery.show_in_osd = v;
            }
            // Device switching
            "device_switching.enabled" => {
                let v = parse_bool(key, value)?;
                self.device_switching.enabled = v;
            }
            "device_switching.mode" => {
                validate_enum(key, value, &["long_press_rotate"])?;
                self.device_switching.mode = value.to_string();
            }
            "device_switching.show_in_osd" => {
                let v = parse_bool(key, value)?;
                self.device_switching.show_in_osd = v;
            }
            // Media control
            "media_control.enabled" => {
                let v = parse_bool(key, value)?;
                self.media_control.enabled = v;
            }
            "media_control.triple_click_action" => {
                validate_enum(key, value, &["play_pause", "next_track"])?;
                self.media_control.triple_click_action = value.to_string();
            }
            // Audio feedback
            "audio_feedback.enabled" => {
                let v = parse_bool(key, value)?;
                self.audio_feedback.enabled = v;
            }
            "audio_feedback.volume" => {
                let v = parse_float(key, value, 0.0, 1.0)?;
                self.audio_feedback.volume = v;
            }
            "audio_feedback.tick" => {
                let v = parse_bool(key, value)?;
                self.audio_feedback.tick = v;
            }
            "audio_feedback.boundary" => {
                let v = parse_bool(key, value)?;
                self.audio_feedback.boundary = v;
            }
            "audio_feedback.mode_change" => {
                let v = parse_bool(key, value)?;
                self.audio_feedback.mode_change = v;
            }
            "audio_feedback.mute" => {
                let v = parse_bool(key, value)?;
                self.audio_feedback.mute = v;
            }
            // Events
            "events.enabled" => {
                let v = parse_bool(key, value)?;
                self.events.enabled = v;
            }
            "events.debounce_ms" => {
                let v = parse_int(key, value, 100, 5000)?;
                self.events.debounce_ms = v;
            }
            "events.scripts_dir" => {
                // Empty string clears the scripts_dir
                if value.is_empty() {
                    self.events.scripts_dir = None;
                } else {
                    self.events.scripts_dir = Some(value.to_string());
                }
            }
            // Tray
            "tray.enabled" => {
                let v = parse_bool(key, value)?;
                self.tray.enabled = v;
            }
            "tray.show_volume" => {
                let v = parse_bool(key, value)?;
                self.tray.show_volume = v;
            }
            "tray.show_battery" => {
                let v = parse_bool(key, value)?;
                self.tray.show_battery = v;
            }
            "tray.show_mode" => {
                let v = parse_bool(key, value)?;
                self.tray.show_mode = v;
            }
            // Daemon
            "daemon.log_level" => {
                validate_enum(key, value, &["error", "warn", "info", "debug", "trace"])?;
                self.daemon.log_level = value.to_string();
            }
            "daemon.log_file_enabled" => {
                let v = parse_bool(key, value)?;
                self.daemon.log_file_enabled = v;
            }
            "daemon.log_max_size_mb" => {
                let v = parse_int(key, value, 1, 100)?;
                self.daemon.log_max_size_mb = v;
            }
            "daemon.log_keep_files" => {
                let v = parse_int(key, value, 1, 10)?;
                self.daemon.log_keep_files = v;
            }
            "daemon.log_json" => {
                let v = parse_bool(key, value)?;
                self.daemon.log_json = v;
            }
            _ => return Err(ConfigError::UnknownKey(key.to_string())),
        }
        Ok(())
    }

    /// Get a config value as JSON by dot-notation key.
    ///
    /// This is a convenience method for CLI output.
    pub fn get_value(&self, key: &str) -> Result<serde_json::Value, ConfigError> {
        let string_value = self.get(key)?;

        // Try to parse as the appropriate JSON type
        // First try boolean
        if string_value == "true" {
            return Ok(serde_json::Value::Bool(true));
        }
        if string_value == "false" {
            return Ok(serde_json::Value::Bool(false));
        }

        // Try integer
        if let Ok(n) = string_value.parse::<i64>() {
            return Ok(serde_json::Value::Number(n.into()));
        }

        // Try float
        if let Ok(n) = string_value.parse::<f64>() {
            if let Some(num) = serde_json::Number::from_f64(n) {
                return Ok(serde_json::Value::Number(num));
            }
        }

        // Default to string
        Ok(serde_json::Value::String(string_value))
    }

    /// Set a config value from a string (alias for set).
    ///
    /// This is a convenience method for CLI compatibility.
    pub fn set_value(&mut self, key: &str, value: &str) -> Result<(), ConfigError> {
        self.set(key, value)
    }

    /// Reset a specific configuration section to defaults.
    pub fn reset_section(&mut self, section: &str) -> Result<(), ConfigError> {
        match section {
            "volume" => self.volume = VolumeConfig::default(),
            "microphone" => self.microphone = MicrophoneConfig::default(),
            "acceleration" => self.acceleration = AccelerationConfig::default(),
            "interaction" => self.interaction = InteractionConfig::default(),
            "sensitivity" => self.sensitivity = SensitivityConfig::default(),
            "osd" => self.osd = OsdConfig::default(),
            "battery" => self.battery = BatteryConfig::default(),
            "device_switching" => self.device_switching = DeviceSwitchingConfig::default(),
            "media_control" => self.media_control = MediaControlConfig::default(),
            "audio_feedback" => self.audio_feedback = AudioFeedbackConfig::default(),
            "events" => self.events = EventsConfig::default(),
            "tray" => self.tray = TrayConfig::default(),
            "daemon" => self.daemon = DaemonConfig::default(),
            _ => {
                return Err(ConfigError::UnknownKey(format!(
                    "unknown section '{}'. Valid sections: volume, microphone, acceleration, interaction, sensitivity, osd, battery, device_switching, media_control, audio_feedback, events, tray, daemon",
                    section
                )));
            }
        }
        Ok(())
    }
}

// Validation helpers

fn parse_int(key: &str, value: &str, min: i32, max: i32) -> Result<i32, ConfigError> {
    let v: i32 = value.parse().map_err(|_| ConfigError::TypeMismatch {
        key: key.to_string(),
        expected: "integer".to_string(),
        got: value.to_string(),
    })?;
    if v < min || v > max {
        return Err(ConfigError::InvalidValue {
            key: key.to_string(),
            message: format!("must be between {} and {}, got {}", min, max, v),
        });
    }
    Ok(v)
}

fn parse_float(key: &str, value: &str, min: f64, max: f64) -> Result<f64, ConfigError> {
    let v: f64 = value.parse().map_err(|_| ConfigError::TypeMismatch {
        key: key.to_string(),
        expected: "float".to_string(),
        got: value.to_string(),
    })?;
    if v < min || v > max {
        return Err(ConfigError::InvalidValue {
            key: key.to_string(),
            message: format!("must be between {} and {}, got {}", min, max, v),
        });
    }
    Ok(v)
}

fn parse_bool(key: &str, value: &str) -> Result<bool, ConfigError> {
    match value.to_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Ok(true),
        "false" | "0" | "no" | "off" => Ok(false),
        _ => Err(ConfigError::TypeMismatch {
            key: key.to_string(),
            expected: "boolean (true/false)".to_string(),
            got: value.to_string(),
        }),
    }
}

fn validate_enum(key: &str, value: &str, allowed: &[&str]) -> Result<(), ConfigError> {
    if !allowed.contains(&value) {
        return Err(ConfigError::InvalidValue {
            key: key.to_string(),
            message: format!(
                "must be one of [{}], got '{}'",
                allowed.join(", "),
                value
            ),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.volume.step_min, 2);
        assert_eq!(config.volume.step_max, 8);
        assert_eq!(config.volume.curve, "logarithmic");
        assert_eq!(config.interaction.double_click_ms, 400);
        assert_eq!(config.interaction.long_press_ms, 1000);
        assert!(config.osd.enabled);
        assert!(!config.audio_feedback.enabled);
    }

    #[test]
    fn test_config_keys_count() {
        let keys = Config::keys();
        assert!(keys.len() >= 45, "Expected at least 45 config keys, got {}", keys.len());
    }

    #[test]
    fn test_get_all_keys() {
        let config = Config::default();
        for key in Config::keys() {
            let result = config.get(key);
            assert!(result.is_ok(), "Failed to get key '{}': {:?}", key, result);
        }
    }

    #[test]
    fn test_set_int_valid() {
        let mut config = Config::default();
        assert!(config.set("volume.step_min", "5").is_ok());
        assert_eq!(config.volume.step_min, 5);
    }

    #[test]
    fn test_set_int_below_range() {
        let mut config = Config::default();
        let result = config.set("volume.step_min", "0");
        assert!(result.is_err());
        match result {
            Err(ConfigError::InvalidValue { key, .. }) => assert_eq!(key, "volume.step_min"),
            _ => panic!("Expected InvalidValue error"),
        }
    }

    #[test]
    fn test_set_int_above_range() {
        let mut config = Config::default();
        let result = config.set("volume.step_min", "21");
        assert!(result.is_err());
    }

    #[test]
    fn test_set_float_valid() {
        let mut config = Config::default();
        assert!(config.set("sensitivity.multiplier", "1.5").is_ok());
        assert!((config.sensitivity.multiplier - 1.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_set_float_invalid() {
        let mut config = Config::default();
        let result = config.set("sensitivity.multiplier", "not_a_number");
        assert!(result.is_err());
    }

    #[test]
    fn test_set_bool_true_variants() {
        let mut config = Config::default();
        for v in &["true", "1", "yes", "on", "True", "TRUE"] {
            assert!(config.set("sensitivity.invert", v).is_ok());
            assert!(config.sensitivity.invert);
        }
    }

    #[test]
    fn test_set_bool_false_variants() {
        let mut config = Config::default();
        config.sensitivity.invert = true;
        for v in &["false", "0", "no", "off", "False", "FALSE"] {
            config.sensitivity.invert = true;
            assert!(config.set("sensitivity.invert", v).is_ok());
            assert!(!config.sensitivity.invert);
        }
    }

    #[test]
    fn test_set_enum_valid() {
        let mut config = Config::default();
        assert!(config.set("volume.curve", "linear").is_ok());
        assert_eq!(config.volume.curve, "linear");
        assert!(config.set("volume.curve", "logarithmic").is_ok());
        assert!(config.set("volume.curve", "exponential").is_ok());
    }

    #[test]
    fn test_set_enum_invalid() {
        let mut config = Config::default();
        let result = config.set("volume.curve", "invalid_curve");
        assert!(result.is_err());
    }

    #[test]
    fn test_unknown_key() {
        let mut config = Config::default();
        let result = config.set("nonexistent.key", "value");
        assert!(matches!(result, Err(ConfigError::UnknownKey(_))));

        let result = config.get("nonexistent.key");
        assert!(matches!(result, Err(ConfigError::UnknownKey(_))));
    }

    #[test]
    fn test_save_and_load() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");

        let mut config = Config::default();
        config.volume.step_min = 5;
        config.volume.curve = "exponential".to_string();
        config.sensitivity.multiplier = 2.5;
        config.osd.enabled = false;

        config.save_to(&path).unwrap();

        let loaded = Config::load_from(&path).unwrap();
        assert_eq!(loaded.volume.step_min, 5);
        assert_eq!(loaded.volume.curve, "exponential");
        assert!((loaded.sensitivity.multiplier - 2.5).abs() < f64::EPSILON);
        assert!(!loaded.osd.enabled);
    }

    #[test]
    fn test_load_nonexistent() {
        let path = PathBuf::from("/nonexistent/path/config.toml");
        let result = Config::load_from(&path);
        assert!(matches!(result, Err(ConfigError::NotFound(_))));
    }

    #[test]
    fn test_toml_roundtrip() {
        let config = Config::default();
        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: Config = toml::from_str(&toml_str).unwrap();
        assert_eq!(config, parsed);
    }

    #[test]
    fn test_partial_toml() {
        // Config should handle partial TOML files, filling in defaults
        let partial_toml = r#"
[volume]
step_min = 5

[osd]
enabled = false
"#;
        let config: Config = toml::from_str(partial_toml).unwrap();
        assert_eq!(config.volume.step_min, 5);
        assert_eq!(config.volume.step_max, 8); // default
        assert!(!config.osd.enabled);
        assert_eq!(config.osd.timeout_ms, 1500); // default
    }

    #[test]
    fn test_config_path_not_empty() {
        let path = Config::path();
        assert!(!path.as_os_str().is_empty());
        assert!(path.ends_with("config.toml"));
    }

    #[test]
    fn test_data_dir_not_empty() {
        let path = Config::data_dir();
        assert!(!path.as_os_str().is_empty());
    }

    #[test]
    fn test_osd_position_enum() {
        let mut config = Config::default();
        let positions = ["center-bottom", "center", "top-right", "top-left", "bottom-right", "bottom-left"];
        for pos in positions {
            assert!(config.set("osd.position", pos).is_ok());
            assert_eq!(config.osd.position, pos);
        }
    }

    #[test]
    fn test_log_level_enum() {
        let mut config = Config::default();
        let levels = ["error", "warn", "info", "debug", "trace"];
        for level in levels {
            assert!(config.set("daemon.log_level", level).is_ok());
            assert_eq!(config.daemon.log_level, level);
        }
    }

    #[test]
    fn test_interaction_timing_ranges() {
        let mut config = Config::default();

        // Double click: 100-800
        assert!(config.set("interaction.double_click_ms", "100").is_ok());
        assert!(config.set("interaction.double_click_ms", "800").is_ok());
        assert!(config.set("interaction.double_click_ms", "99").is_err());
        assert!(config.set("interaction.double_click_ms", "801").is_err());

        // Triple click: 200-1000
        assert!(config.set("interaction.triple_click_ms", "200").is_ok());
        assert!(config.set("interaction.triple_click_ms", "1000").is_ok());
        assert!(config.set("interaction.triple_click_ms", "199").is_err());

        // Long press: 500-3000
        assert!(config.set("interaction.long_press_ms", "500").is_ok());
        assert!(config.set("interaction.long_press_ms", "3000").is_ok());
        assert!(config.set("interaction.long_press_ms", "499").is_err());
    }

    #[test]
    fn test_battery_warning_thresholds() {
        let config = Config::default();
        assert_eq!(config.battery.warning_thresholds, vec![20, 10, 5]);
    }

    #[test]
    fn test_audio_feedback_defaults() {
        let config = Config::default();
        assert!(!config.audio_feedback.enabled);
        assert!((config.audio_feedback.volume - 0.3).abs() < f64::EPSILON);
        assert!(config.audio_feedback.tick);
        assert!(config.audio_feedback.boundary);
    }

    #[test]
    fn test_microphone_curve_steepness() {
        let mut config = Config::default();
        assert!((config.microphone.curve_steepness - 2.0).abs() < f64::EPSILON);

        assert!(config.set("microphone.curve_steepness", "5.0").is_ok());
        assert!((config.microphone.curve_steepness - 5.0).abs() < f64::EPSILON);

        // Out of range
        assert!(config.set("microphone.curve_steepness", "0.5").is_err());
        assert!(config.set("microphone.curve_steepness", "25.0").is_err());
    }

    #[test]
    fn test_events_scripts_dir() {
        let mut config = Config::default();
        assert!(config.events.scripts_dir.is_none());
        assert_eq!(config.get("events.scripts_dir").unwrap(), "");

        assert!(config.set("events.scripts_dir", "/path/to/scripts").is_ok());
        assert_eq!(config.events.scripts_dir, Some("/path/to/scripts".to_string()));
        assert_eq!(config.get("events.scripts_dir").unwrap(), "/path/to/scripts");

        // Clear with empty string
        assert!(config.set("events.scripts_dir", "").is_ok());
        assert!(config.events.scripts_dir.is_none());
    }

    #[test]
    fn test_validate_defaults_pass() {
        let config = Config::default();
        let errors = config.validate();
        assert!(errors.is_empty(), "Default config should be valid: {:?}", errors);
    }

    #[test]
    fn test_validate_step_min_greater_than_max() {
        let mut config = Config::default();
        config.volume.step_min = 10;
        config.volume.step_max = 5;

        let errors = config.validate();
        assert!(!errors.is_empty());
        assert!(errors[0].contains("volume.step_min"));
    }

    #[test]
    fn test_validate_fast_ms_not_less_than_slow_ms() {
        let mut config = Config::default();
        config.acceleration.fast_ms = 500;
        config.acceleration.slow_ms = 100;

        let errors = config.validate();
        assert!(!errors.is_empty());
        assert!(errors[0].contains("acceleration.fast_ms"));
    }

    #[test]
    fn test_validate_interaction_timing_order() {
        let mut config = Config::default();
        // Make double_click >= triple_click (invalid)
        config.interaction.double_click_ms = 800;
        config.interaction.triple_click_ms = 600;

        let errors = config.validate();
        assert!(!errors.is_empty());
        assert!(errors.iter().any(|e| e.contains("double_click_ms")));
    }

    #[test]
    fn test_is_valid_convenience() {
        let config = Config::default();
        assert!(config.is_valid());

        let mut invalid = Config::default();
        invalid.volume.step_min = 20;
        invalid.volume.step_max = 1;
        assert!(!invalid.is_valid());
    }

    #[test]
    fn test_all_keys_can_be_set_and_got() {
        // Verify every key in keys() can be both get and set
        let mut config = Config::default();

        for key in Config::keys() {
            // First verify we can get the key
            let original = config.get(key);
            assert!(original.is_ok(), "Failed to get key '{}': {:?}", key, original);

            // Then verify we can set it back to the same value
            let value = original.unwrap();
            let result = config.set(key, &value);
            assert!(result.is_ok(), "Failed to set key '{}' to '{}': {:?}", key, value, result);

            // Verify the value round-tripped
            let new_value = config.get(key).unwrap();
            assert_eq!(value, new_value, "Value changed after set for key '{}'", key);
        }
    }

    #[test]
    fn test_keys_count_matches_expected() {
        // This test ensures we don't accidentally lose keys
        let keys = Config::keys();
        assert_eq!(keys.len(), 52, "Expected 52 config keys, got {}", keys.len());
    }

    #[test]
    fn test_validate_catches_invalid_enums() {
        // Simulate a manually edited config file with invalid enum values
        // (bypassing the set() method which validates)
        let mut config = Config::default();

        // Invalid volume curve
        config.volume.curve = "invalid_curve".to_string();
        let errors = config.validate();
        assert!(
            errors.iter().any(|e| e.contains("volume.curve")),
            "Should catch invalid volume.curve"
        );

        // Reset and test another enum
        config = Config::default();
        config.osd.position = "middle-ish".to_string();
        let errors = config.validate();
        assert!(
            errors.iter().any(|e| e.contains("osd.position")),
            "Should catch invalid osd.position"
        );

        // Test daemon.log_level
        config = Config::default();
        config.daemon.log_level = "verbose".to_string();
        let errors = config.validate();
        assert!(
            errors.iter().any(|e| e.contains("daemon.log_level")),
            "Should catch invalid daemon.log_level"
        );

        // Test media_control.triple_click_action
        config = Config::default();
        config.media_control.triple_click_action = "unknown_action".to_string();
        let errors = config.validate();
        assert!(
            errors.iter().any(|e| e.contains("triple_click_action")),
            "Should catch invalid triple_click_action"
        );
    }
}
