use hidapi::HidApi;

fn main() {
    let api = HidApi::new().unwrap();

    println!("=== Microsoft devices and anything with 'dial' ===\n");
    for device in api.device_list() {
        let product = device.product_string().unwrap_or("Unknown");
        let manufacturer = device.manufacturer_string().unwrap_or("Unknown");

        let is_microsoft = device.vendor_id() == 0x045E;
        let has_dial = product.to_lowercase().contains("dial");

        if is_microsoft || has_dial {
            println!("VID:{:04X} PID:{:04X}", device.vendor_id(), device.product_id());
            println!("  Manufacturer: {}", manufacturer);
            println!("  Product: {}", product);
            println!("  Path: {:?}", device.path());
            println!();
        }
    }

    println!("\n=== All HID devices ===\n");
    for device in api.device_list() {
        let product = device.product_string().unwrap_or("Unknown");
        let vid = device.vendor_id();
        let pid = device.product_id();
        println!("VID:{:04X} PID:{:04X} - {}", vid, pid, product);
    }
}
