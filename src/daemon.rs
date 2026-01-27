//! Surface Dial daemon implementation
//!
//! This module contains the main daemon loop that handles HID input from the
//! Surface Dial and translates it into volume control, mute toggles, and other
//! actions using the platform abstraction layer.

use crate::config::Config;
use crate::input::{calculate_step, ClickConfig, ClickDetector, ClickResult, RotationProcessor};
use crate::platform::{CurrentPlatform, Key, Platform};
use hidapi::HidApi;
use log::{debug, error, info, warn};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Microsoft Surface Dial USB identifiers
pub const SURFACE_DIAL_VENDOR_ID: u16 = 0x045E;
pub const SURFACE_DIAL_PRODUCT_ID: u16 = 0x091B;

/// Control mode for the dial
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlMode {
    /// Controlling system volume
    Volume,
    /// Controlling microphone volume
    Microphone,
}

/// Statistics about daemon operation
#[derive(Debug, Default)]
pub struct DaemonStats {
    /// When the daemon started
    pub start_time: Option<Instant>,
    /// Total rotation events processed
    pub rotation_count: u64,
    /// Total click events processed
    pub click_count: u64,
    /// Number of mode switches
    pub mode_switches: u64,
}

/// The main daemon that handles Surface Dial input
pub struct Daemon {
    /// Application configuration
    config: Config,
    /// Platform-specific operations
    platform: CurrentPlatform,
    /// Flag to signal shutdown
    running: Arc<AtomicBool>,

    // Control state
    /// Current control mode (volume or mic)
    control_mode: ControlMode,
    /// When mic mode started (for auto-expiry)
    mic_mode_started: Option<Instant>,

    // Input processing
    /// Click pattern detector
    click_detector: ClickDetector,
    /// Rotation sensitivity processor
    rotation_processor: RotationProcessor,
    /// Time of last rotation (for acceleration)
    last_rotation: Option<Instant>,

    // HID state
    /// Whether button was pressed in last frame
    was_button_pressed: bool,
    /// Whether dial is currently connected
    connected: bool,

    /// Operation statistics
    pub stats: DaemonStats,
}

impl Daemon {
    /// Create a new daemon with the given configuration
    pub fn new(config: Config) -> Self {
        let platform = CurrentPlatform::new();

        // Initialize click detector from config
        let click_config = ClickConfig::from_config(&config.interaction);
        let click_detector = ClickDetector::new(click_config);

        // Initialize rotation processor from config
        let sensitivity_config =
            crate::input::SensitivityConfig::from_config(&config.sensitivity);
        let rotation_processor = RotationProcessor::new(sensitivity_config);

        Self {
            config,
            platform,
            running: Arc::new(AtomicBool::new(false)),
            control_mode: ControlMode::Volume,
            mic_mode_started: None,
            click_detector,
            rotation_processor,
            last_rotation: None,
            was_button_pressed: false,
            connected: false,
            stats: DaemonStats::default(),
        }
    }

    /// Get a clone of the running flag for signal handling
    pub fn running(&self) -> Arc<AtomicBool> {
        self.running.clone()
    }

    /// Run the daemon main loop
    pub fn run(&mut self) {
        self.running.store(true, Ordering::SeqCst);
        self.stats.start_time = Some(Instant::now());

        info!(
            "Daemon starting (volume curve={}, mic_duration={}s)",
            self.config.volume.curve, self.config.microphone.mode_duration
        );

        while self.running.load(Ordering::SeqCst) {
            self.tick();
            self.process_hid_events();

            // Small sleep to prevent busy-waiting when no data
            std::thread::sleep(Duration::from_millis(1));
        }

        info!(
            "Daemon stopped. Stats: rotations={}, clicks={}",
            self.stats.rotation_count, self.stats.click_count
        );
    }

    /// Periodic tick for time-based state updates
    fn tick(&mut self) {
        // Check mic mode expiry
        if self.control_mode == ControlMode::Microphone {
            if let Some(started) = self.mic_mode_started {
                let duration = Duration::from_secs(self.config.microphone.mode_duration as u64);
                if started.elapsed() >= duration {
                    info!("Mic mode expired, returning to volume control");
                    self.switch_mode(ControlMode::Volume);
                    self.mic_mode_started = None;
                }
            }
        }

        // Process click detector timeouts
        let click_result = self.click_detector.tick();
        self.process_click_result(click_result);
    }

    /// Process HID events from the Surface Dial
    fn process_hid_events(&mut self) {
        // Try to get HID API
        let Ok(api) = HidApi::new() else {
            if self.connected {
                warn!("HID API unavailable");
                self.connected = false;
            }
            std::thread::sleep(Duration::from_secs(1));
            return;
        };

        // Find and open dial devices
        let devices = self.find_and_open_dial(&api);
        if devices.is_empty() {
            if self.connected {
                info!("Surface Dial disconnected. Waiting for reconnection...");
                self.connected = false;
            }
            std::thread::sleep(Duration::from_millis(500));
            return;
        }

        if !self.connected {
            info!(
                "Surface Dial connected! {} interface(s) open.",
                devices.len()
            );
            self.connected = true;
        }

        // Read events from all interfaces
        let mut buf = [0u8; 64];

        for device in &devices {
            if !self.running.load(Ordering::SeqCst) {
                break;
            }

            match device.read_timeout(&mut buf, 10) {
                Ok(len) if len >= 3 && buf[0] == 0x01 => {
                    // Standard dial report
                    let button_pressed = (buf[1] & 0x01) != 0;
                    let rotation = buf[2] as i8;

                    self.handle_button_state(button_pressed);

                    if rotation != 0 {
                        self.handle_rotation(rotation);
                    }
                }
                Ok(_) => {
                    // No data or unrecognized report
                }
                Err(e) => {
                    debug!("HID read error: {}", e);
                    // Device error - will reconnect on next iteration
                    break;
                }
            }
        }
    }

    /// Find and open all Surface Dial HID devices
    fn find_and_open_dial(&self, api: &HidApi) -> Vec<hidapi::HidDevice> {
        let mut devices = Vec::new();

        for dev in api.device_list() {
            if dev.vendor_id() != SURFACE_DIAL_VENDOR_ID
                || dev.product_id() != SURFACE_DIAL_PRODUCT_ID
            {
                continue;
            }

            if let Ok(device) = dev.open_device(api) {
                let _ = device.set_blocking_mode(false);
                devices.push(device);
            }
        }

        devices
    }

    /// Handle button state changes
    fn handle_button_state(&mut self, button_pressed: bool) {
        if button_pressed && !self.was_button_pressed {
            // Button just pressed
            let result = self.click_detector.button_down();
            self.process_click_result(result);
        } else if !button_pressed && self.was_button_pressed {
            // Button just released
            let result = self.click_detector.button_up();
            self.process_click_result(result);
        }

        self.was_button_pressed = button_pressed;
    }

    /// Handle rotation input
    fn handle_rotation(&mut self, raw_rotation: i8) {
        // Check for device switching mode (long-press + rotate)
        if self.click_detector.is_long_pressing() {
            // Could implement device switching here
            debug!("Long press + rotation: device switching not yet implemented");
            return;
        }

        // Apply sensitivity/dead zone
        let adjusted = match self.rotation_processor.process(raw_rotation) {
            Some(v) => v,
            None => return, // Within dead zone
        };

        // Get current volume based on mode
        let (current_volume, step_min, step_max) = match self.control_mode {
            ControlMode::Volume => (
                self.platform.get_volume().ok(),
                self.config.volume.step_min,
                self.config.volume.step_max,
            ),
            ControlMode::Microphone => {
                // Reset mic mode timer on rotation
                self.mic_mode_started = Some(Instant::now());
                (
                    self.platform.get_mic_volume().ok(),
                    self.config.microphone.step_min,
                    self.config.microphone.step_max,
                )
            }
        };

        if let Some(current) = current_volume {
            // Calculate step based on rotation speed
            let step = calculate_step(
                self.last_rotation,
                step_min,
                step_max,
                self.config.acceleration.fast_ms as u64,
                self.config.acceleration.slow_ms as u64,
            );

            let direction = adjusted.signum();
            let delta = direction * step;
            let new_vol = (current + delta).clamp(0, 100);

            if new_vol != current {
                let result = match self.control_mode {
                    ControlMode::Volume => self.platform.set_volume(new_vol),
                    ControlMode::Microphone => self.platform.set_mic_volume(new_vol),
                };

                match result {
                    Ok(()) => {
                        let mode_name = match self.control_mode {
                            ControlMode::Volume => "Volume",
                            ControlMode::Microphone => "Mic",
                        };
                        info!("{}: {}%", mode_name, new_vol);
                    }
                    Err(e) => error!("Failed to set volume: {}", e),
                }
            }
        }

        self.last_rotation = Some(Instant::now());
        self.stats.rotation_count += 1;
    }

    /// Process a click detection result
    fn process_click_result(&mut self, result: ClickResult) {
        match result {
            ClickResult::SingleClick => {
                info!("Single click: toggle mute");
                self.handle_mute_toggle();
                self.stats.click_count += 1;
            }
            ClickResult::DoubleClick => {
                info!("Double click: switch to mic mode");
                self.switch_mode(ControlMode::Microphone);
                self.mic_mode_started = Some(Instant::now());
                self.stats.click_count += 1;
            }
            ClickResult::TripleClick => {
                if self.config.media_control.enabled {
                    info!("Triple click: media play/pause");
                    self.handle_media_control();
                }
                self.stats.click_count += 1;
            }
            ClickResult::LongPressStart => {
                info!("Long press: F15 down");
                if let Err(e) = self.platform.send_key_down(Key::F15) {
                    error!("Failed to send F15 down: {}", e);
                }
            }
            ClickResult::LongPressEnd => {
                info!("Long press release: F15 up");
                if let Err(e) = self.platform.send_key_up(Key::F15) {
                    error!("Failed to send F15 up: {}", e);
                }
            }
            ClickResult::None => {}
        }
    }

    /// Handle mute toggle based on current mode
    fn handle_mute_toggle(&mut self) {
        let result = match self.control_mode {
            ControlMode::Volume => self.platform.toggle_mute(),
            ControlMode::Microphone => self.platform.toggle_mic_mute(),
        };

        if let Err(e) = result {
            error!("Failed to toggle mute: {}", e);
        }
    }

    /// Switch control mode
    fn switch_mode(&mut self, new_mode: ControlMode) {
        if self.control_mode != new_mode {
            self.control_mode = new_mode;
            self.rotation_processor.reset();
            self.stats.mode_switches += 1;
        }
    }

    /// Handle media control (triple-click action)
    fn handle_media_control(&mut self) {
        use crate::platform::MediaKey;

        let key = match self.config.media_control.triple_click_action.as_str() {
            "play_pause" => MediaKey::PlayPause,
            "next_track" => MediaKey::NextTrack,
            "prev_track" => MediaKey::PrevTrack,
            _ => MediaKey::PlayPause,
        };

        if let Err(e) = self.platform.send_media_key(key) {
            error!("Failed to send media key: {}", e);
        }
    }

    /// Reload configuration (for hot reload support)
    pub fn reload_config(&mut self, new_config: Config) {
        info!("Reloading configuration...");

        // Update click detector
        let click_config = ClickConfig::from_config(&new_config.interaction);
        self.click_detector.update_config(click_config);

        // Update rotation processor
        let sensitivity_config =
            crate::input::SensitivityConfig::from_config(&new_config.sensitivity);
        self.rotation_processor.update_config(sensitivity_config);

        self.config = new_config;
        info!("Configuration reloaded");
    }

    /// Get the current control mode
    pub fn control_mode(&self) -> ControlMode {
        self.control_mode
    }

    /// Check if the dial is connected
    pub fn is_connected(&self) -> bool {
        self.connected
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_control_mode_equality() {
        assert_eq!(ControlMode::Volume, ControlMode::Volume);
        assert_ne!(ControlMode::Volume, ControlMode::Microphone);
    }

    #[test]
    fn test_daemon_stats_default() {
        let stats = DaemonStats::default();
        assert!(stats.start_time.is_none());
        assert_eq!(stats.rotation_count, 0);
        assert_eq!(stats.click_count, 0);
        assert_eq!(stats.mode_switches, 0);
    }

    #[test]
    fn test_daemon_creation() {
        let config = Config::default();
        let daemon = Daemon::new(config);
        assert_eq!(daemon.control_mode(), ControlMode::Volume);
        assert!(!daemon.is_connected());
    }
}
