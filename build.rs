fn main() {
    #[cfg(target_os = "macos")]
    {
        // Embed Info.plist into the binary
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let info_plist_path = format!("{}/Info.plist", manifest_dir);

        println!("cargo:rustc-link-arg=-sectcreate");
        println!("cargo:rustc-link-arg=__TEXT");
        println!("cargo:rustc-link-arg=__info_plist");
        println!("cargo:rustc-link-arg={}", info_plist_path);

        // Rerun if Info.plist changes
        println!("cargo:rerun-if-changed=Info.plist");
    }
}
