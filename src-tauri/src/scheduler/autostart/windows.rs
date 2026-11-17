//! Windows 开机自启：注册表 `HKCU\...\CurrentVersion\Run` 启动项。

use crate::models::AutostartStatus;
use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_SET_VALUE};
use winreg::RegKey;

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const VALUE_NAME: &str = "Toolbox";

pub fn status() -> AutostartStatus {
    let expected_path = super::expected_path();
    match read_value() {
        Some(path) => AutostartStatus {
            supported: true,
            enabled: true,
            path,
            expected_path,
        },
        None => AutostartStatus {
            supported: true,
            enabled: false,
            path: String::new(),
            expected_path,
        },
    }
}

pub fn set_enabled(enabled: bool) -> Result<AutostartStatus, String> {
    if enabled {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let (key, _) = hkcu
            .create_subkey(RUN_KEY)
            .map_err(|err| format!("[PERMISSION] 打开注册表启动项失败: {err}"))?;
        key.set_value(VALUE_NAME, &super::expected_path())
            .map_err(|err| format!("[PERMISSION] 写入开机自启失败: {err}"))?;
    } else {
        match delete_value() {
            Ok(()) => {}
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => return Err(format!("[PERMISSION] 关闭开机自启失败: {err}")),
        }
    }
    Ok(status())
}

fn read_value() -> Option<String> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = hkcu.open_subkey_with_flags(RUN_KEY, KEY_READ).ok()?;
    key.get_value::<String, _>(VALUE_NAME).ok()
}

fn delete_value() -> std::io::Result<()> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = hkcu.open_subkey_with_flags(RUN_KEY, KEY_SET_VALUE)?;
    key.delete_value(VALUE_NAME)
}
