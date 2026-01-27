//! Mock HID device for testing
//!
//! Provides a simulated Surface Dial that can queue events and be controlled
//! programmatically for integration testing without physical hardware.

use super::{DialReport, HidDevice, HidError};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Events that can be queued on a mock HID device
#[derive(Debug, Clone)]
pub enum MockHidEvent {
    /// Rotation event (direction: negative = CCW, positive = CW)
    Rotation(i8),
    /// Button state change
    Button(bool),
    /// Raw HID report data
    RawReport(Vec<u8>),
}

/// Shared state for the mock device
#[derive(Debug)]
struct MockState {
    /// Queue of events to return
    events: VecDeque<MockHidEvent>,
    /// Simulated battery level (0-100)
    battery_level: u8,
    /// Whether device is "connected"
    connected: bool,
    /// Blocking mode
    blocking: bool,
}

/// A mock HID device for testing
///
/// This device can be cloned to share state between the test code
/// (which queues events) and the daemon code (which reads them).
#[derive(Debug, Clone)]
pub struct MockHidDevice {
    state: Arc<Mutex<MockState>>,
}

impl Default for MockHidDevice {
    fn default() -> Self {
        Self::new()
    }
}

impl MockHidDevice {
    /// Create a new mock HID device
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(MockState {
                events: VecDeque::new(),
                battery_level: 75,
                connected: true,
                blocking: false,
            })),
        }
    }

    /// Queue a rotation event
    ///
    /// Positive values = clockwise, negative = counter-clockwise
    pub fn queue_rotation(&self, direction: i8) {
        let mut state = self.state.lock().unwrap();
        state.events.push_back(MockHidEvent::Rotation(direction));
    }

    /// Queue multiple rotation events
    pub fn queue_rotations(&self, directions: &[i8]) {
        let mut state = self.state.lock().unwrap();
        for &dir in directions {
            state.events.push_back(MockHidEvent::Rotation(dir));
        }
    }

    /// Queue a button press/release
    pub fn queue_button(&self, pressed: bool) {
        let mut state = self.state.lock().unwrap();
        state.events.push_back(MockHidEvent::Button(pressed));
    }

    /// Queue a complete click (button down followed by button up)
    pub fn queue_click(&self) {
        self.queue_button(true);
        self.queue_button(false);
    }

    /// Queue a double-click
    pub fn queue_double_click(&self) {
        self.queue_click();
        self.queue_click();
    }

    /// Queue a triple-click
    pub fn queue_triple_click(&self) {
        self.queue_click();
        self.queue_click();
        self.queue_click();
    }

    /// Queue a long press start (button down, no release)
    pub fn queue_long_press_start(&self) {
        self.queue_button(true);
    }

    /// Queue a long press end (button release after long press)
    pub fn queue_long_press_end(&self) {
        self.queue_button(false);
    }

    /// Queue a raw HID report
    pub fn queue_raw_report(&self, data: Vec<u8>) {
        let mut state = self.state.lock().unwrap();
        state.events.push_back(MockHidEvent::RawReport(data));
    }

    /// Set the simulated battery level (0-100)
    pub fn set_battery(&self, level: u8) {
        let mut state = self.state.lock().unwrap();
        state.battery_level = level.min(100);
    }

    /// Get the current simulated battery level
    pub fn get_battery(&self) -> u8 {
        self.state.lock().unwrap().battery_level
    }

    /// Simulate device disconnection
    pub fn disconnect(&self) {
        let mut state = self.state.lock().unwrap();
        state.connected = false;
    }

    /// Simulate device reconnection
    pub fn reconnect(&self) {
        let mut state = self.state.lock().unwrap();
        state.connected = true;
    }

    /// Check if device is "connected"
    pub fn is_connected(&self) -> bool {
        self.state.lock().unwrap().connected
    }

    /// Get number of queued events
    pub fn pending_events(&self) -> usize {
        self.state.lock().unwrap().events.len()
    }

    /// Clear all queued events
    pub fn clear_events(&self) {
        let mut state = self.state.lock().unwrap();
        state.events.clear();
    }
}

impl HidDevice for MockHidDevice {
    fn read_timeout(&self, buf: &mut [u8], timeout: Duration) -> Result<usize, HidError> {
        let mut state = self.state.lock().unwrap();

        if !state.connected {
            return Err(HidError::Disconnected);
        }

        if let Some(event) = state.events.pop_front() {
            let report = match event {
                MockHidEvent::Rotation(dir) => DialReport::new(false, dir),
                MockHidEvent::Button(pressed) => DialReport::new(pressed, 0),
                MockHidEvent::RawReport(data) => {
                    let len = data.len().min(buf.len());
                    buf[..len].copy_from_slice(&data[..len]);
                    return Ok(len);
                }
            };

            let bytes = report.to_bytes();
            let len = bytes.len().min(buf.len());
            buf[..len].copy_from_slice(&bytes[..len]);
            Ok(len)
        } else {
            // No event - simulate timeout
            // In tests, we don't actually sleep for the full timeout
            if timeout > Duration::ZERO && !state.blocking {
                std::thread::sleep(Duration::from_millis(1));
            }
            Err(HidError::Timeout)
        }
    }

    fn write(&self, _data: &[u8]) -> Result<usize, HidError> {
        let state = self.state.lock().unwrap();
        if !state.connected {
            return Err(HidError::Disconnected);
        }
        Ok(0)
    }

    fn get_feature_report(&self, buf: &mut [u8]) -> Result<usize, HidError> {
        let state = self.state.lock().unwrap();

        if !state.connected {
            return Err(HidError::Disconnected);
        }

        // Return battery level in feature report format
        // Format: [report_id, battery_level, charging_status]
        buf[0] = 0x05; // Battery report ID
        buf[1] = state.battery_level;
        buf[2] = 0x00; // Not charging
        Ok(3)
    }

    fn set_blocking_mode(&self, blocking: bool) -> Result<(), HidError> {
        let mut state = self.state.lock().unwrap();
        state.blocking = blocking;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_device_creation() {
        let device = MockHidDevice::new();
        assert!(device.is_connected());
        assert_eq!(device.get_battery(), 75);
        assert_eq!(device.pending_events(), 0);
    }

    #[test]
    fn test_queue_rotation() {
        let device = MockHidDevice::new();
        device.queue_rotation(5);
        device.queue_rotation(-3);
        assert_eq!(device.pending_events(), 2);

        let mut buf = [0u8; 64];

        // Read first rotation
        let len = device.read_timeout(&mut buf, Duration::from_millis(10)).unwrap();
        assert_eq!(len, 3);
        let report = DialReport::parse(&buf[..len]).unwrap();
        assert_eq!(report.rotation, 5);
        assert!(!report.button_pressed);

        // Read second rotation
        let len = device.read_timeout(&mut buf, Duration::from_millis(10)).unwrap();
        let report = DialReport::parse(&buf[..len]).unwrap();
        assert_eq!(report.rotation, -3);
    }

    #[test]
    fn test_queue_click() {
        let device = MockHidDevice::new();
        device.queue_click();
        assert_eq!(device.pending_events(), 2); // down + up

        let mut buf = [0u8; 64];

        // Button down
        let len = device.read_timeout(&mut buf, Duration::from_millis(10)).unwrap();
        let report = DialReport::parse(&buf[..len]).unwrap();
        assert!(report.button_pressed);
        assert_eq!(report.rotation, 0);

        // Button up
        let len = device.read_timeout(&mut buf, Duration::from_millis(10)).unwrap();
        let report = DialReport::parse(&buf[..len]).unwrap();
        assert!(!report.button_pressed);
    }

    #[test]
    fn test_disconnect() {
        let device = MockHidDevice::new();
        device.disconnect();

        let mut buf = [0u8; 64];
        let result = device.read_timeout(&mut buf, Duration::from_millis(10));
        assert!(matches!(result, Err(HidError::Disconnected)));
    }

    #[test]
    fn test_timeout_on_empty() {
        let device = MockHidDevice::new();
        let mut buf = [0u8; 64];

        let result = device.read_timeout(&mut buf, Duration::from_millis(1));
        assert!(matches!(result, Err(HidError::Timeout)));
    }

    #[test]
    fn test_battery_level() {
        let device = MockHidDevice::new();
        device.set_battery(25);

        let mut buf = [0u8; 64];
        let len = device.get_feature_report(&mut buf).unwrap();
        assert_eq!(len, 3);
        assert_eq!(buf[0], 0x05); // Battery report ID
        assert_eq!(buf[1], 25);   // Battery level
    }

    #[test]
    fn test_clone_shares_state() {
        let device1 = MockHidDevice::new();
        let device2 = device1.clone();

        device1.queue_rotation(10);
        assert_eq!(device2.pending_events(), 1);

        device2.disconnect();
        assert!(!device1.is_connected());
    }

    #[test]
    fn test_double_click() {
        let device = MockHidDevice::new();
        device.queue_double_click();
        // Double click = 2 clicks = 4 events (down, up, down, up)
        assert_eq!(device.pending_events(), 4);
    }

    #[test]
    fn test_triple_click() {
        let device = MockHidDevice::new();
        device.queue_triple_click();
        // Triple click = 3 clicks = 6 events
        assert_eq!(device.pending_events(), 6);
    }
}
