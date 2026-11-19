//! 工具与下载任务模型。

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct ToolStatus {
    pub ok: bool,
    pub path: String,
    pub version: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct ToolsStatus {
    pub ytdlp: ToolStatus,
    pub ffmpeg: ToolStatus,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct ToolInstallResult {
    pub tool: String,
    pub path: String,
    pub version: String,
    pub tools: ToolsStatus,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct ToolProgress {
    pub tool: String,
    pub phase: String,
    pub pct: Option<f64>,
    pub received: u64,
    pub total: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct JobProgress {
    pub percent: f64,
    pub speed: String,
    pub eta: String,
    pub downloaded: String,
    pub total: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Job {
    pub id: String,
    pub url: String,
    pub quality: String,
    pub out_dir: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub threads: u32,
    #[serde(default)]
    pub referer: String,
    #[serde(default)]
    pub cookies_browser: String,
    #[serde(default)]
    pub proxy: String,
    #[serde(default)]
    pub extra_args: String,
    /// 字幕档位：空表示不下载，zh / en / both 对应中 / 英 / 中英。
    #[serde(default)]
    pub subtitles: String,
    /// 仅下载字幕，不下载视频/音频（yt-dlp --skip-download）。
    #[serde(default)]
    pub subtitle_only: bool,
    pub status: String,
    pub progress: JobProgress,
    pub log: Vec<String>,
    pub error: String,
    pub file: String,
    pub created_at: String,
    pub started_at: String,
    pub finished_at: String,
    /// 本次任务是否在内存中持有下载凭据（不落盘，默认 false）。
    #[serde(default)]
    pub has_credentials: bool,
    /// 非致命提示，例如站点字幕语言与内容不符。
    #[serde(default)]
    pub notice: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase", default)]
pub struct DownloadRequest {
    pub urls: Vec<String>,
    pub quality: String,
    pub out_dir: Option<String>,
    pub file_name: Option<String>,
    pub threads: Option<u32>,
    pub referer: String,
    pub cookies_browser: String,
    pub proxy: String,
    pub extra_args: String,
    pub subtitles: String,
    /// 仅下载字幕，不下载视频/音频。
    pub subtitle_only: bool,
}

impl Default for DownloadRequest {
    fn default() -> Self {
        Self {
            urls: Vec::new(),
            quality: "best".into(),
            out_dir: None,
            file_name: None,
            threads: None,
            referer: String::new(),
            cookies_browser: String::new(),
            proxy: String::new(),
            extra_args: String::new(),
            subtitles: String::new(),
            subtitle_only: false,
        }
    }
}

/// 重试时补填的下载参数（字段全部可选，缺省表示保持原值）。
///
/// 凭据只进入进程内凭据表，不写回 `jobs.json`。
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct JobRetryRequest {
    pub referer: Option<String>,
    pub cookies_browser: Option<String>,
    pub proxy: Option<String>,
    pub extra_args: Option<String>,
    pub quality: Option<String>,
    pub threads: Option<u32>,
}
