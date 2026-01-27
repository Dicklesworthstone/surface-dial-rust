//! Try opening Surface Dial directly by VID/PID like mac-dial does

use hidapi::HidApi;

const VENDOR_ID: u16 = 0x045E;
const PRODUCT_ID: u16 = 0x091B;

fn main() {
    println!("Surface Dial Direct HID Open Test");
    println!("==================================\n");

    let api = HidApi::new().expect("Failed to init HID API");

    println!("Opening device directly by VID:PID (0x{:04X}:0x{:04X})...\n",
             VENDOR_ID, PRODUCT_ID);

    // Try to open directly like mac-dial does
    match api.open(VENDOR_ID, PRODUCT_ID) {
        Ok(device) => {
            println!("SUCCESS: Device opened!");
            println!("Manufacturer: {:?}", device.get_manufacturer_string());
            println!("Product: {:?}", device.get_product_string());

            println!("\nWaiting for data (blocking, 5 sec timeout)...");
            println!(">>> ROTATE THE DIAL OR PRESS BUTTON NOW <<<\n");

            let mut buf = [0u8; 64];

            for i in 0..10 {
                match device.read_timeout(&mut buf, 500) {
                    Ok(len) if len > 0 => {
                        println!("READ {}: {} bytes: {:02X?}", i, len, &buf[..len]);

                        // Parse like mac-dial does
                        if len >= 4 && buf[0] == 1 {
                            let button = (buf[1] & 0x01) != 0;
                            let rotation = buf[2] as i8;
                            println!("  -> Button: {}, Rotation: {}",
                                     if button { "PRESSED" } else { "released" },
                                     rotation);
                        }
                    }
                    Ok(_) => {
                        println!("READ {}: timeout", i);
                    }
                    Err(e) => {
                        println!("READ {}: error: {}", i, e);
                        break;
                    }
                }
            }
        }
        Err(e) => {
            println!("FAILED to open device: {}", e);
            println!("\nLet me try each interface separately...\n");

            for dev in api.device_list()
                .filter(|d| d.vendor_id() == VENDOR_ID && d.product_id() == PRODUCT_ID)
            {
                println!("Interface: Page=0x{:04X} Usage=0x{:04X} Path={:?}",
                         dev.usage_page(), dev.usage(), dev.path());

                match dev.open_device(&api) {
                    Ok(device) => {
                        println!("  Opened! Reading...");
                        let mut buf = [0u8; 64];
                        match device.read_timeout(&mut buf, 1000) {
                            Ok(len) if len > 0 => {
                                println!("  DATA: {} bytes: {:02X?}", len, &buf[..len]);
                            }
                            Ok(_) => println!("  No data (timeout)"),
                            Err(e) => println!("  Read error: {}", e),
                        }
                    }
                    Err(e) => println!("  Open failed: {}", e),
                }
            }
        }
    }

    println!("\nDone.");
}
