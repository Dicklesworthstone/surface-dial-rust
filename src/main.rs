//! Surface Dial Volume Controller for macOS/Linux/Windows
//!
//! A daemon that uses the Microsoft Surface Dial to control system volume.
//!
//! Controls:
//! - Rotate: Adjust volume (or mic in mic mode)
//! - Click: Toggle mute (or mic mute in mic mode)
//! - Double-click: Switch to mic control for 10 seconds
//! - Triple-click: Play/Pause media (if enabled)
//! - Hold 1 second: Send F15 key

use surface_dial::cli::{exit_codes, Cli, Command};

fn main() {
    let cli = Cli::parse_args();

    let exit_code = match cli.command {
        // No subcommand: run daemon (default behavior)
        None => {
            let daemon_cmd = surface_dial::cli::DaemonCmd::default();
            daemon_cmd.run(cli.json)
        }

        // Explicit daemon subcommand
        Some(Command::Daemon(cmd)) => cmd.run(cli.json),

        // Config subcommand
        Some(Command::Config(cmd)) => cmd.run(cli.json),

        // Status subcommand
        Some(Command::Status(cmd)) => cmd.run(cli.json),

        // Version subcommand
        Some(Command::Version) => {
            print_version(cli.json);
            exit_codes::SUCCESS
        }
    };

    std::process::exit(exit_code);
}

fn print_version(json: bool) {
    let version = env!("CARGO_PKG_VERSION");
    let name = env!("CARGO_PKG_NAME");

    if json {
        println!(
            "{}",
            serde_json::json!({
                "name": name,
                "version": version,
            })
        );
    } else {
        println!("{} {}", name, version);
    }
}
