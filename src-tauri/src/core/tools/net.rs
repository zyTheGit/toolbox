//! 网络下载：客户端构建、镜像选择、流式下载与进度事件。
//!
//! 镜像只影响二进制下载地址，且必须是 https；SHA256 校验文件始终走官方
//! origin（见 `checksum`），避免镜像同时伪造二进制与哈希。

use std::path::Path;
use std::time::{Duration, Instant};

use futures_util::StreamExt;
use reqwest::Client;
use tauri::{AppHandle, Emitter};
use tokio::io::AsyncWriteExt;

use super::UA;
use crate::models::{AppConfig, ToolProgress};

const EMIT_INTERVAL: Duration = Duration::from_millis(300);
const EMIT_BYTES: u64 = 1024 * 1024;

/// 工具二进制大小上限：yt-dlp / ffmpeg 官方产物均远小于此值。
pub(crate) const MAX_BINARY_BYTES: u64 = 200 * 1024 * 1024;
/// 官方校验文件大小上限，防止异常响应拖垮安装流程。
pub(crate) const MAX_CHECKSUM_BYTES: u64 = 2 * 1024 * 1024;

/// 二进制下载地址：允许配置镜像，但镜像必须是 https。
/// http 镜像直接拒绝，绝不回退到“跳过校验”的安装。
pub(crate) fn binary_url(official: &str, mirror: &str) -> Result<String, String> {
    let prefix = mirror.trim();
    if prefix.is_empty() {
        return Ok(official.to_string());
    }
    if !prefix.to_ascii_lowercase().starts_with("https://") {
        return Err(
            "[PARAM] 工具下载镜像必须使用 https:// 地址，已拒绝本次安装；可清空镜像改用官方源"
                .into(),
        );
    }
    if prefix.ends_with('/') {
        Ok(format!("{prefix}{official}"))
    } else {
        Ok(format!("{prefix}/{official}"))
    }
}

/// 大小上限判定（纯函数，便于单测）。
pub(crate) fn exceeds_limit(received: u64, max: u64) -> bool {
    received > max
}

pub(crate) fn build_client(config: &AppConfig) -> Result<Client, String> {
    let mut builder = Client::builder()
        .user_agent(UA)
        .connect_timeout(Duration::from_secs(30))
        .timeout(Duration::from_secs(15 * 60));

    let proxy = config.proxy.trim();
    if proxy.is_empty() {
        builder = builder.no_proxy();
    } else {
        builder = builder.proxy(
            reqwest::Proxy::all(proxy).map_err(|e| format!("[PARAM] 代理地址无效: {e}"))?,
        );
    }

    builder
        .build()
        .map_err(|e| format!("[NETWORK] 初始化网络客户端失败: {e}"))
}

/// 流式下载到 `dest`，超过 `max_bytes` 立即中止并删除临时文件。
pub(crate) async fn download_file(
    client: &Client,
    url: &str,
    dest: &Path,
    app: &AppHandle,
    tool: &str,
    max_bytes: u64,
) -> Result<u64, String> {
    let response = client
        .get(url)
        .header(reqwest::header::USER_AGENT, UA)
        .send()
        .await
        .map_err(|e| format!("[NETWORK] 下载失败: {e}"))?;
    if !response.status().is_success() {
        return Err(format!("[NETWORK] 下载失败: HTTP {}", response.status()));
    }

    let total = response.content_length().unwrap_or(0);
    if exceeds_limit(total, max_bytes) {
        return Err(limit_error(max_bytes));
    }

    let mut file = tokio::fs::File::create(dest)
        .await
        .map_err(|e| format!("[PATH] 创建临时文件失败: {e}"))?;
    let mut stream = response.bytes_stream();
    let mut received: u64 = 0;
    let mut last_at = Instant::now();
    let mut last_bytes: u64 = 0;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("[NETWORK] 下载中断: {e}"))?;
        received += chunk.len() as u64;
        if exceeds_limit(received, max_bytes) {
            drop(file);
            let _ = tokio::fs::remove_file(dest).await;
            return Err(limit_error(max_bytes));
        }
        file.write_all(&chunk)
            .await
            .map_err(|e| format!("[PATH] 写入临时文件失败: {e}"))?;

        if last_at.elapsed() >= EMIT_INTERVAL
            || received.saturating_sub(last_bytes) >= EMIT_BYTES
        {
            last_at = Instant::now();
            last_bytes = received;
            emit_progress(app, tool, "download", percent(received, total), received, total);
        }
    }

    file.flush()
        .await
        .map_err(|e| format!("写入临时文件失败: {e}"))?;
    let _ = file.sync_all().await;
    emit_progress(app, tool, "download", percent(received, total), received, total);
    Ok(received)
}

fn limit_error(max_bytes: u64) -> String {
    format!(
        "[TOOL] 下载内容超过 {} MB 上限，已中止并删除临时文件（旧版本保持不变）",
        max_bytes / (1024 * 1024)
    )
}

fn percent(received: u64, total: u64) -> Option<f64> {
    if total == 0 {
        None
    } else {
        Some((received as f64 / total as f64 * 100.0).min(100.0))
    }
}

pub(crate) fn emit_progress(
    app: &AppHandle,
    tool: &str,
    phase: &str,
    pct: Option<f64>,
    received: u64,
    total: u64,
) {
    let _ = app.emit(
        "tool-progress",
        ToolProgress {
            tool: tool.to_string(),
            phase: phase.to_string(),
            pct,
            received,
            total,
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    const OFFICIAL: &str = "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe";

    #[test]
    fn binary_url_allows_https_mirror_only() {
        assert_eq!(binary_url(OFFICIAL, "").unwrap(), OFFICIAL);
        assert_eq!(
            binary_url(OFFICIAL, "https://mirror.example/").unwrap(),
            format!("https://mirror.example/{OFFICIAL}")
        );
        assert_eq!(
            binary_url(OFFICIAL, "https://mirror.example").unwrap(),
            format!("https://mirror.example/{OFFICIAL}")
        );

        for mirror in ["http://mirror.example/", "ftp://mirror.example/", "mirror.example/"] {
            let err = binary_url(OFFICIAL, mirror).unwrap_err();
            assert!(err.starts_with("[PARAM]"));
            assert!(err.contains("https"), "http 镜像必须给出明确中文错误: {err}");
        }
    }

    #[test]
    fn size_limit_is_inclusive_and_blocks_overflow() {
        assert!(!exceeds_limit(0, MAX_BINARY_BYTES));
        assert!(!exceeds_limit(MAX_BINARY_BYTES, MAX_BINARY_BYTES));
        assert!(exceeds_limit(MAX_BINARY_BYTES + 1, MAX_BINARY_BYTES));
        assert!(exceeds_limit(MAX_CHECKSUM_BYTES + 1, MAX_CHECKSUM_BYTES));
        assert!(limit_error(MAX_BINARY_BYTES).contains("200 MB"));
    }
}
