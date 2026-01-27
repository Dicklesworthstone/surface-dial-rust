//! Mock platform implementation for testing
//!
//! Records all platform calls for verification in tests, without affecting
//! the actual system.

use super::{
    AppInfo, AudioDevice, DeviceType, Key, MediaKey, Platform, PlatformError, PlatformResult,
    Urgency,
};
use std::sync::{atomic::AtomicBool, atomic::AtomicI32, atomic::AtomicU32, atomic::Ordering, Mutex};

/// A mock platform that records all calls for testing
///
/// Uses atomic types and mutexes to be thread-safe (Sync) as required by Platform trait.
#[derive(Debug)]
pub struct MockPlatform {
    // Volume state (atomics for thread safety)
    volume: AtomicI32,
    muted: AtomicBool,
    mic_volume: AtomicI32,
    mic_muted: AtomicBool,

    // Call recording (mutex protected)
    notifications: Mutex<Vec<NotificationRecord>>,
    media_keys: Mutex<Vec<MediaKey>>,
    keys_down: Mutex<Vec<Key>>,
    keys_up: Mutex<Vec<Key>>,
    volume_changes: Mutex<Vec<i32>>,
    mic_volume_changes: Mutex<Vec<i32>>,
    mute_toggles: AtomicU32,
    mic_mute_toggles: AtomicU32,

    // Error simulation
    force_error: AtomicBool,

    // Foreground app simulation
    foreground_app: Mutex<Option<AppInfo>>,
}

/// Record of a notification that was sent
#[derive(Debug, Clone)]
pub struct NotificationRecord {
    pub title: String,
    pub body: String,
    pub urgency: Urgency,
}

impl Default for MockPlatform {
    fn default() -> Self {
        Self::new()
    }
}

impl MockPlatform {
    /// Create a new mock platform with default state
    pub fn new() -> Self {
        Self {
            volume: AtomicI32::new(50),
            muted: AtomicBool::new(false),
            mic_volume: AtomicI32::new(50),
            mic_muted: AtomicBool::new(false),
            notifications: Mutex::new(Vec::new()),
            media_keys: Mutex::new(Vec::new()),
            keys_down: Mutex::new(Vec::new()),
            keys_up: Mutex::new(Vec::new()),
            volume_changes: Mutex::new(Vec::new()),
            mic_volume_changes: Mutex::new(Vec::new()),
            mute_toggles: AtomicU32::new(0),
            mic_mute_toggles: AtomicU32::new(0),
            force_error: AtomicBool::new(false),
            foreground_app: Mutex::new(None),
        }
    }

    /// Set initial volume
    pub fn with_volume(self, vol: i32) -> Self {
        self.volume.store(vol, Ordering::SeqCst);
        self
    }

    /// Set initial mic volume
    pub fn with_mic_volume(self, vol: i32) -> Self {
        self.mic_volume.store(vol, Ordering::SeqCst);
        self
    }

    /// Set initial mute state
    pub fn with_muted(self, muted: bool) -> Self {
        self.muted.store(muted, Ordering::SeqCst);
        self
    }

    /// Force all operations to return an error
    pub fn set_force_error(&self, force: bool) {
        self.force_error.store(force, Ordering::SeqCst);
    }

    /// Set the simulated foreground app
    pub fn set_foreground_app(&self, app: Option<AppInfo>) {
        *self.foreground_app.lock().unwrap() = app;
    }

    // === Inspection methods for tests ===

    /// Get all notifications that were sent
    pub fn notifications_sent(&self) -> Vec<NotificationRecord> {
        self.notifications.lock().unwrap().clone()
    }

    /// Get all media keys that were sent
    pub fn media_keys_sent(&self) -> Vec<MediaKey> {
        self.media_keys.lock().unwrap().clone()
    }

    /// Get all keys that received down events
    pub fn keys_pressed(&self) -> Vec<Key> {
        self.keys_down.lock().unwrap().clone()
    }

    /// Get all keys that received up events
    pub fn keys_released(&self) -> Vec<Key> {
        self.keys_up.lock().unwrap().clone()
    }

    /// Get history of volume changes
    pub fn volume_history(&self) -> Vec<i32> {
        self.volume_changes.lock().unwrap().clone()
    }

    /// Get history of mic volume changes
    pub fn mic_volume_history(&self) -> Vec<i32> {
        self.mic_volume_changes.lock().unwrap().clone()
    }

    /// Get number of mute toggles
    pub fn mute_toggle_count(&self) -> u32 {
        self.mute_toggles.load(Ordering::SeqCst)
    }

    /// Get number of mic mute toggles
    pub fn mic_mute_toggle_count(&self) -> u32 {
        self.mic_mute_toggles.load(Ordering::SeqCst)
    }

    /// Clear all recorded calls
    pub fn clear_history(&self) {
        self.notifications.lock().unwrap().clear();
        self.media_keys.lock().unwrap().clear();
        self.keys_down.lock().unwrap().clear();
        self.keys_up.lock().unwrap().clear();
        self.volume_changes.lock().unwrap().clear();
        self.mic_volume_changes.lock().unwrap().clear();
        self.mute_toggles.store(0, Ordering::SeqCst);
        self.mic_mute_toggles.store(0, Ordering::SeqCst);
    }

    fn check_error(&self) -> PlatformResult<()> {
        if self.force_error.load(Ordering::SeqCst) {
            Err(PlatformError::OperationFailed(
                "Simulated error".to_string(),
            ))
        } else {
            Ok(())
        }
    }
}

impl Platform for MockPlatform {
    fn get_volume(&self) -> PlatformResult<i32> {
        self.check_error()?;
        Ok(self.volume.load(Ordering::SeqCst))
    }

    fn set_volume(&self, vol: i32) -> PlatformResult<()> {
        self.check_error()?;
        let clamped = vol.clamp(0, 100);
        self.volume.store(clamped, Ordering::SeqCst);
        self.volume_changes.lock().unwrap().push(clamped);
        Ok(())
    }

    fn is_muted(&self) -> PlatformResult<bool> {
        self.check_error()?;
        Ok(self.muted.load(Ordering::SeqCst))
    }

    fn toggle_mute(&self) -> PlatformResult<()> {
        self.check_error()?;
        // Atomic fetch_xor for thread-safe toggle
        self.muted.fetch_xor(true, Ordering::SeqCst);
        self.mute_toggles.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn get_mic_volume(&self) -> PlatformResult<i32> {
        self.check_error()?;
        Ok(self.mic_volume.load(Ordering::SeqCst))
    }

    fn set_mic_volume(&self, vol: i32) -> PlatformResult<()> {
        self.check_error()?;
        let clamped = vol.clamp(0, 100);
        self.mic_volume.store(clamped, Ordering::SeqCst);
        self.mic_volume_changes.lock().unwrap().push(clamped);
        Ok(())
    }

    fn is_mic_muted(&self) -> PlatformResult<bool> {
        self.check_error()?;
        Ok(self.mic_muted.load(Ordering::SeqCst))
    }

    fn toggle_mic_mute(&self) -> PlatformResult<()> {
        self.check_error()?;
        self.mic_muted.fetch_xor(true, Ordering::SeqCst);
        self.mic_mute_toggles.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn send_key_down(&self, key: Key) -> PlatformResult<()> {
        self.check_error()?;
        self.keys_down.lock().unwrap().push(key);
        Ok(())
    }

    fn send_key_up(&self, key: Key) -> PlatformResult<()> {
        self.check_error()?;
        self.keys_up.lock().unwrap().push(key);
        Ok(())
    }

    fn send_media_key(&self, key: MediaKey) -> PlatformResult<()> {
        self.check_error()?;
        self.media_keys.lock().unwrap().push(key);
        Ok(())
    }

    fn list_output_devices(&self) -> PlatformResult<Vec<AudioDevice>> {
        self.check_error()?;
        Ok(vec![AudioDevice {
            id: "mock-speakers".to_string(),
            name: "Mock Speakers".to_string(),
            device_type: DeviceType::Speakers,
            is_default: true,
        }])
    }

    fn get_default_output(&self) -> PlatformResult<AudioDevice> {
        self.check_error()?;
        Ok(AudioDevice {
            id: "mock-speakers".to_string(),
            name: "Mock Speakers".to_string(),
            device_type: DeviceType::Speakers,
            is_default: true,
        })
    }

    fn set_default_output(&self, _device_id: &str) -> PlatformResult<()> {
        self.check_error()?;
        Ok(())
    }

    fn list_input_devices(&self) -> PlatformResult<Vec<AudioDevice>> {
        self.check_error()?;
        Ok(vec![AudioDevice {
            id: "mock-mic".to_string(),
            name: "Mock Microphone".to_string(),
            device_type: DeviceType::Unknown,
            is_default: true,
        }])
    }

    fn send_notification(
        &self,
        title: &str,
        body: &str,
        urgency: Urgency,
    ) -> PlatformResult<()> {
        self.check_error()?;
        self.notifications.lock().unwrap().push(NotificationRecord {
            title: title.to_string(),
            body: body.to_string(),
            urgency,
        });
        Ok(())
    }

    fn get_foreground_app(&self) -> PlatformResult<Option<AppInfo>> {
        self.check_error()?;
        Ok(self.foreground_app.lock().unwrap().clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_platform_creation() {
        let platform = MockPlatform::new();
        assert_eq!(platform.get_volume().unwrap(), 50);
        assert!(!platform.is_muted().unwrap());
    }

    #[test]
    fn test_volume_control() {
        let platform = MockPlatform::new();

        platform.set_volume(75).unwrap();
        assert_eq!(platform.get_volume().unwrap(), 75);

        // Check history
        assert_eq!(platform.volume_history(), vec![75]);
    }

    #[test]
    fn test_volume_clamping() {
        let platform = MockPlatform::new();

        platform.set_volume(150).unwrap();
        assert_eq!(platform.get_volume().unwrap(), 100);

        platform.set_volume(-10).unwrap();
        assert_eq!(platform.get_volume().unwrap(), 0);
    }

    #[test]
    fn test_mute_toggle() {
        let platform = MockPlatform::new();

        platform.toggle_mute().unwrap();
        assert!(platform.is_muted().unwrap());
        assert_eq!(platform.mute_toggle_count(), 1);

        platform.toggle_mute().unwrap();
        assert!(!platform.is_muted().unwrap());
        assert_eq!(platform.mute_toggle_count(), 2);
    }

    #[test]
    fn test_media_keys() {
        let platform = MockPlatform::new();

        platform.send_media_key(MediaKey::PlayPause).unwrap();
        platform.send_media_key(MediaKey::NextTrack).unwrap();

        let sent = platform.media_keys_sent();
        assert_eq!(sent.len(), 2);
        assert_eq!(sent[0], MediaKey::PlayPause);
        assert_eq!(sent[1], MediaKey::NextTrack);
    }

    #[test]
    fn test_key_events() {
        let platform = MockPlatform::new();

        platform.send_key_down(Key::F15).unwrap();
        platform.send_key_up(Key::F15).unwrap();

        assert_eq!(platform.keys_pressed(), vec![Key::F15]);
        assert_eq!(platform.keys_released(), vec![Key::F15]);
    }

    #[test]
    fn test_notifications() {
        let platform = MockPlatform::new();

        platform
            .send_notification("Test", "Hello World", Urgency::Normal)
            .unwrap();

        let notifications = platform.notifications_sent();
        assert_eq!(notifications.len(), 1);
        assert_eq!(notifications[0].title, "Test");
        assert_eq!(notifications[0].body, "Hello World");
    }

    #[test]
    fn test_force_error() {
        let platform = MockPlatform::new();
        platform.set_force_error(true);

        assert!(platform.get_volume().is_err());
        assert!(platform.set_volume(50).is_err());
        assert!(platform.toggle_mute().is_err());
    }

    #[test]
    fn test_clear_history() {
        let platform = MockPlatform::new();

        platform.set_volume(75).unwrap();
        platform.toggle_mute().unwrap();
        platform.send_media_key(MediaKey::PlayPause).unwrap();

        platform.clear_history();

        assert!(platform.volume_history().is_empty());
        assert_eq!(platform.mute_toggle_count(), 0);
        assert!(platform.media_keys_sent().is_empty());
    }

    #[test]
    fn test_builder_pattern() {
        let platform = MockPlatform::new()
            .with_volume(80)
            .with_mic_volume(60)
            .with_muted(true);

        assert_eq!(platform.get_volume().unwrap(), 80);
        assert_eq!(platform.get_mic_volume().unwrap(), 60);
        assert!(platform.is_muted().unwrap());
    }

    #[test]
    fn test_thread_safety() {
        use std::sync::Arc;
        use std::thread;

        let platform = Arc::new(MockPlatform::new());
        let mut handles = Vec::new();

        // Spawn multiple threads doing operations
        for i in 0..4 {
            let p = Arc::clone(&platform);
            handles.push(thread::spawn(move || {
                for _ in 0..10 {
                    p.set_volume(i * 10).unwrap();
                    let _ = p.get_volume();
                    p.toggle_mute().unwrap();
                }
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        // Should have recorded 40 mute toggles total
        assert_eq!(platform.mute_toggle_count(), 40);
    }
}
