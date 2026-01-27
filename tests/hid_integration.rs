//! HID device mock integration tests
//!
//! Tests the mock HID device behavior and dial report parsing.

use surface_dial::hid::mock::MockHidDevice;
use surface_dial::hid::{DialReport, HidDevice, HidError};
use std::time::Duration;

#[test]
fn test_mock_hid_rotation_sequence() {
    let device = MockHidDevice::new();

    // Queue a sequence of rotations
    device.queue_rotations(&[1, 2, -1, -3, 5]);

    let mut buf = [0u8; 64];
    let mut rotations = Vec::new();

    // Read all queued rotations
    while device.pending_events() > 0 {
        let len = device.read_timeout(&mut buf, Duration::from_millis(10)).unwrap();
        let report = DialReport::parse(&buf[..len]).unwrap();
        rotations.push(report.rotation);
    }

    assert_eq!(rotations, vec![1, 2, -1, -3, 5]);
}

#[test]
fn test_mock_hid_click_sequence() {
    let device = MockHidDevice::new();

    // Queue a click
    device.queue_click();

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
    assert_eq!(report.rotation, 0);

    // No more events
    assert!(matches!(
        device.read_timeout(&mut buf, Duration::from_millis(1)),
        Err(HidError::Timeout)
    ));
}

#[test]
fn test_mock_hid_disconnect_reconnect() {
    let device = MockHidDevice::new();
    device.queue_rotation(5);

    let mut buf = [0u8; 64];

    // Disconnect
    device.disconnect();
    assert!(matches!(
        device.read_timeout(&mut buf, Duration::from_millis(10)),
        Err(HidError::Disconnected)
    ));

    // Reconnect
    device.reconnect();

    // Event should still be there
    let len = device.read_timeout(&mut buf, Duration::from_millis(10)).unwrap();
    let report = DialReport::parse(&buf[..len]).unwrap();
    assert_eq!(report.rotation, 5);
}

#[test]
fn test_mock_hid_battery_report() {
    let device = MockHidDevice::new();
    device.set_battery(42);

    let mut buf = [0u8; 64];
    let len = device.get_feature_report(&mut buf).unwrap();

    assert_eq!(len, 3);
    assert_eq!(buf[0], 0x05); // Battery report ID
    assert_eq!(buf[1], 42);   // Battery level
    assert_eq!(buf[2], 0x00); // Not charging
}

#[test]
fn test_mock_hid_shared_state() {
    // Verify that cloning shares state
    let device1 = MockHidDevice::new();
    let device2 = device1.clone();

    // Queue event on device1
    device1.queue_rotation(10);

    // Read from device2
    let mut buf = [0u8; 64];
    let len = device2.read_timeout(&mut buf, Duration::from_millis(10)).unwrap();
    let report = DialReport::parse(&buf[..len]).unwrap();
    assert_eq!(report.rotation, 10);

    // Event should be consumed for device1 too
    assert_eq!(device1.pending_events(), 0);
}

#[test]
fn test_dial_report_extreme_values() {
    // Test maximum positive rotation
    let report = DialReport::new(false, 127);
    assert_eq!(report.rotation, 127);

    let bytes = report.to_bytes();
    let parsed = DialReport::parse(&bytes).unwrap();
    assert_eq!(parsed.rotation, 127);

    // Test maximum negative rotation
    let report = DialReport::new(false, -127);
    assert_eq!(report.rotation, -127);

    let bytes = report.to_bytes();
    let parsed = DialReport::parse(&bytes).unwrap();
    assert_eq!(parsed.rotation, -127);
}

#[test]
fn test_dial_report_combined_events() {
    // Real dial can send button + rotation in same report
    let report = DialReport::new(true, 3);
    assert!(report.button_pressed);
    assert_eq!(report.rotation, 3);

    let bytes = report.to_bytes();
    let parsed = DialReport::parse(&bytes).unwrap();
    assert!(parsed.button_pressed);
    assert_eq!(parsed.rotation, 3);
}

#[test]
fn test_multiple_quick_rotations() {
    let device = MockHidDevice::new();

    // Simulate rapid rotation (e.g., 10 ticks)
    for _ in 0..10 {
        device.queue_rotation(1);
    }

    let mut buf = [0u8; 64];
    let mut count = 0;

    while device.pending_events() > 0 {
        let _ = device.read_timeout(&mut buf, Duration::from_millis(10)).unwrap();
        count += 1;
    }

    assert_eq!(count, 10);
}

#[test]
fn test_raw_report_passthrough() {
    let device = MockHidDevice::new();

    // Queue a custom raw report (e.g., some proprietary data)
    let custom_data = vec![0x02, 0xFF, 0xAB, 0xCD];
    device.queue_raw_report(custom_data.clone());

    let mut buf = [0u8; 64];
    let len = device.read_timeout(&mut buf, Duration::from_millis(10)).unwrap();

    assert_eq!(len, 4);
    assert_eq!(&buf[..4], &custom_data[..]);
}
