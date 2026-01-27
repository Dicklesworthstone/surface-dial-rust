use hidapi::HidApi;
use std::time::Duration;
use std::thread;

const VENDOR_ID: u16 = 0x045E;
const PRODUCT_ID: u16 = 0x091B;

fn main() {
    let api = HidApi::new().unwrap();

    println!("Surface Dial interfaces:\n");

    for (i, device_info) in api.device_list()
        .filter(|d| d.vendor_id() == VENDOR_ID && d.product_id() == PRODUCT_ID)
        .enumerate()
    {
        println!("Interface {}: {:?}", i, device_info.path());
        println!("  Usage Page: 0x{:04X}", device_info.usage_page());
        println!("  Usage: 0x{:04X}", device_info.usage());
        println!("  Interface: {:?}", device_info.interface_number());

        // Try to open and read from each interface
        match device_info.open_device(&api) {
            Ok(device) => {
                let _ = device.set_blocking_mode(false);
                let mut buf = [0u8; 64];

                println!("  Status: Opened successfully");
                println!("  Manufacturer: {:?}", device.get_manufacturer_string());
                println!("  Product: {:?}", device.get_product_string());

                // Try a quick read
                match device.read_timeout(&mut buf, 100) {
                    Ok(len) if len > 0 => println!("  Data: {} bytes: {:02X?}", len, &buf[..len]),
                    Ok(_) => println!("  Data: No data available"),
                    Err(e) => println!("  Read error: {}", e),
                }
            }
            Err(e) => {
                println!("  Status: Failed to open: {}", e);
            }
        }
        println!();
    }

    println!("\n=== Listening on all interfaces for 10 seconds ===");
    println!("Rotate the dial or press the button!\n");

    // Open all interfaces and listen
    let mut devices = Vec::new();
    for device_info in api.device_list()
        .filter(|d| d.vendor_id() == VENDOR_ID && d.product_id() == PRODUCT_ID)
    {
        if let Ok(device) = device_info.open_device(&api) {
            let _ = device.set_blocking_mode(false);
            let path = device_info.path().to_string_lossy().to_string();
            let usage = (device_info.usage_page(), device_info.usage());
            devices.push((device, path, usage));
        }
    }

    let start = std::time::Instant::now();
    while start.elapsed() < Duration::from_secs(10) {
        for (device, path, usage) in &devices {
            let mut buf = [0u8; 64];
            if let Ok(len) = device.read_timeout(&mut buf, 10) {
                if len > 0 {
                    println!("[{:?}] UsagePage:0x{:04X} Usage:0x{:04X}",
                        path, usage.0, usage.1);
                    println!("  {} bytes: {:02X?}", len, &buf[..len]);
                }
            }
        }
        thread::sleep(Duration::from_millis(10));
    }

    println!("\nDone.");
}
