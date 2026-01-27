//! Real-time Surface Dial reader - decode rotation and button events

use hidapi::HidApi;
use std::time::{Duration, Instant};

const VENDOR_ID: u16 = 0x045E;
const PRODUCT_ID: u16 = 0x091B;

fn main() {
    println!("Surface Dial Real-Time Reader");
    println!("==============================\n");

    let api = HidApi::new().expect("Failed to init HID API");

    // Find all Surface Dial interfaces
    let devices: Vec<_> = api.device_list()
        .filter(|d| d.vendor_id() == VENDOR_ID && d.product_id() == PRODUCT_ID)
        .collect();

    if devices.is_empty() {
        println!("Surface Dial not found! Make sure it's connected via Bluetooth.");
        return;
    }

    println!("Found {} interface(s). Will read from all of them.\n", devices.len());
    println!(">>> ROTATE THE DIAL AND PRESS THE BUTTON <<<\n");

    // Open all interfaces
    let mut handles = Vec::new();
    for dev in &devices {
        let page = dev.usage_page();
        let usage = dev.usage();
        if let Ok(device) = dev.open_device(&api) {
            let _ = device.set_blocking_mode(false);
            handles.push((device, page, usage));
        }
    }

    // Read loop
    let start = Instant::now();
    let mut last_data: Vec<Vec<u8>> = vec![Vec::new(); handles.len()];

    while start.elapsed() < Duration::from_secs(30) {
        for (i, (device, page, usage)) in handles.iter().enumerate() {
            let mut buf = [0u8; 64];
            if let Ok(len) = device.read_timeout(&mut buf, 10) {
                if len > 0 {
                    let data = buf[..len].to_vec();

                    // Only print if data changed
                    if data != last_data[i] {
                        println!("[Page:0x{:04X} Usage:0x{:04X}] {} bytes: {:02X?}",
                                 page, usage, len, data);

                        // Try to decode
                        if len >= 3 && buf[0] == 0x01 {
                            let button = (buf[1] & 0x01) != 0;
                            let rotation = buf[2] as i8;

                            if button {
                                println!("  >>> BUTTON PRESSED!");
                            }
                            if rotation != 0 {
                                if rotation > 0 {
                                    println!("  >>> ROTATE RIGHT (clockwise): {}", rotation);
                                } else {
                                    println!("  >>> ROTATE LEFT (counter-clockwise): {}", rotation);
                                }
                            }
                        }

                        last_data[i] = data;
                    }
                }
            }
        }
        std::thread::sleep(Duration::from_millis(5));
    }

    println!("\nDone.");
}
