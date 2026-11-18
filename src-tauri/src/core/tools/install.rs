//! 工具安装流程：yt-dlp 直接下载，ffmpeg 按平台分发。

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use reqwest::Client;
use tauri::AppHandle;

use super::checksum::{verify_ytdlp_checksum, ytdlp_url};
use super::net::{binary_url, download_file, emit_progress, MAX_BINARY_BYTES};
use super::{
    exe_file, install_binary, read_version_with_timeout, swap_in,
};
use crate::models::AppConfig;
use crate::state::AppPaths;

pub(crate) async fn install_ytdlp(
    client: &Client,
    app: &AppHandle,
    config: &AppConfig,
    paths: &AppPaths,
) -> Result<(), String> {
    // 二进制可用镜像；校验文件固定官方 origin（见 checksum）。
    let url = binary_url(&ytdlp_url(), &config.tool_download_mirror)?;
    let dest = paths.bin_dir.join(exe_file("ytdlp"));
    let part = paths.bin_dir.join(format!("{}.part", exe_file("ytdlp")));
    let _ = std::fs::remove_file(&part);

    if let Err(err) = download_file(client, &url, &part, app, "ytdlp", MAX_BINARY_BYTES).await {
        let _ = std::fs::remove_file(&part);
        return Err(err);
    }

    if let Err(err) = verify_ytdlp_checksum(client, &part).await {
        let _ = std::fs::remove_file(&part);
        return Err(err);
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&part, std::fs::Permissions::from_mode(0o755))
            .map_err(|e| format!("设置可执行权限失败: {e}"))?;
    }

    install_binary(&part, &dest, "ytdlp")
}

pub(crate) async fn install_ffmpeg(
    client: &Client,
    app: &AppHandle,
    config: &AppConfig,
    paths: &AppPaths,
) -> Result<(), String> {
    #[cfg(not(windows))]
    {
        crate::core::tool_sources::install_ffmpeg_static(client, app, config, paths).await
    }
    #[cfg(windows)]
    {
        use super::checksum::{ffmpeg_url, verify_ffmpeg_checksum};

        let url = binary_url(ffmpeg_url(), &config.tool_download_mirror)?;
        let temp_dir = paths
            .bin_dir
            .join(format!(".tmp-ffmpeg-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).map_err(|e| format!("创建临时目录失败: {e}"))?;
        let zip = temp_dir.join("ffmpeg.zip");

        let outcome = async {
            download_file(client, &url, &zip, app, "ffmpeg", MAX_BINARY_BYTES).await?;
            verify_ffmpeg_checksum(client, &zip).await?;
            emit_progress(app, "ffmpeg", "extract", Some(0.0), 0, 0);
            expand_archive(&zip, &temp_dir).await?;

            let ffmpeg_src = find_file(&temp_dir, "ffmpeg.exe", 6)
                .ok_or_else(|| "压缩包中未找到 ffmpeg.exe".to_string())?;
            if read_version_with_timeout("ffmpeg", &ffmpeg_src, Duration::from_secs(10)).is_empty() {
                return Err("[TOOL] ffmpeg.exe 无法运行，已放弃安装（旧版本保持不变）".into());
            }

            if let Some(ffprobe_src) = find_file(&temp_dir, "ffprobe.exe", 6) {
                let probe_dest = paths.bin_dir.join("ffprobe.exe");
                match swap_in(&ffprobe_src, &probe_dest) {
                    Ok(backup) => {
                        if let Some(backup) = backup {
                            let _ = std::fs::remove_file(backup);
                        }
                    }
                    Err(err) => return Err(format!("[PATH] 保存 ffprobe.exe 失败: {err}")),
                }
            }

            let dest = paths.bin_dir.join("ffmpeg.exe");
            install_binary(&ffmpeg_src, &dest, "ffmpeg")?;

            emit_progress(app, "ffmpeg", "extract", Some(100.0), 0, 0);
            Ok(())
        }
        .await;

        let _ = std::fs::remove_dir_all(&temp_dir);
        outcome
    }
}

async fn expand_archive(zip: &Path, dest: &Path) -> Result<(), String> {
    let script = format!(
        "Expand-Archive -LiteralPath '{}' -DestinationPath '{}' -Force",
        zip.to_string_lossy().replace('\'', "''"),
        dest.to_string_lossy().replace('\'', "''")
    );

    let mut command = tokio::process::Command::new("powershell.exe");
    command
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        command.creation_flags(0x0800_0000);
    }

    let output = command
        .output()
        .await
        .map_err(|e| format!("解压失败: {e}"))?;
    if !output.status.success() {
        let detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if detail.is_empty() {
            format!("解压失败: 退出码 {:?}", output.status.code())
        } else {
            format!("解压失败: {detail}")
        });
    }
    Ok(())
}

fn find_file(dir: &Path, name: &str, depth: usize) -> Option<PathBuf> {
    if depth == 0 {
        return None;
    }
    let entries = std::fs::read_dir(dir).ok()?;
    let mut dirs = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            if path
                .file_name()
                .and_then(|value| value.to_str())
                .is_some_and(|file| file.eq_ignore_ascii_case(name))
            {
                return Some(path);
            }
        } else if path.is_dir() {
            dirs.push(path);
        }
    }
    dirs.into_iter()
        .find_map(|sub| find_file(&sub, name, depth - 1))
}
