//! Daemon subcommand implementation
//!
//! Handles running the Surface Dial daemon with various options.

use clap::Args;
use std::path::PathBuf;

/// Run the daemon
#[derive(Args, Debug, Clone, Default)]
pub struct DaemonCmd {
    /// Path to configuration file (default: ~/.config/surface-dial/config.toml)
    #[arg(short, long, value_name = "FILE")]
    pub config: Option<PathBuf>,

    /// Run in foreground (don't daemonize)
    #[arg(short, long)]
    pub foreground: bool,

    /// Override log level (error, warn, info, debug, trace)
    #[arg(long, value_name = "LEVEL")]
    pub log_level: Option<String>,

    /// Disable file logging
    #[arg(long)]
    pub no_log_file: bool,
}

impl DaemonCmd {
    /// Run the daemon with the given options
    pub fn run(&self, json_output: bool) -> i32 {
        use crate::cli::{exit_codes, Output};
        use crate::config::Config;
        use crate::daemon::Daemon;
        use log::info;
        use std::sync::atomic::Ordering;

        let output = Output::new(json_output);

        // Load configuration
        let mut config = if let Some(ref path) = self.config {
            match Config::load_from(path) {
                Ok(c) => c,
                Err(e) => {
                    output.error(&format!("Failed to load config from {:?}: {}", path, e));
                    return exit_codes::ERROR;
                }
            }
        } else {
            Config::load()
        };

        // Apply command-line overrides
        if let Some(ref level) = self.log_level {
            config.daemon.log_level = level.clone();
        }
        if self.no_log_file {
            config.daemon.log_file_enabled = false;
        }

        // Initialize logging
        env_logger::Builder::from_env(
            env_logger::Env::default().default_filter_or(&config.daemon.log_level),
        )
        .format_timestamp_secs()
        .init();

        info!("Surface Dial Volume Controller starting...");
        info!(
            "Controls: Rotate=volume, Click=mute, 2x-click=mic, Hold=F15"
        );

        if json_output {
            output.success("Daemon starting");
        }

        // Create and run the daemon
        let mut daemon = Daemon::new(config);

        // Set up graceful shutdown handler
        let running = daemon.running();
        if let Err(e) = ctrlc::set_handler(move || {
            info!("Shutdown signal received");
            running.store(false, Ordering::SeqCst);
        }) {
            output.error(&format!("Failed to set Ctrl+C handler: {}", e));
            return exit_codes::ERROR;
        }

        // Run the main loop
        daemon.run();

        info!("Surface Dial Volume Controller stopped.");
        exit_codes::SUCCESS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_daemon_cmd_default() {
        let cmd = DaemonCmd::default();
        assert!(cmd.config.is_none());
        assert!(!cmd.foreground);
        assert!(cmd.log_level.is_none());
        assert!(!cmd.no_log_file);
    }

    #[test]
    fn test_daemon_cmd_with_options() {
        let cmd = DaemonCmd {
            config: Some(PathBuf::from("/custom/config.toml")),
            foreground: true,
            log_level: Some("debug".to_string()),
            no_log_file: true,
        };

        assert_eq!(
            cmd.config,
            Some(PathBuf::from("/custom/config.toml"))
        );
        assert!(cmd.foreground);
        assert_eq!(cmd.log_level, Some("debug".to_string()));
        assert!(cmd.no_log_file);
    }
}
