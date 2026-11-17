use crate::models::AutostartStatus;

#[cfg(windows)]
const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
#[cfg(windows)]
const VALUE_NAME: &str = "Toolbox";

pub fn status() -> AutostartStatus {
    let expected_path = expected_path();
    #[cfg(windows)]
    {
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
    #[cfg(not(windows))]
    {
        AutostartStatus {
            supported: false,
            enabled: false,
            path: String::new(),
            expected_path,
        }
    }
}

pub fn set_enabled(enabled: bool) -> Result<AutostartStatus, String> {
    #[cfg(windows)]
    {
        if enabled {
            use winreg::enums::HKEY_CURRENT_USER;
            use winreg::RegKey;

            let hkcu = RegKey::predef(HKEY_CURRENT_USER);
            let (key, _) = hkcu
                .create_subkey(RUN_KEY)
                .map_err(|err| format!("[PERMISSION] 打开注册表启动项失败: {err}"))?;
            key.set_value(VALUE_NAME, &expected_path())
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
    #[cfg(not(windows))]
    {
        if enabled {
            Err("[TOOL] 开机自启仅支持 Windows".into())
        } else {
            Ok(status())
        }
    }
}

pub fn expected_path() -> String {
    std::env::current_exe()
        .map(|path| format!("\"{}\" --autostart", path.display()))
        .unwrap_or_default()
}

#[cfg(windows)]
fn read_value() -> Option<String> {
    use winreg::enums::{HKEY_CURRENT_USER, KEY_READ};
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = hkcu.open_subkey_with_flags(RUN_KEY, KEY_READ).ok()?;
    key.get_value::<String, _>(VALUE_NAME).ok()
}

#[cfg(windows)]
fn delete_value() -> std::io::Result<()> {
    use winreg::enums::{HKEY_CURRENT_USER, KEY_SET_VALUE};
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = hkcu.open_subkey_with_flags(RUN_KEY, KEY_SET_VALUE)?;
    key.delete_value(VALUE_NAME)
}
