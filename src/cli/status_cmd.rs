//! Status subcommand implementation
//!
//! Handles checking daemon and device status with comprehensive output.

use crate::cli::{exit_codes, Output};
use crate::config::Config;
use crate::daemon::{SURFACE_DIAL_PRODUCT_ID, SURFACE_DIAL_VENDOR_ID};
use clap::Args;
use hidapi::HidApi;
use serde::Serialize;

/// Check daemon and device status
#[derive(Args, Debug, Clone, Default)]
pub struct StatusCmd {
    /// Show detailed device information
    #[arg(short, long)]
    pub detailed: bool,

    /// Only check if device is connected (exit code only)
    #[arg(short, long)]
    pub check: bool,

    /// Continuous status updates (refreshes every second)
    #[arg(short, long)]
    pub watch: bool,
}

/// Complete status information
#[derive(Debug, Serialize)]
pub struct FullStatus {
    pub daemon: DaemonStatus,
    pub device: DeviceStatus,
    pub state: CurrentState,
    pub features: FeatureStatus,
    pub sensitivity: SensitivityStatus,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub audio_devices: Vec<AudioDeviceInfo>,
}

/// Daemon status information
#[derive(Debug, Serialize)]
pub struct DaemonStatus {
    pub running: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pid: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uptime_seconds: Option<u64>,
    pub log_level: String,
}

/// Device status information
#[derive(Debug, Serialize)]
pub struct DeviceStatus {
    pub connected: bool,
    pub device_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vendor_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub product_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub product_name: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub interfaces: Vec<DeviceInterface>,
}

/// Device interface details
#[derive(Debug, Serialize)]
pub struct DeviceInterface {
    pub path: String,
    pub interface_number: i32,
}

/// Current control state
#[derive(Debug, Serialize)]
pub struct CurrentState {
    pub mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume_percent: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub muted: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mic_percent: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mic_muted: Option<bool>,
}

/// Feature status with configuration
#[derive(Debug, Serialize)]
pub struct FeatureStatus {
    pub osd: FeatureInfo,
    pub battery_monitor: FeatureInfo,
    pub system_tray: FeatureInfo,
    pub media_control: FeatureInfo,
    pub audio_feedback: FeatureInfo,
    pub event_hooks: FeatureInfo,
    pub device_switching: FeatureInfo,
}

/// Individual feature information
#[derive(Debug, Serialize)]
pub struct FeatureInfo {
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// Sensitivity settings
#[derive(Debug, Serialize)]
pub struct SensitivityStatus {
    pub dead_zone: i32,
    pub multiplier: f64,
    pub invert: bool,
    pub preset: String,
}

/// Audio device information
#[derive(Debug, Serialize)]
pub struct AudioDeviceInfo {
    pub name: String,
    pub device_type: String,
    pub is_default: bool,
}

impl StatusCmd {
    /// Execute the status subcommand
    pub fn run(&self, json_output: bool) -> i32 {
        let _output = Output::new(json_output);

        if self.watch {
            self.run_watch_mode(json_output)
        } else {
            self.run_single(json_output)
        }
    }

    fn run_single(&self, json_output: bool) -> i32 {
        let status = gather_full_status(self.detailed);

        if self.check {
            // Silent check mode - just return exit code
            return if status.device.connected {
                exit_codes::SUCCESS
            } else {
                exit_codes::DEVICE_NOT_FOUND
            };
        }

        if json_output {
            if let Ok(json) = serde_json::to_string_pretty(&status) {
                println!("{}", json);
            }
        } else {
            print_status_formatted(&status, self.detailed);
        }

        if status.device.connected {
            exit_codes::SUCCESS
        } else {
            exit_codes::DEVICE_NOT_FOUND
        }
    }

    fn run_watch_mode(&self, json_output: bool) -> i32 {
        loop {
            // Clear screen
            print!("\x1B[2J\x1B[H");

            let status = gather_full_status(self.detailed);

            if json_output {
                if let Ok(json) = serde_json::to_string_pretty(&status) {
                    println!("{}", json);
                }
            } else {
                print_status_formatted(&status, self.detailed);
                println!("\n(Press Ctrl+C to exit)");
            }

            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    }
}

/// Gather comprehensive status information
fn gather_full_status(detailed: bool) -> FullStatus {
    let config = Config::load();
    let device_status = check_device_status(detailed);
    let current_state = get_current_state();

    FullStatus {
        daemon: DaemonStatus {
            running: false, // TODO: Check if daemon is running
            pid: None,
            uptime_seconds: None,
            log_level: config.daemon.log_level.clone(),
        },
        device: device_status,
        state: current_state,
        features: get_feature_status(&config),
        sensitivity: SensitivityStatus {
            dead_zone: config.sensitivity.dead_zone,
            multiplier: config.sensitivity.multiplier,
            invert: config.sensitivity.invert,
            preset: config.sensitivity.preset.clone(),
        },
        audio_devices: if detailed {
            list_audio_devices()
        } else {
            Vec::new()
        },
    }
}

/// Check Surface Dial device status
fn check_device_status(detailed: bool) -> DeviceStatus {
    let api = match HidApi::new() {
        Ok(api) => api,
        Err(_) => {
            return DeviceStatus {
                connected: false,
                device_count: 0,
                vendor_id: None,
                product_id: None,
                product_name: None,
                interfaces: Vec::new(),
            };
        }
    };

    let mut interfaces = Vec::new();
    let mut device_count = 0;
    let mut product_name = None;

    for dev in api.device_list() {
        if dev.vendor_id() == SURFACE_DIAL_VENDOR_ID
            && dev.product_id() == SURFACE_DIAL_PRODUCT_ID
        {
            device_count += 1;

            if product_name.is_none() {
                product_name = dev.product_string().map(|s| s.to_string());
            }

            if detailed {
                interfaces.push(DeviceInterface {
                    path: dev.path().to_string_lossy().to_string(),
                    interface_number: dev.interface_number(),
                });
            }
        }
    }

    DeviceStatus {
        connected: device_count > 0,
        device_count,
        vendor_id: if device_count > 0 {
            Some(format!("{:04X}", SURFACE_DIAL_VENDOR_ID))
        } else {
            None
        },
        product_id: if device_count > 0 {
            Some(format!("{:04X}", SURFACE_DIAL_PRODUCT_ID))
        } else {
            None
        },
        product_name,
        interfaces,
    }
}

/// Get current control state
fn get_current_state() -> CurrentState {
    use crate::platform::{CurrentPlatform, Platform};

    let platform = CurrentPlatform::new();

    CurrentState {
        mode: "Volume".to_string(), // Default mode
        volume_percent: platform.get_volume().ok(),
        muted: platform.is_muted().ok(),
        mic_percent: platform.get_mic_volume().ok(),
        mic_muted: platform.is_mic_muted().ok(),
    }
}

/// Get feature status from config
fn get_feature_status(config: &Config) -> FeatureStatus {
    FeatureStatus {
        osd: FeatureInfo {
            enabled: config.osd.enabled,
            detail: Some(format!("{}, {}ms", config.osd.position, config.osd.timeout_ms)),
        },
        battery_monitor: FeatureInfo {
            enabled: config.battery.enabled,
            detail: Some(format!("poll: {}s", config.battery.poll_interval_seconds)),
        },
        system_tray: FeatureInfo {
            enabled: config.tray.enabled,
            detail: None,
        },
        media_control: FeatureInfo {
            enabled: config.media_control.enabled,
            detail: Some(config.media_control.triple_click_action.clone()),
        },
        audio_feedback: FeatureInfo {
            enabled: config.audio_feedback.enabled,
            detail: None,
        },
        event_hooks: FeatureInfo {
            enabled: config.events.enabled,
            detail: Some(format!(
                "{} webhooks",
                config.events.webhooks.len()
            )),
        },
        device_switching: FeatureInfo {
            enabled: config.device_switching.enabled,
            detail: Some(config.device_switching.mode.clone()),
        },
    }
}

/// List audio devices (platform-specific)
fn list_audio_devices() -> Vec<AudioDeviceInfo> {
    use crate::platform::{CurrentPlatform, Platform};

    let platform = CurrentPlatform::new();
    let mut devices = Vec::new();

    // Try to get output devices
    if let Ok(outputs) = platform.list_output_devices() {
        for dev in outputs {
            devices.push(AudioDeviceInfo {
                name: dev.name,
                device_type: format!("{:?}", dev.device_type).to_lowercase(),
                is_default: dev.is_default,
            });
        }
    }

    devices
}

/// Print status in human-readable format with box drawing
fn print_status_formatted(status: &FullStatus, detailed: bool) {
    const CYAN: &str = "\x1b[36m";
    const GREEN: &str = "\x1b[32m";
    const RED: &str = "\x1b[31m";
    const BOLD: &str = "\x1b[1m";
    const RESET: &str = "\x1b[0m";

    println!(
        "{CYAN}╭─ Surface Dial Status ────────────────────────────────────────╮{RESET}"
    );
    println!("{CYAN}│{RESET}");

    // Device section
    println!("{CYAN}│{RESET}  {BOLD}Device{RESET}");
    if status.device.connected {
        println!(
            "{CYAN}│{RESET}    {GREEN}✓{RESET} Surface Dial connected ({} interface{})",
            status.device.device_count,
            if status.device.device_count != 1 { "s" } else { "" }
        );
        if let (Some(vid), Some(pid)) = (&status.device.vendor_id, &status.device.product_id) {
            println!("{CYAN}│{RESET}    • VID:PID {vid}:{pid}");
        }
        if let Some(ref name) = status.device.product_name {
            println!("{CYAN}│{RESET}    • Product: {name}");
        }
    } else {
        println!("{CYAN}│{RESET}    {RED}✗{RESET} Not connected");
        println!("{CYAN}│{RESET}");
        println!("{CYAN}│{RESET}    Make sure the dial is:");
        println!("{CYAN}│{RESET}      1. Paired via Bluetooth settings");
        println!("{CYAN}│{RESET}      2. Powered on (press and hold center button)");
        println!("{CYAN}│{RESET}      3. Within range of your computer");
    }
    println!("{CYAN}│{RESET}");

    // Current State section
    println!("{CYAN}│{RESET}  {BOLD}Current State{RESET}");
    println!("{CYAN}│{RESET}    • Mode: {}", status.state.mode);
    if let Some(vol) = status.state.volume_percent {
        let muted_str = if status.state.muted == Some(true) {
            format!(" {RED}(muted){RESET}")
        } else {
            String::new()
        };
        println!("{CYAN}│{RESET}    • Volume: {vol}%{muted_str}");
    }
    if let Some(mic) = status.state.mic_percent {
        let muted_str = if status.state.mic_muted == Some(true) {
            format!(" {RED}(muted){RESET}")
        } else {
            String::new()
        };
        println!("{CYAN}│{RESET}    • Mic: {mic}%{muted_str}");
    }
    println!("{CYAN}│{RESET}");

    // Features section
    println!("{CYAN}│{RESET}  {BOLD}Features{RESET}");
    print_feature_line("OSD", &status.features.osd);
    print_feature_line("Battery Monitor", &status.features.battery_monitor);
    print_feature_line("System Tray", &status.features.system_tray);
    print_feature_line("Media Control", &status.features.media_control);
    print_feature_line("Audio Feedback", &status.features.audio_feedback);
    print_feature_line("Event Hooks", &status.features.event_hooks);
    print_feature_line("Device Switch", &status.features.device_switching);
    println!("{CYAN}│{RESET}");

    // Sensitivity section
    println!("{CYAN}│{RESET}  {BOLD}Sensitivity{RESET}");
    println!("{CYAN}│{RESET}    • Dead zone: {}", status.sensitivity.dead_zone);
    println!("{CYAN}│{RESET}    • Multiplier: {:.1}x", status.sensitivity.multiplier);
    println!(
        "{CYAN}│{RESET}    • Invert: {}",
        if status.sensitivity.invert { "Yes" } else { "No" }
    );
    println!("{CYAN}│{RESET}    • Preset: {}", status.sensitivity.preset);

    // Audio devices section (if detailed)
    if detailed && !status.audio_devices.is_empty() {
        println!("{CYAN}│{RESET}");
        println!("{CYAN}│{RESET}  {BOLD}Audio Devices{RESET}");
        for dev in &status.audio_devices {
            let default_marker = if dev.is_default {
                format!(" {GREEN}(default){RESET}")
            } else {
                String::new()
            };
            println!(
                "{CYAN}│{RESET}    • {}: {}{}",
                dev.device_type, dev.name, default_marker
            );
        }
    }

    // Interface details (if detailed and connected)
    if detailed && !status.device.interfaces.is_empty() {
        println!("{CYAN}│{RESET}");
        println!("{CYAN}│{RESET}  {BOLD}Device Interfaces{RESET}");
        for iface in &status.device.interfaces {
            println!("{CYAN}│{RESET}    • Interface {}: {}", iface.interface_number, iface.path);
        }
    }

    println!("{CYAN}│{RESET}");
    println!(
        "{CYAN}╰──────────────────────────────────────────────────────────────╯{RESET}"
    );
}

fn print_feature_line(name: &str, info: &FeatureInfo) {
    const CYAN: &str = "\x1b[36m";
    const GREEN: &str = "\x1b[32m";
    const RED: &str = "\x1b[31m";
    const DIM: &str = "\x1b[2m";
    const RESET: &str = "\x1b[0m";

    let icon = if info.enabled {
        format!("{GREEN}✓{RESET}")
    } else {
        format!("{RED}✗{RESET}")
    };

    let detail_str = match (&info.detail, info.enabled) {
        (Some(d), true) => format!(" {DIM}({d}){RESET}"),
        (_, false) => format!(" {DIM}(disabled){RESET}"),
        _ => String::new(),
    };

    println!("{CYAN}│{RESET}    {icon} {name:14}{detail_str}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_cmd_default() {
        let cmd = StatusCmd::default();
        assert!(!cmd.detailed);
        assert!(!cmd.check);
        assert!(!cmd.watch);
    }

    #[test]
    fn test_full_status_serialization() {
        let status = FullStatus {
            daemon: DaemonStatus {
                running: true,
                pid: Some(12345),
                uptime_seconds: Some(3600),
                log_level: "info".to_string(),
            },
            device: DeviceStatus {
                connected: true,
                device_count: 1,
                vendor_id: Some("045E".to_string()),
                product_id: Some("091B".to_string()),
                product_name: Some("Surface Dial".to_string()),
                interfaces: vec![],
            },
            state: CurrentState {
                mode: "Volume".to_string(),
                volume_percent: Some(65),
                muted: Some(false),
                mic_percent: Some(80),
                mic_muted: Some(false),
            },
            features: FeatureStatus {
                osd: FeatureInfo {
                    enabled: true,
                    detail: Some("center-bottom, 1500ms".to_string()),
                },
                battery_monitor: FeatureInfo {
                    enabled: true,
                    detail: Some("poll: 300s".to_string()),
                },
                system_tray: FeatureInfo {
                    enabled: true,
                    detail: None,
                },
                media_control: FeatureInfo {
                    enabled: true,
                    detail: Some("play_pause".to_string()),
                },
                audio_feedback: FeatureInfo {
                    enabled: false,
                    detail: None,
                },
                event_hooks: FeatureInfo {
                    enabled: false,
                    detail: Some("0 webhooks".to_string()),
                },
                device_switching: FeatureInfo {
                    enabled: true,
                    detail: Some("long_press_rotate".to_string()),
                },
            },
            sensitivity: SensitivityStatus {
                dead_zone: 0,
                multiplier: 1.0,
                invert: false,
                preset: "default".to_string(),
            },
            audio_devices: vec![],
        };

        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("\"running\":true"));
        assert!(json.contains("\"connected\":true"));
        assert!(json.contains("Surface Dial"));
    }

    #[test]
    fn test_device_status_disconnected() {
        let status = DeviceStatus {
            connected: false,
            device_count: 0,
            vendor_id: None,
            product_id: None,
            product_name: None,
            interfaces: Vec::new(),
        };

        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("\"connected\":false"));
        assert!(!json.contains("vendor_id")); // Skip None
    }

    #[test]
    fn test_check_device_status() {
        // This test will pass whether or not a device is connected
        let status = check_device_status(true);
        assert!(status.device_count == 0 || status.connected);
    }

    #[test]
    fn test_get_feature_status() {
        let config = Config::default();
        let features = get_feature_status(&config);

        assert!(features.osd.enabled);
        assert!(features.battery_monitor.enabled);
        assert!(!features.audio_feedback.enabled);
    }

    #[test]
    fn test_current_state_default() {
        let state = get_current_state();
        assert_eq!(state.mode, "Volume");
    }
}
