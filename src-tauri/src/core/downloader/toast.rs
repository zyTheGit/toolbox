//! 下载事件的应用内提示：事件载荷与通知文案。

use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::models::Job;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DownloadToast {
    kind: &'static str,
    title: &'static str,
    text: String,
}

/// 应用内提示事件：窗口可见时由前端弹出消息并按设置播放提示音。
pub(super) fn emit_toast(app: &AppHandle, kind: &'static str, title: &'static str, text: &str) {
    let _ = app.emit(
        "download-notify",
        DownloadToast {
            kind,
            title,
            text: text.to_string(),
        },
    );
}

pub(super) fn job_label(job: &Job) -> String {
    let label = if job.name.trim().is_empty() {
        std::path::Path::new(&job.file)
            .file_name()
            .map(|value| value.to_string_lossy().to_string())
            .unwrap_or_default()
    } else {
        job.name.trim().to_string()
    };
    let label: String = label.chars().take(60).collect();
    if label.is_empty() {
        "m3u8 任务".to_string()
    } else {
        label
    }
}

/// 失败通知只带最关键的一行原因：去掉内部错误标签，超长截断。
pub(super) fn failure_hint(error: &str) -> String {
    let line = error
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("原因见任务日志");
    let text = match line.strip_prefix('[').and_then(|rest| rest.split_once("] ")) {
        Some((_, rest)) => rest,
        None => line,
    };
    let text: String = text.chars().take(80).collect();
    if text.is_empty() {
        "原因见任务日志".to_string()
    } else {
        text
    }
}
