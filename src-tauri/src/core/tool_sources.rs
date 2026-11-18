// Windows 构建下本模块不被调用（tools.rs 按平台分发），豁免未使用告警。
#![cfg_attr(windows, allow(dead_code))]

//! 非 Windows 平台的 ffmpeg 安装源。
//!
//! macOS/Linux 使用 GitHub `eugeneware/ffmpeg-static` 静态构建：
//! 通过 Release API 获取目标资产的下载地址与官方 SHA256（asset digest），
//! 下载后做流式哈希校验。API 不可达、资产或 digest 缺失、格式异常、
//! 哈希不匹配均视为 fail-closed：终止安装、清理临时文件、保留旧版本。

use reqwest::Client;
use tauri::AppHandle;

use crate::core::tools::{binary_url, download_file, exe_file, install_binary, verify_sha256, MAX_BINARY_BYTES};
use crate::models::AppConfig;
use crate::state::AppPaths;

const RELEASE_API: &str = "https://api.github.com/repos/eugeneware/ffmpeg-static/releases/latest";
const DIGEST_PREFIX: &str = "sha256:";

/// 当前平台对应的 ffmpeg-static 资产名。
fn ffmpeg_asset() -> Result<&'static str, String> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => Ok("ffmpeg-darwin-arm64"),
        ("macos", "x86_64") => Ok("ffmpeg-darwin-x64"),
        ("linux", "x86_64") => Ok("ffmpeg-linux-x64"),
        ("linux", "aarch64") => Ok("ffmpeg-linux-arm64"),
        _ => Err("[TOOL] 当前平台暂不支持自动安装 ffmpeg，请自行安装".into()),
    }
}

/// 调用 GitHub Release API，返回目标资产的下载地址与官方 SHA256。
async fn resolve_asset(client: &Client, name: &str) -> Result<(String, String), String> {
    let response = client
        .get(RELEASE_API)
        .header(reqwest::header::ACCEPT, "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| format!("[NETWORK] 获取 ffmpeg 发布信息失败: {e}"))?;
    if !response.status().is_success() {
        return Err(format!(
            "[NETWORK] 获取 ffmpeg 发布信息失败: HTTP {}",
            response.status()
        ));
    }
    let payload: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("[NETWORK] 解析 ffmpeg 发布信息失败: {e}"))?;
    let assets = payload["assets"]
        .as_array()
        .ok_or_else(|| "[TOOL] 官方发布信息缺少资产列表，已放弃安装".to_string())?;
    for asset in assets {
        if asset["name"].as_str() != Some(name) {
            continue;
        }
        let url = asset["browser_download_url"]
            .as_str()
            .ok_or_else(|| "[TOOL] 官方资产缺少下载地址，已放弃安装".to_string())?;
        let digest = asset["digest"]
            .as_str()
            .and_then(|value| value.strip_prefix(DIGEST_PREFIX))
            .map(str::trim)
            .filter(|value| value.len() == 64 && value.chars().all(|c| c.is_ascii_hexdigit()))
            .ok_or_else(|| "[TOOL] 官方资产缺少有效 SHA256，已放弃安装".to_string())?;
        return Ok((url.to_string(), digest.to_ascii_lowercase()));
    }
    Err(format!("[TOOL] 官方发布中未找到 {name} 资产，已放弃安装"))
}

/// 下载并安装 ffmpeg 静态二进制（由非 Windows 平台的安装流程调用）。
pub(crate) async fn install_ffmpeg_static(
    client: &Client,
    app: &AppHandle,
    config: &AppConfig,
    paths: &AppPaths,
) -> Result<(), String> {
    let name = ffmpeg_asset()?;
    let (download_url, expected) = resolve_asset(client, name).await?;
    // 二进制可用镜像（必须 https）；SHA256 来自官方 Release API，不受镜像影响。
    let url = binary_url(&download_url, &config.tool_download_mirror)?;
    let dest = paths.bin_dir.join(exe_file("ffmpeg"));
    let part = paths
        .bin_dir
        .join(format!(".tmp-{name}-{}", uuid::Uuid::new_v4()));
    let _ = std::fs::remove_file(&part);

    let outcome = async {
        download_file(client, &url, &part, app, "ffmpeg", MAX_BINARY_BYTES).await?;
        verify_sha256(&part, &expected, "ffmpeg")?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&part, std::fs::Permissions::from_mode(0o755))
                .map_err(|e| format!("设置可执行权限失败: {e}"))?;
        }

        install_binary(&part, &dest, "ffmpeg")
    }
    .await;

    // fail-closed：任何环节失败都清理临时文件，保留旧版本。
    if outcome.is_err() {
        let _ = std::fs::remove_file(&part);
    }
    outcome
}
