//! Poll for Surface Dial and read when it wakes up

use hidapi::HidApi;
use std::time::{Duration, Instant};
use std::thread;

const VENDOR_ID: u16 = 0x045E;
const PRODUCT_ID: u16 = 0x091B;

fn main() {
    println!("Surface Dial Polling Test");
    println!("=========================\n");
    println!("The dial sleeps when idle. Wake it by rotating or pressing!\n");

    let timeout = Duration::from_secs(30);
    let start = Instant::now();

    loop {
        if start.elapsed() > timeout {
            println!("\nTimeout after 30 seconds.");
            break;
        }

        // Refresh HID API to see newly connected devices
        let Ok(api) = HidApi::new() else {
            thread::sleep(Duration::from_millis(500));
            continue;
        };

        let devices: Vec<_> = api.device_list()
            .filter(|d| d.vendor_id() == VENDOR_ID && d.product_id() == PRODUCT_ID)
            .collect();

        if devices.is_empty() {
            print!(".");
            std::io::Write::flush(&mut std::io::stdout()).ok();
            thread::sleep(Duration::from_millis(500));
            continue;
        }

        println!("\n\nDial woke up! Found {} interface(s)", devices.len());

        // Try to open and read from each interface
        for dev in &devices {
            let page = dev.usage_page();
            let usage = dev.usage();
            println!("\nInterface: Page=0x{:04X} Usage=0x{:04X}", page, usage);

            match dev.open_device(&api) {
                Ok(device) => {
                    println!("  Opened! Attempting reads...");

                    // Try several reads
                    for i in 0..20 {
                        let mut buf = [0u8; 64];
                        match device.read_timeout(&mut buf, 250) {
                            Ok(len) if len > 0 => {
                                println!("  READ[{}]: {} bytes: {:02X?}", i, len, &buf[..len]);

                                // Parse Surface Dial report format
                                if len >= 3 {
                                    if buf[0] == 1 {
                                        // Standard dial report
                                        let button = (buf[1] & 0x01) != 0;
                                        let rotation = buf[2] as i8;
                                        println!("    -> Button:{} Rotation:{}", button, rotation);
                                    } else {
                                        println!("    -> Report ID: {}", buf[0]);
                                    }
                                }
                            }
                            Ok(_) => {
                                // Timeout - no data
                            }
                            Err(e) => {
                                println!("  READ[{}]: error: {}", i, e);
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    println!("  Failed to open: {}", e);
                }
            }
        }

        println!("\nSleeping 2 seconds then checking again...");
        thread::sleep(Duration::from_secs(2));
    }

    println!("\nDone.");
}
