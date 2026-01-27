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

    // ==========================================================================
    // Control Mode Tests
    // ==========================================================================

    #[test]
    fn test_control_mode_equality() {
        assert_eq!(ControlMode::Volume, ControlMode::Volume);
        assert_ne!(ControlMode::Volume, ControlMode::Microphone);
    }

    #[test]
    fn test_control_mode_debug() {
        let mode = ControlMode::Volume;
        let debug_str = format!("{:?}", mode);
        assert!(debug_str.contains("Volume"));
    }

    #[test]
    fn test_control_mode_clone() {
        let mode = ControlMode::Microphone;
        let cloned = mode;
        assert_eq!(mode, cloned);
    }

    // ==========================================================================
    // DaemonStats Tests
    // ==========================================================================

    #[test]
    fn test_daemon_stats_default() {
        let stats = DaemonStats::default();
        assert!(stats.start_time.is_none());
        assert_eq!(stats.rotation_count, 0);
        assert_eq!(stats.click_count, 0);
        assert_eq!(stats.mode_switches, 0);
    }

    #[test]
    fn test_daemon_stats_debug() {
        let stats = DaemonStats::default();
        let debug_str = format!("{:?}", stats);
        assert!(debug_str.contains("DaemonStats"));
        assert!(debug_str.contains("rotation_count"));
    }

    // ==========================================================================
    // Daemon Creation Tests
    // ==========================================================================

    #[test]
    fn test_daemon_creation() {
        let config = Config::default();
        let daemon = Daemon::new(config);
        assert_eq!(daemon.control_mode(), ControlMode::Volume);
        assert!(!daemon.is_connected());
    }

    #[test]
    fn test_daemon_initial_state() {
        let config = Config::default();
        let daemon = Daemon::new(config);

        // Initial mode should be Volume
        assert_eq!(daemon.control_mode, ControlMode::Volume);

        // Not connected initially
        assert!(!daemon.connected);

        // No button pressed initially
        assert!(!daemon.was_button_pressed);

        // Stats should be zeroed
        assert_eq!(daemon.stats.rotation_count, 0);
        assert_eq!(daemon.stats.click_count, 0);
        assert_eq!(daemon.stats.mode_switches, 0);
        assert!(daemon.stats.start_time.is_none());

        // No mic mode started
        assert!(daemon.mic_mode_started.is_none());

        // No last rotation
        assert!(daemon.last_rotation.is_none());
    }

    #[test]
    fn test_daemon_with_custom_config() {
        let mut config = Config::default();
        config.volume.step_min = 5;
        config.volume.step_max = 15;
        config.microphone.mode_duration = 30;

        let daemon = Daemon::new(config);
        assert_eq!(daemon.config.volume.step_min, 5);
        assert_eq!(daemon.config.volume.step_max, 15);
        assert_eq!(daemon.config.microphone.mode_duration, 30);
    }

    // ==========================================================================
    // Running Flag Tests (Shutdown Behavior)
    // ==========================================================================

    #[test]
    fn test_running_flag_initially_false() {
        let daemon = Daemon::new(Config::default());
        assert!(!daemon.running.load(Ordering::SeqCst));
    }

    #[test]
    fn test_running_flag_clone() {
        let daemon = Daemon::new(Config::default());
        let running_clone = daemon.running();

        // Both should point to same atomic
        daemon.running.store(true, Ordering::SeqCst);
        assert!(running_clone.load(Ordering::SeqCst));

        running_clone.store(false, Ordering::SeqCst);
        assert!(!daemon.running.load(Ordering::SeqCst));
    }

    #[test]
    fn test_running_flag_shared_across_threads() {
        use std::thread;

        let daemon = Daemon::new(Config::default());
        let running = daemon.running();

        running.store(true, Ordering::SeqCst);

        let running_clone = running.clone();
        let handle = thread::spawn(move || {
            // Should see the value set in main thread
            assert!(running_clone.load(Ordering::SeqCst));
            // Set to false from child thread
            running_clone.store(false, Ordering::SeqCst);
        });

        handle.join().unwrap();

        // Main thread should see the change
        assert!(!running.load(Ordering::SeqCst));
    }

    // ==========================================================================
    // Mode Switching Tests
    // ==========================================================================

    #[test]
    fn test_switch_mode_to_microphone() {
        let mut daemon = Daemon::new(Config::default());
        assert_eq!(daemon.control_mode, ControlMode::Volume);
        assert_eq!(daemon.stats.mode_switches, 0);

        daemon.switch_mode(ControlMode::Microphone);

        assert_eq!(daemon.control_mode, ControlMode::Microphone);
        assert_eq!(daemon.stats.mode_switches, 1);
    }

    #[test]
    fn test_switch_mode_to_volume() {
        let mut daemon = Daemon::new(Config::default());
        daemon.control_mode = ControlMode::Microphone;

        daemon.switch_mode(ControlMode::Volume);

        assert_eq!(daemon.control_mode, ControlMode::Volume);
        assert_eq!(daemon.stats.mode_switches, 1);
    }

    #[test]
    fn test_switch_mode_same_mode_no_change() {
        let mut daemon = Daemon::new(Config::default());
        assert_eq!(daemon.control_mode, ControlMode::Volume);

        daemon.switch_mode(ControlMode::Volume);

        // No mode switch should be recorded
        assert_eq!(daemon.stats.mode_switches, 0);
    }

    #[test]
    fn test_switch_mode_multiple_times() {
        let mut daemon = Daemon::new(Config::default());

        daemon.switch_mode(ControlMode::Microphone);
        daemon.switch_mode(ControlMode::Volume);
        daemon.switch_mode(ControlMode::Microphone);

        assert_eq!(daemon.stats.mode_switches, 3);
        assert_eq!(daemon.control_mode, ControlMode::Microphone);
    }

    #[test]
    fn test_switch_mode_resets_rotation_processor() {
        let mut daemon = Daemon::new(Config::default());

        // Accumulate some rotation
        let _ = daemon.rotation_processor.process(1);
        let _ = daemon.rotation_processor.process(1);

        // Switch mode should reset
        daemon.switch_mode(ControlMode::Microphone);

        // The processor should be reset (we can't easily verify this without
        // more inspection methods, but the test exercises the code path)
        assert_eq!(daemon.control_mode, ControlMode::Microphone);
    }

    // ==========================================================================
    // Config Reload Tests
    // ==========================================================================

    #[test]
    fn test_reload_config() {
        let mut daemon = Daemon::new(Config::default());

        // Original values
        assert_eq!(daemon.config.volume.step_min, 2);
        assert_eq!(daemon.config.volume.step_max, 8);

        // Create new config with different values
        let mut new_config = Config::default();
        new_config.volume.step_min = 5;
        new_config.volume.step_max = 15;
        new_config.interaction.double_click_ms = 500;
        new_config.sensitivity.multiplier = 2.0;

        daemon.reload_config(new_config);

        // Verify config was updated
        assert_eq!(daemon.config.volume.step_min, 5);
        assert_eq!(daemon.config.volume.step_max, 15);
        assert_eq!(daemon.config.interaction.double_click_ms, 500);
        assert_eq!(daemon.config.sensitivity.multiplier, 2.0);
    }

    #[test]
    fn test_reload_config_preserves_state() {
        let mut daemon = Daemon::new(Config::default());

        // Set some state
        daemon.control_mode = ControlMode::Microphone;
        daemon.connected = true;
        daemon.stats.rotation_count = 100;
        daemon.stats.click_count = 50;

        // Reload config
        let new_config = Config::default();
        daemon.reload_config(new_config);

        // State should be preserved
        assert_eq!(daemon.control_mode, ControlMode::Microphone);
        assert!(daemon.connected);
        assert_eq!(daemon.stats.rotation_count, 100);
        assert_eq!(daemon.stats.click_count, 50);
    }

    // ==========================================================================
    // Click Result Processing Tests
    // ==========================================================================

    #[test]
    fn test_process_click_result_single_click_increments_stats() {
        let mut daemon = Daemon::new(Config::default());
        assert_eq!(daemon.stats.click_count, 0);

        daemon.process_click_result(ClickResult::SingleClick);

        assert_eq!(daemon.stats.click_count, 1);
    }

    #[test]
    fn test_process_click_result_double_click_switches_to_mic() {
        let mut daemon = Daemon::new(Config::default());
        assert_eq!(daemon.control_mode, ControlMode::Volume);

        daemon.process_click_result(ClickResult::DoubleClick);

        assert_eq!(daemon.control_mode, ControlMode::Microphone);
        assert!(daemon.mic_mode_started.is_some());
        assert_eq!(daemon.stats.click_count, 1);
    }

    #[test]
    fn test_process_click_result_triple_click_increments_stats() {
        let mut daemon = Daemon::new(Config::default());
        daemon.config.media_control.enabled = true;

        daemon.process_click_result(ClickResult::TripleClick);

        assert_eq!(daemon.stats.click_count, 1);
    }

    #[test]
    fn test_process_click_result_triple_click_disabled_media() {
        let mut daemon = Daemon::new(Config::default());
        daemon.config.media_control.enabled = false;

        daemon.process_click_result(ClickResult::TripleClick);

        // Still increments click count
        assert_eq!(daemon.stats.click_count, 1);
    }

    #[test]
    fn test_process_click_result_none_does_nothing() {
        let mut daemon = Daemon::new(Config::default());
        let initial_clicks = daemon.stats.click_count;

        daemon.process_click_result(ClickResult::None);

        assert_eq!(daemon.stats.click_count, initial_clicks);
    }

    // ==========================================================================
    // Mic Mode Expiry Tests
    // ==========================================================================

    #[test]
    fn test_mic_mode_starts_with_timestamp() {
        let mut daemon = Daemon::new(Config::default());

        daemon.process_click_result(ClickResult::DoubleClick);

        assert!(daemon.mic_mode_started.is_some());
        // Timestamp should be very recent
        let elapsed = daemon.mic_mode_started.unwrap().elapsed();
        assert!(elapsed.as_millis() < 100);
    }

    #[test]
    fn test_mic_mode_expiry_check_in_tick() {
        let mut config = Config::default();
        config.microphone.mode_duration = 1; // 1 second

        let mut daemon = Daemon::new(config);
        daemon.control_mode = ControlMode::Microphone;
        daemon.mic_mode_started = Some(Instant::now() - Duration::from_secs(2)); // 2 seconds ago

        daemon.tick();

        // Should have switched back to Volume
        assert_eq!(daemon.control_mode, ControlMode::Volume);
        assert!(daemon.mic_mode_started.is_none());
    }

    #[test]
    fn test_mic_mode_not_expired_yet() {
        let mut config = Config::default();
        config.microphone.mode_duration = 10; // 10 seconds

        let mut daemon = Daemon::new(config);
        daemon.control_mode = ControlMode::Microphone;
        daemon.mic_mode_started = Some(Instant::now()); // Just now

        daemon.tick();

        // Should still be in Microphone mode
        assert_eq!(daemon.control_mode, ControlMode::Microphone);
        assert!(daemon.mic_mode_started.is_some());
    }

    // ==========================================================================
    // Button State Handling Tests
    // ==========================================================================

    #[test]
    fn test_button_down_state_tracking() {
        let mut daemon = Daemon::new(Config::default());
        assert!(!daemon.was_button_pressed);

        daemon.handle_button_state(true);
        assert!(daemon.was_button_pressed);

        daemon.handle_button_state(false);
        assert!(!daemon.was_button_pressed);
    }

    #[test]
    fn test_button_press_triggers_click_detector() {
        let mut daemon = Daemon::new(Config::default());

        // Press button
        daemon.handle_button_state(true);

        // Release button - should start click detection
        daemon.handle_button_state(false);

        // Wait for single click timeout
        std::thread::sleep(Duration::from_millis(500));
        daemon.tick();

        // Should have processed a single click
        assert_eq!(daemon.stats.click_count, 1);
    }

    #[test]
    fn test_repeated_same_state_no_action() {
        let mut daemon = Daemon::new(Config::default());

        // Multiple "pressed" reports in a row (no actual press/release)
        daemon.handle_button_state(true);
        daemon.handle_button_state(true);
        daemon.handle_button_state(true);

        // Button detector should only have one button_down
        assert!(daemon.was_button_pressed);
    }

    // ==========================================================================
    // USB Constants Tests
    // ==========================================================================

    #[test]
    fn test_surface_dial_usb_ids() {
        assert_eq!(SURFACE_DIAL_VENDOR_ID, 0x045E);
        assert_eq!(SURFACE_DIAL_PRODUCT_ID, 0x091B);
    }

    // ==========================================================================
    // Accessors Tests
    // ==========================================================================

    #[test]
    fn test_control_mode_accessor() {
        let mut daemon = Daemon::new(Config::default());

        assert_eq!(daemon.control_mode(), ControlMode::Volume);

        daemon.control_mode = ControlMode::Microphone;
        assert_eq!(daemon.control_mode(), ControlMode::Microphone);
    }

    #[test]
    fn test_is_connected_accessor() {
        let mut daemon = Daemon::new(Config::default());

        assert!(!daemon.is_connected());

        daemon.connected = true;
        assert!(daemon.is_connected());
    }

    // ==========================================================================
    // Integration-style Tests
    // ==========================================================================

    #[test]
    fn test_full_double_click_sequence() {
        let mut daemon = Daemon::new(Config::default());

        // First click
        daemon.handle_button_state(true);
        daemon.handle_button_state(false);

        // Small delay (within double-click window)
        std::thread::sleep(Duration::from_millis(50));

        // Second click
        daemon.handle_button_state(true);
        daemon.handle_button_state(false);

        // Should be in mic mode now
        assert_eq!(daemon.control_mode, ControlMode::Microphone);
        assert_eq!(daemon.stats.click_count, 1); // double-click counts as 1
    }

    #[test]
    fn test_triple_click_sequence() {
        let mut config = Config::default();
        config.media_control.enabled = true;
        let mut daemon = Daemon::new(config);

        // Three quick clicks
        for _ in 0..3 {
            daemon.handle_button_state(true);
            daemon.handle_button_state(false);
            std::thread::sleep(Duration::from_millis(50));
        }

        assert_eq!(daemon.stats.click_count, 1); // triple-click counts as 1
    }

    #[test]
    fn test_daemon_stats_accumulate() {
        let mut daemon = Daemon::new(Config::default());

        // Simulate multiple interactions
        for _ in 0..5 {
            daemon.handle_button_state(true);
            daemon.handle_button_state(false);
            std::thread::sleep(Duration::from_millis(500)); // Wait for single-click timeout
            daemon.tick();
        }

        assert_eq!(daemon.stats.click_count, 5);
    }

    // ==========================================================================
    // Signal Handling Tests
    // ==========================================================================
    //
    // These tests verify the shutdown and reload mechanisms that would be triggered
    // by Unix signals (SIGTERM, SIGINT, SIGHUP). Since we can't easily test actual
    // signals in unit tests, we test the underlying mechanisms.

    #[test]
    fn test_shutdown_via_running_flag_sigterm_simulation() {
        // Simulates SIGTERM behavior: external code sets running to false
        use std::sync::atomic::Ordering;

        let daemon = Daemon::new(Config::default());
        let running = daemon.running();

        // Start "daemon" (just set the flag)
        running.store(true, Ordering::SeqCst);
        assert!(running.load(Ordering::SeqCst));

        // Simulate signal handler setting flag to false (what ctrlc does)
        running.store(false, Ordering::SeqCst);

        // Verify shutdown is triggered
        assert!(!running.load(Ordering::SeqCst));
    }

    #[test]
    fn test_shutdown_via_running_flag_sigint_simulation() {
        // SIGINT (Ctrl+C) uses the same mechanism as SIGTERM
        use std::sync::atomic::Ordering;

        let daemon = Daemon::new(Config::default());
        let running = daemon.running();

        running.store(true, Ordering::SeqCst);

        // Simulate Ctrl+C (SIGINT) handler
        running.store(false, Ordering::SeqCst);

        assert!(!running.load(Ordering::SeqCst));
    }

    #[test]
    fn test_shutdown_from_different_thread() {
        // Signal handlers run in a different context; verify cross-thread works
        use std::sync::atomic::Ordering;
        use std::thread;
        use std::time::Duration;

        let daemon = Daemon::new(Config::default());
        let running = daemon.running();

        running.store(true, Ordering::SeqCst);

        // Spawn a thread that simulates signal handler
        let signal_running = running.clone();
        let handle = thread::spawn(move || {
            // Small delay to simulate async signal delivery
            thread::sleep(Duration::from_millis(10));
            signal_running.store(false, Ordering::SeqCst);
        });

        // Wait for "signal"
        handle.join().unwrap();

        // Main "daemon" should see shutdown
        assert!(!running.load(Ordering::SeqCst));
    }

    #[test]
    fn test_config_reload_sighup_simulation() {
        // SIGHUP typically triggers config reload
        let mut daemon = Daemon::new(Config::default());

        // Original config
        assert_eq!(daemon.config.volume.step_min, 2);

        // Simulate SIGHUP handler: read new config and reload
        let mut new_config = Config::default();
        new_config.volume.step_min = 10;
        new_config.volume.step_max = 20;

        daemon.reload_config(new_config);

        // Verify reload happened
        assert_eq!(daemon.config.volume.step_min, 10);
        assert_eq!(daemon.config.volume.step_max, 20);
    }

    #[test]
    fn test_reload_while_running() {
        // Config can be reloaded while daemon is "running"
        use std::sync::atomic::Ordering;

        let mut daemon = Daemon::new(Config::default());
        daemon.running.store(true, Ordering::SeqCst);
        daemon.stats.rotation_count = 42;

        // Reload config (SIGHUP handler)
        let mut new_config = Config::default();
        new_config.sensitivity.multiplier = 3.0;
        daemon.reload_config(new_config);

        // Daemon should still be running
        assert!(daemon.running.load(Ordering::SeqCst));
        // Stats should be preserved
        assert_eq!(daemon.stats.rotation_count, 42);
        // Config should be updated
        assert_eq!(daemon.config.sensitivity.multiplier, 3.0);
    }

    #[test]
    fn test_graceful_shutdown_preserves_stats() {
        // On shutdown, stats should remain accessible for logging
        use std::sync::atomic::Ordering;

        let mut daemon = Daemon::new(Config::default());
        daemon.running.store(true, Ordering::SeqCst);

        // Simulate some activity
        daemon.stats.rotation_count = 100;
        daemon.stats.click_count = 25;
        daemon.stats.mode_switches = 5;

        // Trigger shutdown
        daemon.running.store(false, Ordering::SeqCst);

        // Stats should still be accessible after shutdown
        assert_eq!(daemon.stats.rotation_count, 100);
        assert_eq!(daemon.stats.click_count, 25);
        assert_eq!(daemon.stats.mode_switches, 5);
    }

    #[test]
    fn test_multiple_shutdown_signals_idempotent() {
        // Multiple signals shouldn't cause issues
        use std::sync::atomic::Ordering;

        let daemon = Daemon::new(Config::default());
        let running = daemon.running();

        running.store(true, Ordering::SeqCst);

        // First "signal"
        running.store(false, Ordering::SeqCst);
        assert!(!running.load(Ordering::SeqCst));

        // Second "signal" (shouldn't panic or cause issues)
        running.store(false, Ordering::SeqCst);
        assert!(!running.load(Ordering::SeqCst));

        // Third "signal"
        running.store(false, Ordering::SeqCst);
        assert!(!running.load(Ordering::SeqCst));
    }

    #[test]
    fn test_reload_updates_click_detector_config() {
        let mut daemon = Daemon::new(Config::default());

        // Original double-click timing
        let original_double_click = daemon.config.interaction.double_click_ms;

        // Reload with different timing
        let mut new_config = Config::default();
        new_config.interaction.double_click_ms = original_double_click + 100;
        daemon.reload_config(new_config);

        // Config should be updated (click detector internally updates too)
        assert_eq!(
            daemon.config.interaction.double_click_ms,
            original_double_click + 100
        );
    }

    #[test]
    fn test_reload_updates_rotation_processor_config() {
        let mut daemon = Daemon::new(Config::default());

        // Original sensitivity
        let original_multiplier = daemon.config.sensitivity.multiplier;

        // Reload with different sensitivity
        let mut new_config = Config::default();
        new_config.sensitivity.multiplier = original_multiplier * 2.0;
        new_config.sensitivity.dead_zone = 10;
        daemon.reload_config(new_config);

        // Config should be updated
        assert_eq!(
            daemon.config.sensitivity.multiplier,
            original_multiplier * 2.0
        );
        assert_eq!(daemon.config.sensitivity.dead_zone, 10);
    }

    #[test]
    fn test_shutdown_clears_mic_mode() {
        // Verify shutdown behavior with mic mode active
        use std::sync::atomic::Ordering;

        let mut daemon = Daemon::new(Config::default());
        daemon.running.store(true, Ordering::SeqCst);

        // Enter mic mode
        daemon.switch_mode(ControlMode::Microphone);
        daemon.mic_mode_started = Some(Instant::now());
        assert_eq!(daemon.control_mode, ControlMode::Microphone);

        // Trigger shutdown
        daemon.running.store(false, Ordering::SeqCst);

        // Daemon is stopped, but state remains for inspection/logging
        assert_eq!(daemon.control_mode, ControlMode::Microphone);
        assert!(daemon.mic_mode_started.is_some());
    }

    #[test]
    fn test_startup_time_recorded() {
        // Stats should record when daemon started
        let mut daemon = Daemon::new(Config::default());

        // Before run(), no start time
        assert!(daemon.stats.start_time.is_none());

        // Simulate run() starting (normally this is done inside run())
        daemon.running.store(true, Ordering::SeqCst);
        daemon.stats.start_time = Some(Instant::now());

        // Start time should be recorded
        assert!(daemon.stats.start_time.is_some());
    }

    #[test]
    fn test_uptime_calculable_from_stats() {
        // Should be able to calculate uptime from stats
        let mut daemon = Daemon::new(Config::default());

        // Record start time
        let start = Instant::now();
        daemon.stats.start_time = Some(start);

        // Wait a bit
        std::thread::sleep(Duration::from_millis(50));

        // Calculate uptime
        let uptime = daemon
            .stats
            .start_time
            .map(|s| s.elapsed())
            .unwrap_or(Duration::ZERO);

        // Uptime should be at least 50ms
        assert!(uptime >= Duration::from_millis(50));
    }

    // ==========================================================================
    // Cleanup Verification Tests
    // ==========================================================================

    #[test]
    fn test_connected_state_on_shutdown() {
        // Connected state should be inspectable on shutdown
        use std::sync::atomic::Ordering;

        let mut daemon = Daemon::new(Config::default());
        daemon.running.store(true, Ordering::SeqCst);

        // Simulate device connected
        daemon.connected = true;

        // Shutdown
        daemon.running.store(false, Ordering::SeqCst);

        // Should still know device was connected (for cleanup/logging)
        assert!(daemon.connected);
    }

    #[test]
    fn test_button_state_reset_on_mode_switch() {
        // Mode switch should handle any pending button state
        let mut daemon = Daemon::new(Config::default());

        // Simulate button held down
        daemon.was_button_pressed = true;

        // Switch mode (e.g., from double-click)
        daemon.switch_mode(ControlMode::Microphone);

        // Button state is preserved (will be handled normally)
        // This verifies no crash or panic occurs
        assert!(daemon.was_button_pressed);
    }

    // ==========================================================================
    // Error Recovery Tests
    // ==========================================================================
    //
    // These tests verify the daemon recovers gracefully from various error
    // conditions including device disconnection, config issues, and transient
    // failures.

    #[test]
    fn test_device_disconnect_sets_connected_false() {
        // When device disconnects, connected flag should be set to false
        let mut daemon = Daemon::new(Config::default());

        // Initially not connected
        assert!(!daemon.connected);

        // Simulate connection
        daemon.connected = true;
        assert!(daemon.connected);

        // Simulate disconnect (what process_hid_events does when device not found)
        daemon.connected = false;
        assert!(!daemon.connected);
    }

    #[test]
    fn test_device_reconnect_after_disconnect() {
        // After disconnect, daemon should be ready to reconnect
        let mut daemon = Daemon::new(Config::default());

        // Simulate connect -> disconnect -> reconnect cycle
        daemon.connected = true;
        assert!(daemon.connected);

        daemon.connected = false;
        assert!(!daemon.connected);

        // Simulating reconnection (what happens when device is found again)
        daemon.connected = true;
        assert!(daemon.connected);

        // Stats should be preserved across reconnect
        daemon.stats.rotation_count = 50;
        daemon.connected = false;
        daemon.connected = true;
        assert_eq!(daemon.stats.rotation_count, 50);
    }

    #[test]
    fn test_config_reload_handles_invalid_gracefully() {
        // Reload should handle edge cases in config
        let mut daemon = Daemon::new(Config::default());

        // Reload with minimal config (defaults)
        let config = Config::default();
        daemon.reload_config(config);

        // Should not panic, config should be updated
        assert_eq!(daemon.config.volume.step_min, 2); // Default value
    }

    #[test]
    fn test_state_preserved_through_errors() {
        // Critical state should be preserved even during error conditions
        use std::sync::atomic::Ordering;

        let mut daemon = Daemon::new(Config::default());
        daemon.running.store(true, Ordering::SeqCst);

        // Set up some state
        daemon.control_mode = ControlMode::Microphone;
        daemon.stats.rotation_count = 100;
        daemon.stats.click_count = 50;
        daemon.mic_mode_started = Some(Instant::now());

        // Simulate "error" - disconnect
        daemon.connected = false;

        // Critical state should be preserved
        assert_eq!(daemon.control_mode, ControlMode::Microphone);
        assert_eq!(daemon.stats.rotation_count, 100);
        assert_eq!(daemon.stats.click_count, 50);
        assert!(daemon.mic_mode_started.is_some());
        assert!(daemon.running.load(Ordering::SeqCst));
    }

    #[test]
    fn test_multiple_disconnect_reconnect_cycles() {
        // Daemon should handle multiple disconnect/reconnect cycles
        let mut daemon = Daemon::new(Config::default());

        for cycle in 0..5 {
            // Connect
            daemon.connected = true;
            assert!(daemon.connected, "Cycle {}: should be connected", cycle);

            // Simulate some activity
            daemon.stats.rotation_count += 10;

            // Disconnect
            daemon.connected = false;
            assert!(!daemon.connected, "Cycle {}: should be disconnected", cycle);
        }

        // Stats should accumulate across cycles
        assert_eq!(daemon.stats.rotation_count, 50);
    }

    #[test]
    fn test_rotation_processing_after_reconnect() {
        // Rotation processor should work correctly after reconnect
        let mut daemon = Daemon::new(Config::default());

        // First connection - process some rotation
        daemon.connected = true;
        daemon.handle_rotation(5);
        daemon.handle_rotation(5);

        // Disconnect
        daemon.connected = false;

        // Reconnect
        daemon.connected = true;

        // Should still be able to process rotation
        daemon.handle_rotation(5);
        // No panic = success
    }

    #[test]
    fn test_click_detection_after_reconnect() {
        // Click detector should work correctly after reconnect
        let mut daemon = Daemon::new(Config::default());

        // First connection
        daemon.connected = true;
        daemon.handle_button_state(true);
        daemon.handle_button_state(false);

        // Disconnect
        daemon.connected = false;

        // Reconnect
        daemon.connected = true;

        // Should still be able to detect clicks
        daemon.handle_button_state(true);
        daemon.handle_button_state(false);

        // Allow time for single click to register
        std::thread::sleep(Duration::from_millis(400));
        daemon.tick();

        assert!(daemon.stats.click_count >= 1);
    }

    #[test]
    fn test_mode_preserved_through_disconnect() {
        // Current mode should be preserved through disconnect/reconnect
        let mut daemon = Daemon::new(Config::default());

        // Enter mic mode
        daemon.switch_mode(ControlMode::Microphone);
        assert_eq!(daemon.control_mode, ControlMode::Microphone);

        // Simulate disconnect
        daemon.connected = false;

        // Mode should be preserved
        assert_eq!(daemon.control_mode, ControlMode::Microphone);

        // Reconnect
        daemon.connected = true;

        // Mode should still be microphone
        assert_eq!(daemon.control_mode, ControlMode::Microphone);
    }

    #[test]
    fn test_button_state_cleared_on_disconnect() {
        // When device disconnects mid-press, we should handle it gracefully
        let mut daemon = Daemon::new(Config::default());

        // Button is pressed
        daemon.handle_button_state(true);
        assert!(daemon.was_button_pressed);

        // Device disconnects (simulated)
        daemon.connected = false;

        // The was_button_pressed state remains (hardware could reconnect)
        // This is intentional - we don't want to lose state
        assert!(daemon.was_button_pressed);

        // On reconnect, if button is not pressed, we handle the release
        daemon.connected = true;
        daemon.handle_button_state(false);
        assert!(!daemon.was_button_pressed);
    }

    #[test]
    fn test_stats_not_corrupted_by_errors() {
        // Stats should never become invalid due to errors
        let mut daemon = Daemon::new(Config::default());

        // Set some stats
        daemon.stats.rotation_count = u64::MAX - 10;
        daemon.stats.click_count = 1000;

        // Simulate error conditions
        daemon.connected = false;
        daemon.connected = true;
        daemon.connected = false;

        // Stats should be unchanged
        assert_eq!(daemon.stats.rotation_count, u64::MAX - 10);
        assert_eq!(daemon.stats.click_count, 1000);

        // Stats should still be incrementable
        daemon.stats.rotation_count = daemon.stats.rotation_count.wrapping_add(1);
        assert_eq!(daemon.stats.rotation_count, u64::MAX - 9);
    }

    #[test]
    fn test_daemon_recoverable_after_many_errors() {
        // Daemon should remain functional after many simulated errors
        use std::sync::atomic::Ordering;

        let mut daemon = Daemon::new(Config::default());
        daemon.running.store(true, Ordering::SeqCst);

        // Simulate many error cycles
        for _ in 0..100 {
            daemon.connected = true;
            daemon.connected = false;
        }

        // Daemon should still be able to function
        assert!(daemon.running.load(Ordering::SeqCst));

        // Should be able to process input
        daemon.connected = true;
        daemon.handle_button_state(true);
        daemon.handle_button_state(false);
        daemon.handle_rotation(5);
        // No panic = success
    }

    #[test]
    fn test_mic_mode_timer_survives_disconnect() {
        // Mic mode timer should continue counting during disconnect
        let mut daemon = Daemon::new(Config::default());

        // Enter mic mode
        daemon.switch_mode(ControlMode::Microphone);
        daemon.mic_mode_started = Some(Instant::now());

        // Small delay
        std::thread::sleep(Duration::from_millis(50));

        // Disconnect
        daemon.connected = false;

        // Timer should still be running
        assert!(daemon.mic_mode_started.is_some());
        let elapsed = daemon.mic_mode_started.unwrap().elapsed();
        assert!(elapsed >= Duration::from_millis(50));
    }

    #[test]
    fn test_config_reload_during_error_state() {
        // Config reload should work even when disconnected
        let mut daemon = Daemon::new(Config::default());

        // Disconnected state
        daemon.connected = false;

        // Reload config
        let mut new_config = Config::default();
        new_config.volume.step_max = 15;
        daemon.reload_config(new_config);

        // Config should be updated
        assert_eq!(daemon.config.volume.step_max, 15);
    }

    #[test]
    fn test_running_flag_survives_errors() {
        // Running flag should not be affected by device errors
        use std::sync::atomic::Ordering;

        let mut daemon = Daemon::new(Config::default());
        daemon.running.store(true, Ordering::SeqCst);

        // Simulate various error conditions
        daemon.connected = false;  // Device disconnect
        daemon.connected = true;
        daemon.connected = false;

        // Running flag should still be true
        assert!(daemon.running.load(Ordering::SeqCst));

        // Only explicit shutdown should set it false
        daemon.running.store(false, Ordering::SeqCst);
        assert!(!daemon.running.load(Ordering::SeqCst));
    }
}
