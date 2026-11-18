use std::path::{Path, PathBuf};

use serde::{de::DeserializeOwned, Serialize};

/// Load a JSON document from disk.
///
/// `Ok(None)` means the file does not exist yet. A corrupt file is moved aside
/// (never silently treated as empty) and reported as `Err`, so callers can keep
/// the user informed without destroying the original bytes.
pub fn load_json<T: DeserializeOwned>(path: &Path) -> Result<Option<T>, String> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(io_error("读取", path, &err)),
    };
    let name = label(path);

    match serde_json::from_str::<T>(&text) {
        Ok(value) => Ok(Some(value)),
        Err(err) => {
            let backup = backup_corrupt(path);
            Err(match backup {
                Some(path) => format!(
                    "[DATA] {name} 内容损坏（{err}），原文件已备份为 {}",
                    path.display()
                ),
                None => format!("[DATA] {name} 内容损坏（{err}），备份失败"),
            })
        }
    }
}

/// Atomically write JSON: serialize, write a uniquely named temp file in the
/// same directory, then rename over the destination.
pub fn save_json<T: Serialize + ?Sized>(path: &Path, value: &T) -> Result<(), String> {
    let text = serde_json::to_string_pretty(value)
        .map_err(|err| format!("[DATA] 序列化 {} 失败: {err}", label(path)))?;
    write_bytes(path, text.as_bytes())
}

pub fn write_bytes(path: &Path, bytes: &[u8]) -> Result<(), String> {
    write_atomic(path, bytes, false)
}

/// 敏感文件写入（如 `secrets.bin`）：Unix 下临时文件创建即 0600，
/// rename 后目标文件不会出现 group/other 可读窗口；Windows 继续依赖 DPAPI。
pub fn write_private_bytes(path: &Path, bytes: &[u8]) -> Result<(), String> {
    write_atomic(path, bytes, true)
}

/// 收紧已存在敏感文件的权限（Unix 0600）。旧版本遗留的 `secrets.bin`
/// 可能仍是 0644，读取时统一收紧，而不是只在新写入时设置。
/// 返回是否发生了权限变更；非 Unix 平台始终返回 false。
pub fn restrict_private(path: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let Ok(meta) = std::fs::metadata(path) else {
            return false;
        };
        if meta.permissions().mode() & 0o777 == PRIVATE_FILE_MODE {
            return false;
        }
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(PRIVATE_FILE_MODE))
            .is_ok()
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        false
    }
}

fn write_atomic(path: &Path, bytes: &[u8], private: bool) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| io_error("创建目录", path, &err))?;
    }

    let temp = unique_temp(path);
    if let Err(err) = write_temp(&temp, bytes, private) {
        let _ = std::fs::remove_file(&temp);
        return Err(io_error("写入", path, &err));
    }
    if let Err(err) = std::fs::rename(&temp, path) {
        let _ = std::fs::remove_file(&temp);
        return Err(io_error("保存", path, &err));
    }
    Ok(())
}

/// `private = true` 时，Unix 用 `OpenOptions::mode(0600)` 创建临时文件再写入。
fn write_temp(path: &Path, bytes: &[u8], private: bool) -> std::io::Result<()> {
    #[cfg(unix)]
    if private {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(PRIVATE_FILE_MODE)
            .open(path)?;
        file.write_all(bytes)?;
        return file.sync_all();
    }
    #[cfg(not(unix))]
    let _ = private;
    std::fs::write(path, bytes)
}

#[cfg(unix)]
const PRIVATE_FILE_MODE: u32 = 0o600;

pub fn load_bytes(path: &Path) -> Result<Option<Vec<u8>>, String> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(io_error("读取", path, &err)),
    }
}

/// Map io failures onto the codes the UI acts on: permission problems get a
/// fix-permissions hint, everything else points at the path / data.
fn io_error(action: &str, path: &Path, err: &std::io::Error) -> String {
    let code = match err.kind() {
        std::io::ErrorKind::PermissionDenied => "PERMISSION",
        std::io::ErrorKind::NotFound => "PATH",
        _ => "DATA",
    };
    format!("[{code}] {action} {} 失败: {err}", label(path))
}

pub fn backup_corrupt(path: &Path) -> Option<PathBuf> {
    let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
    let file_name = path.file_name()?.to_string_lossy().to_string();
    let backup = path.with_file_name(format!("{file_name}.corrupt-{stamp}"));
    std::fs::rename(path, &backup).ok().map(|_| backup)
}

fn unique_temp(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_else(|| "data".into());
    path.with_file_name(format!(".{file_name}.tmp-{}", uuid::Uuid::new_v4()))
}

fn label(path: &Path) -> String {
    path.file_name()
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir() -> PathBuf {
        std::env::temp_dir().join(format!("toolbox-persist-{}", uuid::Uuid::new_v4()))
    }

    #[test]
    fn missing_file_is_not_an_error() {
        let dir = temp_dir();
        let loaded: Option<Vec<String>> = load_json(&dir.join("missing.json")).unwrap();
        assert!(loaded.is_none());
    }

    #[test]
    fn corrupt_file_is_backed_up_and_reported() {
        let dir = temp_dir();
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("data.json");
        std::fs::write(&path, "{ not json").unwrap();

        let result = load_json::<Vec<String>>(&path);
        assert!(result.is_err());
        assert!(!path.exists(), "corrupt file must be moved aside");

        let backups: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .map(|entry| entry.file_name().to_string_lossy().to_string())
            .filter(|name| name.starts_with("data.json.corrupt-"))
            .collect();
        assert_eq!(backups.len(), 1);
    }

    #[test]
    fn save_replaces_existing_and_leaves_no_temp_files() {
        let dir = temp_dir();
        let path = dir.join("data.json");
        save_json(&path, &vec!["first".to_string()]).unwrap();
        save_json(&path, &vec!["second".to_string()]).unwrap();

        let loaded: Vec<String> = load_json(&path).unwrap().unwrap();
        assert_eq!(loaded, vec!["second".to_string()]);
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1);
    }

    #[test]
    fn concurrent_writes_stay_readable() {
        let dir = temp_dir();
        let path = dir.join("concurrent.json");
        let mut handles = Vec::new();
        for index in 0..20 {
            let path = path.clone();
            handles.push(std::thread::spawn(move || {
                let value = vec![format!("value-{index}"); 32];
                save_json(&path, &value).unwrap();
            }));
        }
        for handle in handles {
            handle.join().unwrap();
        }
        let loaded: Vec<String> = load_json(&path).unwrap().unwrap();
        assert_eq!(loaded.len(), 32);
    }

    #[cfg(unix)]
    #[test]
    fn private_write_is_0600_and_tightens_existing_file() {
        use std::os::unix::fs::PermissionsExt;

        let dir = temp_dir();
        let path = dir.join("secrets.bin");
        write_private_bytes(&path, b"secret").unwrap();
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "新建敏感文件必须是 0600");

        // 模拟旧版本遗留的宽松权限：读取时收紧，而不是只在新写入时设置。
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert!(restrict_private(&path));
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
        assert!(!restrict_private(&path), "已是 0600 时不应重复报告变更");
        let _ = std::fs::remove_dir_all(dir);
    }
}
