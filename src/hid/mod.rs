//! HID device abstraction module
//!
//! Provides a trait-based abstraction for HID device access, enabling
//! mock implementations for testing without physical hardware.

use std::time::Duration;
use thiserror::Error;

/// Mock HID devices for testing (always available for integration tests)
pub mod mock;

/// Errors that can occur during HID operations
#[derive(Debug, Error)]
pub enum HidError {
    /// I/O error during read/write
    #[error("IO error: {0}")]
    Io(String),
    /// Device was disconnected
    #[error("Device disconnected")]
    Disconnected,
    /// Read operation timed out with no data
    #[error("Timeout")]
    Timeout,
    /// Device not found
    #[error("Device not found")]
    NotFound,
    /// HID API initialization failed
    #[error("API error: {0}")]
    ApiError(String),
}

impl From<hidapi::HidError> for HidError {
    fn from(e: hidapi::HidError) -> Self {
        HidError::Io(e.to_string())
    }
}

/// Trait abstracting HID device operations
///
/// This trait allows for mock implementations in tests while using
/// real HID devices in production.
pub trait HidDevice: Send {
    /// Read data with timeout
    ///
    /// Returns the number of bytes read, or an error if the read failed.
    /// Returns `HidError::Timeout` if no data was available within the timeout.
    fn read_timeout(&self, buf: &mut [u8], timeout: Duration) -> Result<usize, HidError>;

    /// Write data to the device
    fn write(&self, data: &[u8]) -> Result<usize, HidError>;

    /// Get a feature report from the device
    fn get_feature_report(&self, buf: &mut [u8]) -> Result<usize, HidError>;

    /// Set blocking mode
    fn set_blocking_mode(&self, blocking: bool) -> Result<(), HidError>;
}

/// Wrapper around hidapi::HidDevice implementing our trait
pub struct RealHidDevice {
    device: hidapi::HidDevice,
}

impl RealHidDevice {
    /// Create a new wrapper around a hidapi device
    pub fn new(device: hidapi::HidDevice) -> Self {
        Self { device }
    }
}

impl HidDevice for RealHidDevice {
    fn read_timeout(&self, buf: &mut [u8], timeout: Duration) -> Result<usize, HidError> {
        let timeout_ms = timeout.as_millis() as i32;
        match self.device.read_timeout(buf, timeout_ms) {
            Ok(0) => Err(HidError::Timeout),
            Ok(n) => Ok(n),
            Err(e) => Err(HidError::Io(e.to_string())),
        }
    }

    fn write(&self, data: &[u8]) -> Result<usize, HidError> {
        self.device.write(data).map_err(|e| HidError::Io(e.to_string()))
    }

    fn get_feature_report(&self, buf: &mut [u8]) -> Result<usize, HidError> {
        self.device
            .get_feature_report(buf)
            .map_err(|e| HidError::Io(e.to_string()))
    }

    fn set_blocking_mode(&self, blocking: bool) -> Result<(), HidError> {
        self.device
            .set_blocking_mode(blocking)
            .map_err(|e| HidError::Io(e.to_string()))
    }
}

/// Surface Dial HID report structure
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DialReport {
    /// Report ID (should be 0x01 for standard dial reports)
    pub report_id: u8,
    /// Button state (bit 0 = pressed)
    pub button_pressed: bool,
    /// Rotation value (-127 to 127)
    pub rotation: i8,
}

impl DialReport {
    /// Parse a dial report from raw HID data
    ///
    /// Returns None if the data is not a valid dial report.
    pub fn parse(buf: &[u8]) -> Option<Self> {
        if buf.len() < 3 || buf[0] != 0x01 {
            return None;
        }

        Some(Self {
            report_id: buf[0],
            button_pressed: (buf[1] & 0x01) != 0,
            rotation: buf[2] as i8,
        })
    }

    /// Create a dial report with the given state
    pub fn new(button_pressed: bool, rotation: i8) -> Self {
        Self {
            report_id: 0x01,
            button_pressed,
            rotation,
        }
    }

    /// Encode the report to bytes
    pub fn to_bytes(&self) -> [u8; 3] {
        [
            self.report_id,
            if self.button_pressed { 0x01 } else { 0x00 },
            self.rotation as u8,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dial_report_parse_valid() {
        let buf = [0x01, 0x01, 0x05];
        let report = DialReport::parse(&buf).unwrap();
        assert!(report.button_pressed);
        assert_eq!(report.rotation, 5);
    }

    #[test]
    fn test_dial_report_parse_no_button() {
        let buf = [0x01, 0x00, 0xFB]; // -5 as u8
        let report = DialReport::parse(&buf).unwrap();
        assert!(!report.button_pressed);
        assert_eq!(report.rotation, -5);
    }

    #[test]
    fn test_dial_report_parse_invalid_report_id() {
        let buf = [0x02, 0x00, 0x00];
        assert!(DialReport::parse(&buf).is_none());
    }

    #[test]
    fn test_dial_report_parse_too_short() {
        let buf = [0x01, 0x00];
        assert!(DialReport::parse(&buf).is_none());
    }

    #[test]
    fn test_dial_report_roundtrip() {
        let original = DialReport::new(true, -10);
        let bytes = original.to_bytes();
        let parsed = DialReport::parse(&bytes).unwrap();
        assert_eq!(original, parsed);
    }
}
