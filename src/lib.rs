//! Surface Dial Volume Controller library
//!
//! This crate provides the core functionality for the Surface Dial volume controller daemon.
//!
//! ## Modules
//!
//! - [`cli`] - Command-line interface with clap-based subcommands
//! - [`config`] - Configuration file management with 50+ configurable keys
//! - [`platform`] - Cross-platform abstraction for volume control, key simulation, etc.
//! - [`logging`] - Structured logging with file rotation and color support
//! - [`input`] - Click detection and rotation sensitivity processing
//! - [`daemon`] - Main daemon loop implementation
//! - [`hid`] - HID device abstraction for testing
//! - [`pidfile`] - PID file management for daemon lifecycle

pub mod cli;
pub mod config;
pub mod daemon;
pub mod hid;
pub mod input;
pub mod logging;
pub mod pidfile;
pub mod platform;
