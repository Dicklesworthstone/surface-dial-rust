//! Windows platform implementation
//!
//! Uses PowerShell for most operations as a fallback.
//! For production use, this should be replaced with native Windows APIs
//! via the windows-rs crate for better performance.

use super::*;
use std::process::Command;

/// Windows platform implementation
pub struct Windows;

impl Windows {
    /// Create a new Windows platform instance
    pub fn new() -> Self {
        Self
    }

    /// Run a PowerShell command and return stdout
    fn powershell(&self, script: &str) -> PlatformResult<String> {
        let output = Command::new("powershell")
            .args(["-NoProfile", "-Command", script])
            .output()
            .map_err(|e| PlatformError::OperationFailed(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            return Err(PlatformError::OperationFailed(stderr));
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    /// Run a PowerShell command, ignoring output
    fn powershell_quiet(&self, script: &str) -> PlatformResult<()> {
        let output = Command::new("powershell")
            .args(["-NoProfile", "-Command", script])
            .output()
            .map_err(|e| PlatformError::OperationFailed(e.to_string()))?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            Err(PlatformError::OperationFailed(stderr))
        }
    }
}

impl Default for Windows {
    fn default() -> Self {
        Self::new()
    }
}

impl Platform for Windows {
    fn get_volume(&self) -> PlatformResult<i32> {
        // Use AudioDeviceCmdlets module or nircmd
        // This is a simplified approach using PowerShell
        let script = r#"
            Add-Type -TypeDefinition @"
            using System.Runtime.InteropServices;
            [Guid("5CDF2C82-841E-4546-9722-0CF74078229A"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
            interface IAudioEndpointVolume {
                int f(); int g(); int h(); int i();
                int SetMasterVolumeLevelScalar(float fLevel, System.Guid pguidEventContext);
                int j();
                int GetMasterVolumeLevelScalar(out float pfLevel);
                int k(); int l(); int m(); int n();
                int GetMute(out bool pbMute);
                int SetMute(bool bMute, System.Guid pguidEventContext);
            }
            [Guid("D666063F-1587-4E43-81F1-B948E807363F"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
            interface IMMDevice { int Activate(ref System.Guid id, int clsCtx, int activationParams, out IAudioEndpointVolume aev); }
            [Guid("A95664D2-9614-4F35-A746-DE8DB63617E6"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
            interface IMMDeviceEnumerator { int f(); int GetDefaultAudioEndpoint(int dataFlow, int role, out IMMDevice endpoint); }
            [ComImport, Guid("BCDE0395-E52F-467C-8E3D-C4579291692E")] class MMDeviceEnumeratorComObject { }
            public class Audio {
                static IAudioEndpointVolume Vol() {
                    var enumerator = new MMDeviceEnumeratorComObject() as IMMDeviceEnumerator;
                    IMMDevice dev; enumerator.GetDefaultAudioEndpoint(0, 1, out dev);
                    IAudioEndpointVolume epv; var epvid = typeof(IAudioEndpointVolume).GUID;
                    dev.Activate(ref epvid, 23, 0, out epv); return epv;
                }
                public static float GetVolume() { float v; Vol().GetMasterVolumeLevelScalar(out v); return v; }
                public static void SetVolume(float v) { Vol().SetMasterVolumeLevelScalar(v, System.Guid.Empty); }
                public static bool GetMute() { bool m; Vol().GetMute(out m); return m; }
                public static void SetMute(bool m) { Vol().SetMute(m, System.Guid.Empty); }
            }
"@
            [Math]::Round([Audio]::GetVolume() * 100)
        "#;

        let output = self.powershell(script)?;
        output
            .parse()
            .map_err(|_| PlatformError::ParseError(format!("Cannot parse volume: {}", output)))
    }

    fn set_volume(&self, vol: i32) -> PlatformResult<()> {
        let vol = vol.clamp(0, 100);
        let vol_float = vol as f64 / 100.0;

        // Simplified approach - use nircmd if available, or PowerShell
        let script = format!(
            r#"
            # Try nircmd first (faster)
            $nircmd = Get-Command nircmd -ErrorAction SilentlyContinue
            if ($nircmd) {{
                nircmd setsysvolume {}
            }} else {{
                # PowerShell fallback
                $vol = {}
                Add-Type -TypeDefinition @"
                using System.Runtime.InteropServices;
                [Guid("5CDF2C82-841E-4546-9722-0CF74078229A"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
                interface IAudioEndpointVolume {{
                    int f(); int g(); int h(); int i();
                    int SetMasterVolumeLevelScalar(float fLevel, System.Guid pguidEventContext);
                    int j();
                    int GetMasterVolumeLevelScalar(out float pfLevel);
                    int k(); int l(); int m(); int n();
                    int GetMute(out bool pbMute);
                    int SetMute(bool bMute, System.Guid pguidEventContext);
                }}
                [Guid("D666063F-1587-4E43-81F1-B948E807363F"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
                interface IMMDevice {{ int Activate(ref System.Guid id, int clsCtx, int activationParams, out IAudioEndpointVolume aev); }}
                [Guid("A95664D2-9614-4F35-A746-DE8DB63617E6"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
                interface IMMDeviceEnumerator {{ int f(); int GetDefaultAudioEndpoint(int dataFlow, int role, out IMMDevice endpoint); }}
                [ComImport, Guid("BCDE0395-E52F-467C-8E3D-C4579291692E")] class MMDeviceEnumeratorComObject {{ }}
                public class Audio {{
                    public static void SetVolume(float v) {{
                        var enumerator = new MMDeviceEnumeratorComObject() as IMMDeviceEnumerator;
                        IMMDevice dev; enumerator.GetDefaultAudioEndpoint(0, 1, out dev);
                        IAudioEndpointVolume epv; var epvid = typeof(IAudioEndpointVolume).GUID;
                        dev.Activate(ref epvid, 23, 0, out epv);
                        epv.SetMasterVolumeLevelScalar(v, System.Guid.Empty);
                    }}
                }}
"@
                [Audio]::SetVolume($vol)
            }}
        "#,
            (vol as f64 / 100.0 * 65535.0) as i32,
            vol_float
        );

        self.powershell_quiet(&script)
    }

    fn is_muted(&self) -> PlatformResult<bool> {
        let script = r#"
            Add-Type -TypeDefinition @"
            using System.Runtime.InteropServices;
            [Guid("5CDF2C82-841E-4546-9722-0CF74078229A"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
            interface IAudioEndpointVolume {
                int f(); int g(); int h(); int i(); int j(); int k();
                int GetMasterVolumeLevelScalar(out float pfLevel);
                int l(); int m(); int n(); int o();
                int GetMute(out bool pbMute);
                int SetMute(bool bMute, System.Guid pguidEventContext);
            }
            [Guid("D666063F-1587-4E43-81F1-B948E807363F"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
            interface IMMDevice { int Activate(ref System.Guid id, int clsCtx, int activationParams, out IAudioEndpointVolume aev); }
            [Guid("A95664D2-9614-4F35-A746-DE8DB63617E6"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
            interface IMMDeviceEnumerator { int f(); int GetDefaultAudioEndpoint(int dataFlow, int role, out IMMDevice endpoint); }
            [ComImport, Guid("BCDE0395-E52F-467C-8E3D-C4579291692E")] class MMDeviceEnumeratorComObject { }
            public class Audio {
                public static bool GetMute() {
                    var enumerator = new MMDeviceEnumeratorComObject() as IMMDeviceEnumerator;
                    IMMDevice dev; enumerator.GetDefaultAudioEndpoint(0, 1, out dev);
                    IAudioEndpointVolume epv; var epvid = typeof(IAudioEndpointVolume).GUID;
                    dev.Activate(ref epvid, 23, 0, out epv);
                    bool m; epv.GetMute(out m); return m;
                }
            }
"@
            [Audio]::GetMute()
        "#;

        let output = self.powershell(script)?;
        Ok(output.to_lowercase() == "true")
    }

    fn toggle_mute(&self) -> PlatformResult<()> {
        let muted = self.is_muted()?;
        let script = format!(
            r#"
            Add-Type -TypeDefinition @"
            using System.Runtime.InteropServices;
            [Guid("5CDF2C82-841E-4546-9722-0CF74078229A"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
            interface IAudioEndpointVolume {{
                int f(); int g(); int h(); int i(); int j(); int k();
                int GetMasterVolumeLevelScalar(out float pfLevel);
                int l(); int m(); int n(); int o();
                int GetMute(out bool pbMute);
                int SetMute(bool bMute, System.Guid pguidEventContext);
            }}
            [Guid("D666063F-1587-4E43-81F1-B948E807363F"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
            interface IMMDevice {{ int Activate(ref System.Guid id, int clsCtx, int activationParams, out IAudioEndpointVolume aev); }}
            [Guid("A95664D2-9614-4F35-A746-DE8DB63617E6"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
            interface IMMDeviceEnumerator {{ int f(); int GetDefaultAudioEndpoint(int dataFlow, int role, out IMMDevice endpoint); }}
            [ComImport, Guid("BCDE0395-E52F-467C-8E3D-C4579291692E")] class MMDeviceEnumeratorComObject {{ }}
            public class Audio {{
                public static void SetMute(bool m) {{
                    var enumerator = new MMDeviceEnumeratorComObject() as IMMDeviceEnumerator;
                    IMMDevice dev; enumerator.GetDefaultAudioEndpoint(0, 1, out dev);
                    IAudioEndpointVolume epv; var epvid = typeof(IAudioEndpointVolume).GUID;
                    dev.Activate(ref epvid, 23, 0, out epv);
                    epv.SetMute(m, System.Guid.Empty);
                }}
            }}
"@
            [Audio]::SetMute(${})
        "#,
            !muted
        );

        self.powershell_quiet(&script)
    }

    fn get_mic_volume(&self) -> PlatformResult<i32> {
        // Similar to get_volume but for input device (dataFlow = 1)
        // For now, return a placeholder
        Err(PlatformError::NotImplemented)
    }

    fn set_mic_volume(&self, _vol: i32) -> PlatformResult<()> {
        Err(PlatformError::NotImplemented)
    }

    fn is_mic_muted(&self) -> PlatformResult<bool> {
        Err(PlatformError::NotImplemented)
    }

    fn toggle_mic_mute(&self) -> PlatformResult<()> {
        Err(PlatformError::NotImplemented)
    }

    fn send_key_down(&self, key: Key) -> PlatformResult<()> {
        let key_code = match key {
            Key::F15 => "0x7E", // VK_F15
            Key::F16 => "0x7F", // VK_F16
            Key::F17 => "0x80", // VK_F17
            Key::F18 => "0x81", // VK_F18
            Key::F19 => "0x82", // VK_F19
        };

        let script = format!(
            r#"
            Add-Type @"
            using System;
            using System.Runtime.InteropServices;
            public class Keyboard {{
                [DllImport("user32.dll")]
                public static extern void keybd_event(byte bVk, byte bScan, uint dwFlags, UIntPtr dwExtraInfo);
            }}
"@
            [Keyboard]::keybd_event({}, 0, 0, [UIntPtr]::Zero)
        "#,
            key_code
        );

        self.powershell_quiet(&script)
    }

    fn send_key_up(&self, key: Key) -> PlatformResult<()> {
        let key_code = match key {
            Key::F15 => "0x7E",
            Key::F16 => "0x7F",
            Key::F17 => "0x80",
            Key::F18 => "0x81",
            Key::F19 => "0x82",
        };

        let script = format!(
            r#"
            Add-Type @"
            using System;
            using System.Runtime.InteropServices;
            public class Keyboard {{
                [DllImport("user32.dll")]
                public static extern void keybd_event(byte bVk, byte bScan, uint dwFlags, UIntPtr dwExtraInfo);
            }}
"@
            [Keyboard]::keybd_event({}, 0, 2, [UIntPtr]::Zero)  # KEYEVENTF_KEYUP = 2
        "#,
            key_code
        );

        self.powershell_quiet(&script)
    }

    fn send_media_key(&self, key: MediaKey) -> PlatformResult<()> {
        let key_code = match key {
            MediaKey::PlayPause => "0xB3",  // VK_MEDIA_PLAY_PAUSE
            MediaKey::NextTrack => "0xB0",  // VK_MEDIA_NEXT_TRACK
            MediaKey::PrevTrack => "0xB1",  // VK_MEDIA_PREV_TRACK
            MediaKey::VolumeUp => "0xAF",   // VK_VOLUME_UP
            MediaKey::VolumeDown => "0xAE", // VK_VOLUME_DOWN
            MediaKey::Mute => "0xAD",       // VK_VOLUME_MUTE
        };

        let script = format!(
            r#"
            Add-Type @"
            using System;
            using System.Runtime.InteropServices;
            public class Keyboard {{
                [DllImport("user32.dll")]
                public static extern void keybd_event(byte bVk, byte bScan, uint dwFlags, UIntPtr dwExtraInfo);
            }}
"@
            [Keyboard]::keybd_event({}, 0, 0, [UIntPtr]::Zero)
            Start-Sleep -Milliseconds 50
            [Keyboard]::keybd_event({}, 0, 2, [UIntPtr]::Zero)
        "#,
            key_code, key_code
        );

        self.powershell_quiet(&script)
    }

    fn list_output_devices(&self) -> PlatformResult<Vec<AudioDevice>> {
        // TODO: Implement with Windows Core Audio API
        Ok(vec![AudioDevice {
            id: "default".to_string(),
            name: "Default Output".to_string(),
            device_type: DeviceType::Unknown,
            is_default: true,
        }])
    }

    fn get_default_output(&self) -> PlatformResult<AudioDevice> {
        Ok(AudioDevice {
            id: "default".to_string(),
            name: "Default Output".to_string(),
            device_type: DeviceType::Unknown,
            is_default: true,
        })
    }

    fn set_default_output(&self, _device_id: &str) -> PlatformResult<()> {
        Err(PlatformError::NotImplemented)
    }

    fn list_input_devices(&self) -> PlatformResult<Vec<AudioDevice>> {
        Ok(vec![AudioDevice {
            id: "default".to_string(),
            name: "Default Input".to_string(),
            device_type: DeviceType::Unknown,
            is_default: true,
        }])
    }

    fn send_notification(&self, title: &str, body: &str, _urgency: Urgency) -> PlatformResult<()> {
        // Use Windows Toast notification via PowerShell
        let title = title.replace("'", "''");
        let body = body.replace("'", "''");

        let script = format!(
            r#"
            [Windows.UI.Notifications.ToastNotificationManager, Windows.UI.Notifications, ContentType = WindowsRuntime] | Out-Null
            [Windows.Data.Xml.Dom.XmlDocument, Windows.Data.Xml.Dom.XmlDocument, ContentType = WindowsRuntime] | Out-Null
            $template = @"
            <toast>
                <visual>
                    <binding template="ToastText02">
                        <text id="1">{}</text>
                        <text id="2">{}</text>
                    </binding>
                </visual>
            </toast>
"@
            $xml = New-Object Windows.Data.Xml.Dom.XmlDocument
            $xml.LoadXml($template)
            $toast = [Windows.UI.Notifications.ToastNotification]::new($xml)
            [Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier("Surface Dial").Show($toast)
        "#,
            title, body
        );

        self.powershell_quiet(&script)
    }

    fn get_foreground_app(&self) -> PlatformResult<Option<AppInfo>> {
        let script = r#"
            Add-Type @"
            using System;
            using System.Runtime.InteropServices;
            using System.Text;
            public class ForegroundApp {
                [DllImport("user32.dll")]
                public static extern IntPtr GetForegroundWindow();
                [DllImport("user32.dll")]
                public static extern int GetWindowText(IntPtr hWnd, StringBuilder text, int count);
                [DllImport("user32.dll")]
                public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint processId);
            }
"@
            $hwnd = [ForegroundApp]::GetForegroundWindow()
            $title = New-Object System.Text.StringBuilder 256
            [ForegroundApp]::GetWindowText($hwnd, $title, 256)
            $title.ToString()
        "#;

        let output = self.powershell(script)?;
        if output.is_empty() {
            return Ok(None);
        }

        Ok(Some(AppInfo {
            name: output,
            bundle_id: None,
            process_id: None,
            executable: None,
        }))
    }

    fn get_daemon_pid(&self) -> Option<u32> {
        let script = r#"
            $proc = Get-Process -Name "surface-dial" -ErrorAction SilentlyContinue
            if ($proc) { $proc.Id } else { "" }
        "#;

        self.powershell(script)
            .ok()
            .and_then(|s| s.parse().ok())
    }

    fn is_daemon_installed(&self) -> bool {
        // Check for Windows Task Scheduler entry
        let script = r#"
            $task = Get-ScheduledTask -TaskName "SurfaceDialController" -ErrorAction SilentlyContinue
            if ($task) { "true" } else { "false" }
        "#;

        self.powershell(script)
            .map(|s| s == "true")
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_windows() {
        let windows = Windows::new();
        let _ = windows;
    }

    #[test]
    fn test_default_windows() {
        let windows = Windows::default();
        let _ = windows;
    }
}
