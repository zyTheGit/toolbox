//! 网络下载：客户端构建、流式下载与进度事件。

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

pub(crate) fn with_mirror(url: &str, mirror: &str) -> String {
    let prefix = mirror.trim();
    if prefix.is_empty() {
        return url.to_string();
    }
    if prefix.ends_with('/') {
        format!("{prefix}{url}")
    } else {
        format!("{prefix}/{url}")
    }
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

pub(crate) async fn download_file(
    client: &Client,
    url: &str,
    dest: &Path,
    app: &AppHandle,
    tool: &str,
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
    let mut file = tokio::fs::File::create(dest)
        .await
        .map_err(|e| format!("[PATH] 创建临时文件失败: {e}"))?;
    let mut stream = response.bytes_stream();
    let mut received: u64 = 0;
    let mut last_at = Instant::now();
    let mut last_bytes: u64 = 0;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("[NETWORK] 下载中断: {e}"))?;
        file.write_all(&chunk)
            .await
            .map_err(|e| format!("[PATH] 写入临时文件失败: {e}"))?;
        received += chunk.len() as u64;

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
