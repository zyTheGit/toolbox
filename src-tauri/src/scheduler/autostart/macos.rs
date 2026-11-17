//! macOS 开机自启：launchd 用户级代理（`~/Library/LaunchAgents`）。

use crate::models::AutostartStatus;
use std::path::PathBuf;

const LABEL: &str = "com.toolbox.desktop";

fn plist_path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(
        PathBuf::from(home)
            .join("Library/LaunchAgents")
            .join(format!("{LABEL}.plist")),
    )
}

/// 生成固定格式的 launchd 配置，启动参数与 Windows 启动项保持一致。
fn build_plist(exe: &str) -> String {
    let escaped = exe
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
         <plist version=\"1.0\">\n\
         <dict>\n\
         \t<key>Label</key>\n\
         \t<string>{LABEL}</string>\n\
         \t<key>ProgramArguments</key>\n\
         \t<array>\n\
         \t\t<string>{escaped}</string>\n\
         \t\t<string>--autostart</string>\n\
         \t</array>\n\
         \t<key>RunAtLoad</key>\n\
         \t<true/>\n\
         </dict>\n\
         </plist>\n"
    )
}

/// 从 plist 提取程序路径（跳过 Label 与 --autostart 参数），用于与期望路径比对。
fn extract_program(content: &str) -> Option<String> {
    for part in content.split("<string>").skip(1) {
        let value = part.split("</string>").next().unwrap_or_default();
        let value = value
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&amp;", "&");
        if value != LABEL && value != "--autostart" && !value.is_empty() {
            return Some(value);
        }
    }
    None
}

pub fn status() -> AutostartStatus {
    let expected_path = super::expected_path();
    let Some(dest) = plist_path() else {
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
            .and_then(|content| extract_program(&content))
            .map(|exe| format!("\"{exe}\" --autostart"))
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
    let Some(dest) = plist_path() else {
        return Err("[PERMISSION] 无法确定用户目录，开机自启设置失败".into());
    };
    if enabled {
        let exe = std::env::current_exe()
            .map_err(|err| format!("[PERMISSION] 无法获取程序路径: {err}"))?;
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|err| format!("[PERMISSION] 创建 LaunchAgents 目录失败: {err}"))?;
        }
        std::fs::write(&dest, build_plist(&exe.display().to_string()))
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
