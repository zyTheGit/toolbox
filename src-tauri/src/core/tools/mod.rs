//! 工具管理：yt-dlp / ffmpeg 的检测、状态缓存与安装入口。

mod checksum;
mod install;
mod net;

pub use checksum::{verify_ffmpeg_checksum, verify_ytdlp_checksum};
pub(crate) use checksum::verify_sha256;
pub(crate) use net::{download_file, with_mirror};

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use tauri::AppHandle;

use crate::core::config::expand_home;
use crate::models::{AppConfig, ToolInstallResult, ToolStatus, ToolsStatus};
use crate::state::AppPaths;

pub(crate) const UA: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Safari/537.36 Toolbox/0.1";
const CACHE_TTL: Duration = Duration::from_secs(30);

struct CacheEntry {
    at: Instant,
    status: ToolStatus,
}

static CACHE: OnceLock<Mutex<HashMap<String, CacheEntry>>> = OnceLock::new();

fn cache() -> std::sync::MutexGuard<'static, HashMap<String, CacheEntry>> {
    CACHE
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

pub fn get_status(config: &AppConfig, paths: &AppPaths, force: bool) -> ToolsStatus {
    ToolsStatus {
        ytdlp: status_for("ytdlp", &config.tool_paths.ytdlp, paths, force),
        ffmpeg: status_for("ffmpeg", &config.tool_paths.ffmpeg, paths, force),
    }
}

fn status_for(tool: &str, configured: &str, paths: &AppPaths, force: bool) -> ToolStatus {
    let Some(path) = locate(tool, configured, paths) else {
        return ToolStatus::default();
    };
    let path_text = path.to_string_lossy().to_string();

    if !force {
        if let Some(entry) = cache().get(tool) {
            if entry.status.path == path_text && entry.at.elapsed() < CACHE_TTL {
                return entry.status.clone();
            }
        }
    }

    let version = read_version(tool, &path);
    let status = ToolStatus {
        ok: !version.is_empty(),
        path: path_text,
        version,
    };
    cache().insert(
        tool.to_string(),
        CacheEntry {
            at: Instant::now(),
            status: status.clone(),
        },
    );
    status
}

fn locate(tool: &str, configured: &str, paths: &AppPaths) -> Option<PathBuf> {
    let configured = expand_home(configured.trim());
    if !configured.trim().is_empty() {
        let path = PathBuf::from(configured);
        if path.is_file() {
            return Some(path);
        }
    }

    let bundled = paths.bin_dir.join(exe_file(tool));
    if bundled.is_file() {
        return Some(bundled);
    }

    let binary = if tool == "ytdlp" { "yt-dlp" } else { "ffmpeg" };
    if let Ok(path) = which::which(binary) {
        if path.is_file() {
            return Some(path);
        }
    }
    #[cfg(windows)]
    if let Ok(path) = which::which(format!("{binary}.exe")) {
        if path.is_file() {
            return Some(path);
        }
    }
    None
}

pub(crate) fn exe_file(tool: &str) -> String {
    let base = if tool == "ytdlp" { "yt-dlp" } else { "ffmpeg" };
    if cfg!(windows) {
        format!("{base}.exe")
    } else {
        base.to_string()
    }
}

fn read_version(tool: &str, path: &Path) -> String {
    read_version_with_timeout(tool, path, Duration::from_secs(10))
}

/// Run `<tool> --version` (or `-version`) with a hard timeout so a broken
/// binary can never block tool detection or installation.
pub(crate) fn read_version_with_timeout(tool: &str, path: &Path, timeout: Duration) -> String {
    let mut command = std::process::Command::new(path);
    if tool == "ffmpeg" {
        command.arg("-version");
    } else {
        command.arg("--version");
    }
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }

    let Ok(mut child) = command.spawn() else {
        return String::new();
    };
    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    return String::new();
                }
                use std::io::Read;
                let mut stdout = String::new();
                if let Some(mut pipe) = child.stdout.take() {
                    let _ = pipe.read_to_string(&mut stdout);
                }
                let mut stderr = String::new();
                if let Some(mut pipe) = child.stderr.take() {
                    let _ = pipe.read_to_string(&mut stderr);
                }
                let text = if stdout.trim().is_empty() {
                    stderr
                } else {
                    stdout
                };
                return text
                    .lines()
                    .map(str::trim)
                    .find(|line| !line.is_empty())
                    .map(|line| line.chars().take(160).collect())
                    .unwrap_or_default();
            }
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return String::new();
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(_) => return String::new(),
        }
    }
}

fn tool_display(tool: &str) -> &'static str {
    if tool == "ffmpeg" {
        "ffmpeg"
    } else {
        "yt-dlp"
    }
}

/// Validate the downloaded binary, then swap it in while keeping the previous
/// version as a rollback target. A failed probe restores the old file.
pub(crate) fn install_binary(src: &Path, dest: &Path, tool: &str) -> Result<(), String> {
    if read_version_with_timeout(tool, src, Duration::from_secs(10)).is_empty() {
        let _ = std::fs::remove_file(src);
        return Err(format!(
            "[TOOL] {} 下载内容无法运行，已放弃安装（旧版本保持不变）",
            tool_display(tool)
        ));
    }

    let backup = swap_in(src, dest).map_err(|e| format!("[PATH] 保存 {} 失败: {e}", tool_display(tool)))?;

    if read_version_with_timeout(tool, dest, Duration::from_secs(10)).is_empty() {
        let _ = std::fs::remove_file(dest);
        if let Some(backup) = &backup {
            let _ = std::fs::rename(backup, dest);
        }
        return Err(format!(
            "[TOOL] {} 安装后无法运行，已回滚到旧版本",
            tool_display(tool)
        ));
    }

    if let Some(backup) = backup {
        let _ = std::fs::remove_file(backup);
    }
    Ok(())
}

/// Move `src` over `dest`, renaming the previous file to a backup first. The
/// caller decides whether to delete or restore the returned backup.
pub(crate) fn swap_in(src: &Path, dest: &Path) -> Result<Option<PathBuf>, String> {
    let backup = if dest.exists() {
        let name = dest
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("tool");
        let backup = dest.with_file_name(format!(".{name}.bak-{}", uuid::Uuid::new_v4()));
        std::fs::rename(dest, &backup).map_err(|e| format!("[PATH] 备份旧版本失败: {e}"))?;
        Some(backup)
    } else {
        None
    };

    if let Err(err) = std::fs::rename(src, dest) {
        if let Some(backup) = &backup {
            let _ = std::fs::rename(backup, dest);
        }
        return Err(format!("[PATH] 替换文件失败: {err}"));
    }
    Ok(backup)
}

pub async fn install_tool(
    app: &AppHandle,
    tool: &str,
    config: &AppConfig,
    paths: &AppPaths,
) -> Result<ToolInstallResult, String> {
    let key = match tool {
        "ytdlp" => "ytdlp",
        "ffmpeg" => "ffmpeg",
        _ => return Err(format!("[PARAM] 未知工具: {tool}")),
    };

    std::fs::create_dir_all(&paths.bin_dir)
        .map_err(|e| format!("[PATH] 创建工具目录失败: {e}"))?;
    let client = net::build_client(config)?;

    if key == "ytdlp" {
        install::install_ytdlp(&client, app, config, paths).await?;
    } else {
        install::install_ffmpeg(&client, app, config, paths).await?;
    }

    let tools = get_status(config, paths, true);
    let status = if key == "ytdlp" {
        tools.ytdlp.clone()
    } else {
        tools.ffmpeg.clone()
    };
    if !status.ok {
        return Err(format!(
            "[TOOL] {} 安装后无法运行，请检查杀毒软件拦截或网络后重试",
            tool_display(key)
        ));
    }
    net::emit_progress(app, key, "done", Some(100.0), 0, 0);

    Ok(ToolInstallResult {
        tool: key.to_string(),
        path: status.path,
        version: status.version,
        tools,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("toolbox-tools-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn missing_binary_probe_returns_empty() {
        let dir = temp_dir();
        let missing = dir.join("nope.exe");
        assert!(read_version_with_timeout("ytdlp", &missing, Duration::from_secs(1)).is_empty());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn swap_in_keeps_previous_file_as_backup() {
        let dir = temp_dir();
        let dest = dir.join("yt-dlp.exe");
        let src = dir.join("yt-dlp.part");
        std::fs::write(&dest, b"old").unwrap();
        std::fs::write(&src, b"new").unwrap();

        let backup = swap_in(&src, &dest).unwrap().expect("backup");
        assert_eq!(std::fs::read(&dest).unwrap(), b"new");
        assert_eq!(std::fs::read(&backup).unwrap(), b"old");
        assert!(!src.exists());

        std::fs::remove_file(backup).unwrap();
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn invalid_binary_is_rejected_and_old_binary_survives() {
        let dir = temp_dir();
        let dest = dir.join("yt-dlp.exe");
        let src = dir.join("yt-dlp.part");
        std::fs::write(&dest, b"old").unwrap();
        std::fs::write(&src, b"not a program").unwrap();

        assert!(install_binary(&src, &dest, "ytdlp").is_err());
        assert_eq!(std::fs::read(&dest).unwrap(), b"old");
        assert!(!src.exists());
        let _ = std::fs::remove_dir_all(dir);
    }
}
