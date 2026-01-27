//! Structured logging infrastructure with file output and rotation
//!
//! This module provides a dual-output logger that writes to both console (with colors)
//! and a rotating log file. It integrates with the config system for log level and
//! file settings.
//!
//! ## Features
//!
//! - Colored console output by log level
//! - File logging with automatic rotation
//! - Configurable max file size and number of rotated files to keep
//! - Optional JSON output for machine parsing
//! - Structured event logging for analytics

use crate::config::DaemonConfig;
use chrono::Local;
use log::{Level, LevelFilter, Log, Metadata, Record};
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::sync::Mutex;

/// A logger that outputs to both console and a rotating log file
pub struct DualLogger {
    /// Log level filter for console output
    console_level: LevelFilter,
    /// Log level filter for file output
    file_level: LevelFilter,
    /// Rotating file writer (wrapped in Mutex for thread safety)
    file: Option<Mutex<RotatingFile>>,
    /// Whether to use JSON format for file output
    json_mode: bool,
    /// Whether console output is a TTY (for color support)
    is_tty: bool,
}

/// A file writer with automatic rotation based on size
pub struct RotatingFile {
    /// Buffered file writer
    writer: BufWriter<File>,
    /// Path to the current log file
    path: PathBuf,
    /// Current size of the log file in bytes
    current_size: u64,
    /// Maximum size before rotation (in bytes)
    max_size: u64,
    /// Number of rotated files to keep
    keep_files: u32,
}

impl RotatingFile {
    /// Create a new rotating file writer
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the log file
    /// * `max_size_mb` - Maximum file size in megabytes before rotation
    /// * `keep_files` - Number of rotated log files to keep
    pub fn new(path: PathBuf, max_size_mb: u32, keep_files: u32) -> std::io::Result<Self> {
        let max_size = (max_size_mb as u64) * 1024 * 1024;

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)?;

        let current_size = file.metadata()?.len();

        Ok(Self {
            writer: BufWriter::new(file),
            path,
            current_size,
            max_size,
            keep_files,
        })
    }

    /// Write data to the file, rotating if necessary
    pub fn write(&mut self, data: &[u8]) -> std::io::Result<()> {
        // Check if rotation needed
        if self.current_size + data.len() as u64 > self.max_size {
            self.rotate()?;
        }

        self.writer.write_all(data)?;
        self.writer.flush()?;
        self.current_size += data.len() as u64;
        Ok(())
    }

    /// Rotate the log file
    fn rotate(&mut self) -> std::io::Result<()> {
        // Close current file
        self.writer.flush()?;

        // Rotate existing files (oldest gets deleted)
        for i in (1..self.keep_files).rev() {
            let old_path = self.rotated_path(i);
            let new_path = self.rotated_path(i + 1);
            if old_path.exists() {
                if i + 1 > self.keep_files {
                    std::fs::remove_file(&old_path)?;
                } else {
                    std::fs::rename(&old_path, &new_path)?;
                }
            }
        }

        // Delete the oldest file if it exists and exceeds keep_files
        let oldest_path = self.rotated_path(self.keep_files);
        if oldest_path.exists() {
            std::fs::remove_file(&oldest_path)?;
        }

        // Rename current to .1
        if self.path.exists() {
            std::fs::rename(&self.path, self.rotated_path(1))?;
        }

        // Create new file
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&self.path)?;

        self.writer = BufWriter::new(file);
        self.current_size = 0;

        Ok(())
    }

    /// Get the path for a rotated file (e.g., daemon.log.1, daemon.log.2)
    fn rotated_path(&self, n: u32) -> PathBuf {
        let mut path = self.path.clone();
        let filename = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        path.set_file_name(format!("{}.{}", filename, n));
        path
    }
}

impl DualLogger {
    /// Initialize the logging system with the given configuration
    ///
    /// This should be called once at application startup.
    pub fn init(config: &DaemonConfig) -> Result<(), log::SetLoggerError> {
        let level = Self::parse_level(&config.log_level);

        let file = if config.log_file_enabled {
            let log_path = Self::default_log_path();

            RotatingFile::new(
                log_path,
                config.log_max_size_mb as u32,
                config.log_keep_files as u32,
            )
            .ok()
            .map(Mutex::new)
        } else {
            None
        };

        // Check if stdout is a TTY for color support
        let is_tty = atty_check();

        let logger = Self {
            console_level: level,
            file_level: level,
            file,
            json_mode: config.log_json,
            is_tty,
        };

        log::set_boxed_logger(Box::new(logger))?;
        log::set_max_level(level);
        Ok(())
    }

    /// Initialize with default settings (for testing or simple use)
    pub fn init_default() -> Result<(), log::SetLoggerError> {
        let config = DaemonConfig::default();
        Self::init(&config)
    }

    /// Get the default log file path
    pub fn default_log_path() -> PathBuf {
        dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("surface-dial")
            .join("daemon.log")
    }

    /// Parse a log level string into a LevelFilter
    fn parse_level(level: &str) -> LevelFilter {
        match level.to_lowercase().as_str() {
            "error" => LevelFilter::Error,
            "warn" => LevelFilter::Warn,
            "info" => LevelFilter::Info,
            "debug" => LevelFilter::Debug,
            "trace" => LevelFilter::Trace,
            _ => LevelFilter::Info,
        }
    }

    /// Format a log record for console output (with colors if TTY)
    fn format_console(&self, record: &Record) -> String {
        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let level = record.level();
        let target = record.target();
        let message = record.args();

        if self.is_tty {
            let (color, reset) = match level {
                Level::Error => ("\x1b[31m", "\x1b[0m"), // Red
                Level::Warn => ("\x1b[33m", "\x1b[0m"),  // Yellow
                Level::Info => ("\x1b[32m", "\x1b[0m"),  // Green
                Level::Debug => ("\x1b[36m", "\x1b[0m"), // Cyan
                Level::Trace => ("\x1b[90m", "\x1b[0m"), // Gray
            };
            format!(
                "{color}[{timestamp}] [{level:5}] [{target}] {message}{reset}\n"
            )
        } else {
            format!("[{timestamp}] [{level:5}] [{target}] {message}\n")
        }
    }

    /// Format a log record for file output
    fn format_file(&self, record: &Record) -> String {
        let timestamp = Local::now().format("%Y-%m-%dT%H:%M:%S%.3f%z");
        let level = record.level();
        let target = record.target();
        let message = record.args().to_string();

        if self.json_mode {
            // Structured JSON output
            let json = serde_json::json!({
                "timestamp": timestamp.to_string(),
                "level": level.to_string(),
                "target": target,
                "message": message
            });
            format!("{}\n", json)
        } else {
            format!("[{timestamp}] [{level:5}] [{target}] {message}\n")
        }
    }
}

impl Log for DualLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= self.console_level || metadata.level() <= self.file_level
    }

    fn log(&self, record: &Record) {
        // Console output
        if record.level() <= self.console_level {
            let formatted = self.format_console(record);
            // Use stderr for log output (standard practice)
            eprint!("{}", formatted);
        }

        // File output
        if record.level() <= self.file_level {
            if let Some(ref file) = self.file {
                let formatted = self.format_file(record);
                if let Ok(mut file) = file.lock() {
                    let _ = file.write(formatted.as_bytes());
                }
            }
        }
    }

    fn flush(&self) {
        if let Some(ref file) = self.file {
            if let Ok(mut file) = file.lock() {
                let _ = file.writer.flush();
            }
        }
    }
}

/// Check if stderr is a TTY (for color support)
fn atty_check() -> bool {
    // Check common environment variables that indicate terminal support
    // This is a simpler approach that doesn't require libc
    if std::env::var("NO_COLOR").is_ok() {
        return false;
    }

    // Check for common terminal indicators
    std::env::var("TERM").map(|t| !t.is_empty() && t != "dumb").unwrap_or(false)
        || std::env::var("COLORTERM").is_ok()
        || std::env::var("WT_SESSION").is_ok() // Windows Terminal
        || std::env::var("ANSICON").is_ok() // Windows ANSI
        || std::env::var("CLICOLOR").is_ok()
        || std::env::var("FORCE_COLOR").is_ok()
}

/// Structured event for analytics and debugging
#[derive(Debug, Clone, serde::Serialize)]
pub struct StructuredEvent {
    /// ISO-8601 timestamp
    pub timestamp: String,
    /// Event type (e.g., "volume_change", "mute_toggle")
    pub event_type: String,
    /// Component that generated the event (e.g., "daemon", "config")
    pub component: String,
    /// Event-specific data
    pub data: serde_json::Value,
}

impl StructuredEvent {
    /// Create a new structured event
    pub fn new<T: serde::Serialize>(event_type: &str, component: &str, data: T) -> Self {
        Self {
            timestamp: chrono::Utc::now().to_rfc3339(),
            event_type: event_type.to_string(),
            component: component.to_string(),
            data: serde_json::to_value(data).unwrap_or(serde_json::Value::Null),
        }
    }

    /// Convert to JSON string
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

/// Convenience macro for creating structured events
#[macro_export]
macro_rules! log_event {
    ($event_type:expr, $component:expr, $($field:tt)*) => {{
        let event = $crate::logging::StructuredEvent::new(
            $event_type,
            $component,
            serde_json::json!($($field)*)
        );
        log::info!(target: "events", "{}", event.to_json());
    }};
}

// Re-export for convenience
pub use log::{debug, error, info, trace, warn};

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_rotating_file_creation() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("test.log");

        let rf = RotatingFile::new(path.clone(), 1, 3);
        assert!(rf.is_ok());
        assert!(path.exists());
    }

    #[test]
    fn test_rotating_file_write() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("test.log");

        let mut rf = RotatingFile::new(path.clone(), 1, 3).unwrap();
        rf.write(b"Hello, world!\n").unwrap();

        let contents = std::fs::read_to_string(&path).unwrap();
        assert_eq!(contents, "Hello, world!\n");
    }

    #[test]
    fn test_rotating_file_rotation() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("test.log");

        // Create with very small max size (100 bytes)
        let mut rf = RotatingFile::new(path.clone(), 0, 3).unwrap();
        rf.max_size = 100; // Override for test

        // Write enough to trigger rotation
        let data = "x".repeat(60);
        rf.write(data.as_bytes()).unwrap();
        rf.write(data.as_bytes()).unwrap(); // Should trigger rotation

        // Check rotated file exists
        assert!(temp.path().join("test.log.1").exists());
    }

    #[test]
    fn test_rotated_path() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("daemon.log");
        let rf = RotatingFile::new(path.clone(), 1, 3).unwrap();

        assert_eq!(rf.rotated_path(1), temp.path().join("daemon.log.1"));
        assert_eq!(rf.rotated_path(2), temp.path().join("daemon.log.2"));
    }

    #[test]
    fn test_parse_level() {
        assert_eq!(DualLogger::parse_level("error"), LevelFilter::Error);
        assert_eq!(DualLogger::parse_level("WARN"), LevelFilter::Warn);
        assert_eq!(DualLogger::parse_level("Info"), LevelFilter::Info);
        assert_eq!(DualLogger::parse_level("debug"), LevelFilter::Debug);
        assert_eq!(DualLogger::parse_level("trace"), LevelFilter::Trace);
        assert_eq!(DualLogger::parse_level("invalid"), LevelFilter::Info);
    }

    #[test]
    fn test_default_log_path() {
        let path = DualLogger::default_log_path();
        assert!(path.to_string_lossy().contains("surface-dial"));
        assert!(path.to_string_lossy().contains("daemon.log"));
    }

    #[test]
    fn test_structured_event_creation() {
        let event = StructuredEvent::new(
            "volume_change",
            "daemon",
            serde_json::json!({"old": 50, "new": 55}),
        );

        assert_eq!(event.event_type, "volume_change");
        assert_eq!(event.component, "daemon");
        assert!(!event.timestamp.is_empty());
    }

    #[test]
    fn test_structured_event_to_json() {
        let event = StructuredEvent::new("test", "test", serde_json::json!({"key": "value"}));

        let json = event.to_json();
        assert!(json.contains("test"));
        assert!(json.contains("key"));
        assert!(json.contains("value"));
    }

    #[test]
    fn test_atty_check_does_not_panic() {
        // Just ensure it doesn't panic
        let _ = atty_check();
    }
}
