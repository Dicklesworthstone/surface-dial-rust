//! Config subcommand implementation
//!
//! Handles configuration management: show, get, set, reset, path.

use crate::cli::{exit_codes, Output};
use crate::config::Config;
use clap::{Args, Subcommand};

/// Configuration management commands
#[derive(Subcommand, Debug, Clone)]
pub enum ConfigCmd {
    /// Show all configuration settings
    Show,

    /// Get a specific configuration value
    Get(GetArgs),

    /// Set a configuration value
    Set(SetArgs),

    /// Reset configuration to defaults
    Reset(ResetArgs),

    /// Show configuration file path
    Path,
}

/// Arguments for config get
#[derive(Args, Debug, Clone)]
pub struct GetArgs {
    /// Configuration key (e.g., volume.step_min, daemon.log_level)
    pub key: String,
}

/// Arguments for config set
#[derive(Args, Debug, Clone)]
pub struct SetArgs {
    /// Configuration key (e.g., volume.step_min, daemon.log_level)
    pub key: String,

    /// Value to set
    pub value: String,

    /// Don't save to file, just show what would change
    #[arg(long)]
    pub dry_run: bool,
}

/// Arguments for config reset
#[derive(Args, Debug, Clone)]
pub struct ResetArgs {
    /// Reset specific section only (volume, microphone, daemon, etc.)
    #[arg(short, long)]
    pub section: Option<String>,

    /// Force reset without confirmation
    #[arg(short, long)]
    pub force: bool,
}

impl ConfigCmd {
    /// Execute the config subcommand
    pub fn run(&self, json_output: bool) -> i32 {
        let output = Output::new(json_output);

        match self {
            ConfigCmd::Show => show_config(&output),
            ConfigCmd::Get(args) => get_config(&args.key, &output),
            ConfigCmd::Set(args) => set_config(&args.key, &args.value, args.dry_run, &output),
            ConfigCmd::Reset(args) => reset_config(args.section.as_deref(), args.force, &output),
            ConfigCmd::Path => show_path(&output),
        }
    }
}

/// Show all configuration settings
fn show_config(output: &Output) -> i32 {
    let config = Config::load();

    if let Ok(value) = serde_json::to_value(&config) {
        output.json_value(&value);
        exit_codes::SUCCESS
    } else {
        output.error("Failed to serialize configuration");
        exit_codes::ERROR
    }
}

/// Get a specific configuration value
fn get_config(key: &str, output: &Output) -> i32 {
    let config = Config::load();

    match config.get_value(key) {
        Ok(value) => {
            output.json_value(&value);
            exit_codes::SUCCESS
        }
        Err(e) => {
            output.error(&format!("Failed to get '{}': {}", key, e));
            exit_codes::INVALID_ARGS
        }
    }
}

/// Set a configuration value
fn set_config(key: &str, value: &str, dry_run: bool, output: &Output) -> i32 {
    let mut config = Config::load();

    // Try to set the value
    match config.set_value(key, value) {
        Ok(()) => {
            if dry_run {
                output.success(&format!("Would set {} = {}", key, value));
            } else {
                // Save the configuration
                if let Err(e) = config.save() {
                    output.error(&format!("Failed to save configuration: {}", e));
                    return exit_codes::ERROR;
                }
                output.success(&format!("Set {} = {}", key, value));
            }
            exit_codes::SUCCESS
        }
        Err(e) => {
            output.error(&format!("Failed to set '{}': {}", key, e));
            exit_codes::INVALID_ARGS
        }
    }
}

/// Reset configuration to defaults
fn reset_config(section: Option<&str>, force: bool, output: &Output) -> i32 {
    if !force {
        output.error("Reset requires --force flag to confirm");
        return exit_codes::INVALID_ARGS;
    }

    let mut config = Config::load();

    match section {
        Some(s) => {
            // Reset specific section
            match config.reset_section(s) {
                Ok(()) => {
                    if let Err(e) = config.save() {
                        output.error(&format!("Failed to save configuration: {}", e));
                        return exit_codes::ERROR;
                    }
                    output.success(&format!("Reset section '{}' to defaults", s));
                }
                Err(e) => {
                    output.error(&format!("Failed to reset section '{}': {}", s, e));
                    return exit_codes::INVALID_ARGS;
                }
            }
        }
        None => {
            // Reset entire config
            config = Config::default();
            if let Err(e) = config.save() {
                output.error(&format!("Failed to save configuration: {}", e));
                return exit_codes::ERROR;
            }
            output.success("Reset all configuration to defaults");
        }
    }

    exit_codes::SUCCESS
}

/// Show configuration file path
fn show_path(output: &Output) -> i32 {
    let path = Config::config_path();
    let exists = path.exists();

    if output.json {
        println!(
            "{}",
            serde_json::json!({
                "path": path.display().to_string(),
                "exists": exists
            })
        );
    } else {
        println!("{}", path.display());
        if !exists {
            println!("(file does not exist, will use defaults)");
        }
    }

    exit_codes::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_cmd_variants() {
        // Just ensure the enum variants exist and can be matched
        let cmds = vec![
            ConfigCmd::Show,
            ConfigCmd::Get(GetArgs {
                key: "test".to_string(),
            }),
            ConfigCmd::Set(SetArgs {
                key: "test".to_string(),
                value: "value".to_string(),
                dry_run: false,
            }),
            ConfigCmd::Reset(ResetArgs {
                section: None,
                force: false,
            }),
            ConfigCmd::Path,
        ];

        assert_eq!(cmds.len(), 5);
    }

    #[test]
    fn test_get_args() {
        let args = GetArgs {
            key: "volume.step_min".to_string(),
        };
        assert_eq!(args.key, "volume.step_min");
    }

    #[test]
    fn test_set_args() {
        let args = SetArgs {
            key: "volume.step_max".to_string(),
            value: "10".to_string(),
            dry_run: true,
        };
        assert_eq!(args.key, "volume.step_max");
        assert_eq!(args.value, "10");
        assert!(args.dry_run);
    }

    #[test]
    fn test_reset_args() {
        let args = ResetArgs {
            section: Some("volume".to_string()),
            force: true,
        };
        assert_eq!(args.section, Some("volume".to_string()));
        assert!(args.force);
    }
}
