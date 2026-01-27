//! Test blocking HID reads from Surface Dial
//!
//! The dial might need blocking reads instead of polling

use hidapi::HidApi;
use std::time::Duration;

const VENDOR_ID: u16 = 0x045E;
const PRODUCT_ID: u16 = 0x091B;

fn main() {
    println!("Surface Dial Blocking HID Read Test");
    println!("====================================\n");

    let api = HidApi::new().expect("Failed to init HID API");

    // Find all Surface Dial interfaces
    let interfaces: Vec<_> = api.device_list()
        .filter(|d| d.vendor_id() == VENDOR_ID && d.product_id() == PRODUCT_ID)
        .collect();

    println!("Found {} Surface Dial interface(s)\n", interfaces.len());

    for (i, device_info) in interfaces.iter().enumerate() {
        println!("Interface {}: Usage Page 0x{:04X}, Usage 0x{:04X}",
            i, device_info.usage_page(), device_info.usage());
    }

    // Try the Consumer Control interface (Usage Page 0x0001, Usage 0x000E - System Control)
    // Or try Usage Page 0x000D (Digitizer) for haptic feedback
    // Or try Vendor-specific (0xFF07)

    println!("\n=== Attempting to open each interface with BLOCKING reads ===");
    println!("Rotate the dial or press the button!\n");

    for (i, device_info) in interfaces.iter().enumerate() {
        let usage_page = device_info.usage_page();
        let usage = device_info.usage();

        println!("--- Interface {} (Page:0x{:04X} Usage:0x{:04X}) ---", i, usage_page, usage);

        match device_info.open_device(&api) {
            Ok(device) => {
                // Set to BLOCKING mode with timeout
                let _ = device.set_blocking_mode(true);

                println!("  Opened. Waiting 3 seconds for data (BLOCKING)...");

                let mut buf = [0u8; 64];
                match device.read_timeout(&mut buf, 3000) {
                    Ok(len) if len > 0 => {
                        println!("  GOT DATA! {} bytes: {:02X?}", len, &buf[..len]);
                    }
                    Ok(_) => {
                        println!("  Timeout (no data in 3 sec)");
                    }
                    Err(e) => {
                        println!("  Read error: {}", e);
                    }
                }
            }
            Err(e) => {
                println!("  Failed to open: {}", e);
            }
        }
        println!();
    }

    println!("Done.");
}
