//! Linux 开机自启：XDG 自启（`$XDG_CONFIG_HOME/autostart`）。

use crate::models::AutostartStatus;
use std::path::PathBuf;

const DESKTOP_ID: &str = "com.toolbox.desktop";

fn autostart_dir() -> Option<PathBuf> {
    if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
        if !xdg.is_empty() {
            return Some(PathBuf::from(xdg).join("autostart"));
        }
    }
    let home = std::env::var_os("HOME")?;
    Some(PathBuf::from(home).join(".config").join("autostart"))
}

fn desktop_path() -> Option<PathBuf> {
    Some(
        autostart_dir()?.join(format!("{DESKTOP_ID}.desktop")),
    )
}

/// Exec 值与 Windows 启动项格式保持一致：`"<程序路径>" --autostart`。
fn build_desktop(exec: &str) -> String {
    format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Name=Toolbox\n\
         Exec={exec} --autostart\n"
    )
}

fn extract_exec(content: &str) -> Option<String> {
    content
        .lines()
        .find(|line| line.starts_with("Exec="))
        .map(|line| line["Exec=".len()..].trim().to_string())
}

pub fn status() -> AutostartStatus {
    let expected_path = super::expected_path();
    let Some(dest) = desktop_path() else {
        return AutostartStatus {
            supported: false,
            enabled: false,
            path: String::new(),
            expected_path,
        };
    };
    let enabled = dest.exists();
    let path = if enabled {
        std::fs::read_to_string(&dest)
            .ok()
            .and_then(|content| extract_exec(&content))
            .unwrap_or_default()
    } else {
        String::new()
    };
    AutostartStatus {
        supported: true,
        enabled,
        path,
        expected_path,
    }
}

pub fn set_enabled(enabled: bool) -> Result<AutostartStatus, String> {
    let Some(dest) = desktop_path() else {
        return Err("[PERMISSION] 无法确定用户目录，开机自启设置失败".into());
    };
    if enabled {
        let exe = std::env::current_exe()
            .map_err(|err| format!("[PERMISSION] 无法获取程序路径: {err}"))?;
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|err| format!("[PERMISSION] 创建自启目录失败: {err}"))?;
        }
        let exec = format!("\"{}\"", exe.display());
        std::fs::write(&dest, build_desktop(&exec))
            .map_err(|err| format!("[PERMISSION] 写入开机自启配置失败: {err}"))?;
    } else {
        match std::fs::remove_file(&dest) {
            Ok(()) => {}
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => return Err(format!("[PERMISSION] 关闭开机自启失败: {err}")),
        }
    }
    Ok(status())
}
