//! 开机自启：公共接口与平台分发。
//!
//! - Windows：注册表 `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`
//! - macOS：launchd 用户代理 `~/Library/LaunchAgents/<label>.plist`
//! - Linux：XDG 自启 `~/.config/autostart/<id>.desktop`

use crate::models::AutostartStatus;

#[cfg(windows)]
mod windows;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(all(unix, not(target_os = "macos")))]
mod linux;

pub fn status() -> AutostartStatus {
    #[cfg(windows)]
    {
        windows::status()
    }
    #[cfg(target_os = "macos")]
    {
        macos::status()
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        linux::status()
    }
    #[cfg(not(any(windows, unix)))]
    {
        AutostartStatus {
            supported: false,
            enabled: false,
            path: String::new(),
            expected_path: expected_path(),
        }
    }
}

pub fn set_enabled(enabled: bool) -> Result<AutostartStatus, String> {
    #[cfg(windows)]
    {
        windows::set_enabled(enabled)
    }
    #[cfg(target_os = "macos")]
    {
        macos::set_enabled(enabled)
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        linux::set_enabled(enabled)
    }
    #[cfg(not(any(windows, unix)))]
    {
        let _ = enabled;
        Err("[TOOL] 当前平台不支持开机自启".into())
    }
}

/// 期望的启动命令（供前端比对启动项是否指向当前程序）。
pub fn expected_path() -> String {
    std::env::current_exe()
        .map(|path| format!("\"{}\" --autostart", path.display()))
        .unwrap_or_default()
}
