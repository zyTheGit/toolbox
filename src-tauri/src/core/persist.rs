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
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| io_error("创建目录", path, &err))?;
    }

    let temp = unique_temp(path);
    if let Err(err) = std::fs::write(&temp, bytes) {
        let _ = std::fs::remove_file(&temp);
        return Err(io_error("写入", path, &err));
    }
    if let Err(err) = std::fs::rename(&temp, path) {
        let _ = std::fs::remove_file(&temp);
        return Err(io_error("保存", path, &err));
    }
    Ok(())
}

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
}
