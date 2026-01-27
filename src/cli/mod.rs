//! CLI module for Surface Dial
//!
//! Provides a clap-based command-line interface with subcommands for
//! daemon control, configuration management, and status checks.

mod config_cmd;
mod daemon_cmd;
mod status_cmd;

pub use config_cmd::ConfigCmd;
pub use daemon_cmd::DaemonCmd;
pub use status_cmd::StatusCmd;

use clap::{Parser, Subcommand};

/// Surface Dial Volume Controller
///
/// A daemon that uses the Microsoft Surface Dial to control system volume.
///
/// Controls:
///   Rotate         - Adjust volume (or mic in mic mode)
///   Click          - Toggle mute (or mic mute in mic mode)
///   Double-click   - Switch to mic control for 10 seconds
///   Triple-click   - Play/Pause media (if enabled)
///   Hold 1 second  - Send F15 key (hold until release)
#[derive(Parser, Debug)]
#[command(name = "surface-dial")]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    /// Enable verbose output (-v, -vv, -vvv for increasing levels)
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,

    /// Output in JSON format for machine parsing
    #[arg(long, global = true)]
    pub json: bool,

    /// Suppress all output except errors
    #[arg(short, long, global = true)]
    pub quiet: bool,

    #[command(subcommand)]
    pub command: Option<Command>,
}

/// Available subcommands
#[derive(Subcommand, Debug)]
pub enum Command {
    /// Run the daemon (this is the default if no subcommand is given)
    Daemon(DaemonCmd),

    /// Manage configuration settings
    #[command(subcommand)]
    Config(ConfigCmd),

    /// Check daemon and device status
    Status(StatusCmd),

    /// Show version information
    Version,
}

impl Cli {
    /// Parse CLI arguments
    pub fn parse_args() -> Self {
        Self::parse()
    }

    /// Get the log level based on verbosity flags
    pub fn log_level(&self) -> &'static str {
        if self.quiet {
            "error"
        } else {
            match self.verbose {
                0 => "info",
                1 => "debug",
                _ => "trace",
            }
        }
    }
}

/// Result type for CLI operations
pub type CliResult<T> = Result<T, CliError>;

/// CLI error type
#[derive(Debug, thiserror::Error)]
pub enum CliError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    #[error("Device not found: {0}")]
    DeviceNotFound(String),

    #[error("{0}")]
    Other(String),
}

/// Exit codes for CLI operations
pub mod exit_codes {
    /// Success
    pub const SUCCESS: i32 = 0;
    /// General error
    pub const ERROR: i32 = 1;
    /// Invalid arguments
    pub const INVALID_ARGS: i32 = 2;
    /// Device not found
    pub const DEVICE_NOT_FOUND: i32 = 3;
}

/// Output helper for consistent formatting
pub struct Output {
    /// Whether to output in JSON format
    pub json: bool,
}

impl Output {
    pub fn new(json: bool) -> Self {
        Self { json }
    }

    /// Print a success message
    pub fn success(&self, message: &str) {
        if self.json {
            println!(
                "{}",
                serde_json::json!({
                    "status": "success",
                    "message": message
                })
            );
        } else {
            println!("{}", message);
        }
    }

    /// Print an error message to stderr
    pub fn error(&self, message: &str) {
        if self.json {
            eprintln!(
                "{}",
                serde_json::json!({
                    "status": "error",
                    "message": message
                })
            );
        } else {
            eprintln!("Error: {}", message);
        }
    }

    /// Print data as JSON or formatted text
    pub fn data<T: serde::Serialize + std::fmt::Display>(&self, data: &T) {
        if self.json {
            if let Ok(json) = serde_json::to_string_pretty(data) {
                println!("{}", json);
            }
        } else {
            println!("{}", data);
        }
    }

    /// Print raw JSON value
    pub fn json_value(&self, value: &serde_json::Value) {
        if self.json {
            if let Ok(json) = serde_json::to_string_pretty(value) {
                println!("{}", json);
            }
        } else {
            // For non-JSON mode, print a simplified view
            print_json_as_text(value, 0);
        }
    }
}

/// Print JSON value as readable text with indentation
fn print_json_as_text(value: &serde_json::Value, indent: usize) {
    let prefix = "  ".repeat(indent);
    match value {
        serde_json::Value::Object(map) => {
            for (k, v) in map {
                match v {
                    serde_json::Value::Object(_) => {
                        println!("{}{}:", prefix, k);
                        print_json_as_text(v, indent + 1);
                    }
                    serde_json::Value::Array(arr) => {
                        println!("{}{}:", prefix, k);
                        for item in arr {
                            print_json_as_text(item, indent + 1);
                        }
                    }
                    _ => {
                        println!("{}{}: {}", prefix, k, format_json_value(v));
                    }
                }
            }
        }
        serde_json::Value::Array(arr) => {
            for item in arr {
                println!("{}- {}", prefix, format_json_value(item));
            }
        }
        _ => {
            println!("{}{}", prefix, format_json_value(value));
        }
    }
}

/// Format a simple JSON value as a string
fn format_json_value(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "null".to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => s.clone(),
        _ => value.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parse_no_args() {
        let cli = Cli::try_parse_from(["surface-dial"]).unwrap();
        assert!(cli.command.is_none());
        assert!(!cli.json);
        assert!(!cli.quiet);
        assert_eq!(cli.verbose, 0);
    }

    #[test]
    fn test_cli_parse_daemon() {
        let cli = Cli::try_parse_from(["surface-dial", "daemon"]).unwrap();
        assert!(matches!(cli.command, Some(Command::Daemon(_))));
    }

    #[test]
    fn test_cli_parse_status() {
        let cli = Cli::try_parse_from(["surface-dial", "status"]).unwrap();
        assert!(matches!(cli.command, Some(Command::Status(_))));
    }

    #[test]
    fn test_cli_parse_config_show() {
        let cli = Cli::try_parse_from(["surface-dial", "config", "show"]).unwrap();
        assert!(matches!(cli.command, Some(Command::Config(ConfigCmd::Show))));
    }

    #[test]
    fn test_cli_parse_version() {
        let cli = Cli::try_parse_from(["surface-dial", "version"]).unwrap();
        assert!(matches!(cli.command, Some(Command::Version)));
    }

    #[test]
    fn test_cli_verbose_levels() {
        let cli = Cli::try_parse_from(["surface-dial", "-v"]).unwrap();
        assert_eq!(cli.verbose, 1);
        assert_eq!(cli.log_level(), "debug");

        let cli = Cli::try_parse_from(["surface-dial", "-vv"]).unwrap();
        assert_eq!(cli.verbose, 2);
        assert_eq!(cli.log_level(), "trace");
    }

    #[test]
    fn test_cli_quiet_mode() {
        let cli = Cli::try_parse_from(["surface-dial", "-q"]).unwrap();
        assert!(cli.quiet);
        assert_eq!(cli.log_level(), "error");
    }

    #[test]
    fn test_cli_json_flag() {
        let cli = Cli::try_parse_from(["surface-dial", "--json"]).unwrap();
        assert!(cli.json);
    }

    #[test]
    fn test_output_helper() {
        let output = Output::new(false);
        // Just ensure it doesn't panic
        output.success("test");
    }

    #[test]
    fn test_format_json_value() {
        assert_eq!(format_json_value(&serde_json::json!(null)), "null");
        assert_eq!(format_json_value(&serde_json::json!(true)), "true");
        assert_eq!(format_json_value(&serde_json::json!(42)), "42");
        assert_eq!(format_json_value(&serde_json::json!("hello")), "hello");
    }
}
