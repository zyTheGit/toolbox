use serde::{Deserialize, Serialize};

use crate::core::secrets::SecretStatus;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct ToolPaths {
    pub ytdlp: String,
    pub ffmpeg: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct StartupConfig {
    pub start_minimized: bool,
    pub delay_seconds: u32,
}

impl Default for StartupConfig {
    fn default() -> Self {
        Self {
            start_minimized: true,
            delay_seconds: 10,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct OpenAiConfig {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
}

impl Default for OpenAiConfig {
    fn default() -> Self {
        Self {
            base_url: "https://api.openai.com/v1".into(),
            api_key: String::new(),
            model: "gpt-4o-mini".into(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DeepLConfig {
    pub api_key: String,
    pub endpoint: String,
}

impl Default for DeepLConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            endpoint: "https://api-free.deepl.com/v2/translate".into(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct BaiduConfig {
    pub app_id: String,
    pub secret_key: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct YoudaoConfig {
    pub app_key: String,
    pub app_secret: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TranslateConfig {
    pub provider: String,
    pub fallback: Vec<String>,
    #[serde(default = "default_tts_rate")]
    pub tts_rate: f64,
    pub openai: OpenAiConfig,
    pub deepl: DeepLConfig,
    pub baidu: BaiduConfig,
    pub youdao: YoudaoConfig,
}

fn default_tts_rate() -> f64 {
    1.0
}

impl Default for TranslateConfig {
    fn default() -> Self {
        Self {
            provider: "microsoft".into(),
            fallback: vec!["google".into(), "openai".into()],
            tts_rate: default_tts_rate(),
            openai: OpenAiConfig::default(),
            deepl: DeepLConfig::default(),
            baidu: BaiduConfig::default(),
            youdao: YoudaoConfig::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct HistoryConfig {
    pub enabled: bool,
    pub max_entries: u32,
}

impl Default for HistoryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_entries: 200,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct SecurityConfig {
    pub allow_commands: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct NotifyConfig {
    pub download: bool,
    pub task: bool,
}

impl Default for NotifyConfig {
    fn default() -> Self {
        Self {
            download: true,
            task: true,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct QuickTranslateConfig {
    pub enabled: bool,
    pub shortcut: String,
}

impl Default for QuickTranslateConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            shortcut: "CommandOrControl+Shift+T".into(),
        }
    }
}

pub const CONFIG_SCHEMA_VERSION: u32 = 1;

fn default_schema_version() -> u32 {
    CONFIG_SCHEMA_VERSION
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AppConfig {
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    pub download_dir: String,
    #[serde(default)]
    pub download_concurrency: u32,
    #[serde(default)]
    pub fragment_threads: u32,
    pub proxy: String,
    pub tool_download_mirror: String,
    pub tool_paths: ToolPaths,
    pub startup: StartupConfig,
    pub translate: TranslateConfig,
    pub history: HistoryConfig,
    pub security: SecurityConfig,
    #[serde(default)]
    pub notify: NotifyConfig,
    #[serde(default)]
    pub quick_translate: QuickTranslateConfig,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ProviderInfo {
    pub id: String,
    pub name: String,
    pub requires_key: bool,
    pub ready: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TranslateRequest {
    pub text: String,
    pub direction: String,
    pub provider: Option<String>,
}

impl Default for TranslateRequest {
    fn default() -> Self {
        Self {
            text: String::new(),
            direction: "auto".into(),
            provider: None,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct TranslateResult {
    pub text: String,
    pub provider: String,
    pub provider_name: String,
    pub source: String,
    pub target: String,
    pub latency_ms: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub id: String,
    pub text: String,
    pub result: String,
    pub source: String,
    pub target: String,
    pub direction: String,
    pub provider: String,
    pub provider_name: String,
    pub favorite: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase", default)]
pub struct HistoryQuery {
    pub keyword: Option<String>,
    pub direction: Option<String>,
    pub provider: Option<String>,
    pub favorites_only: bool,
    pub limit: u32,
    pub offset: u32,
}

impl Default for HistoryQuery {
    fn default() -> Self {
        Self {
            keyword: None,
            direction: None,
            provider: None,
            favorites_only: false,
            limit: 200,
            offset: 0,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct HistoryPage {
    pub entries: Vec<HistoryEntry>,
    pub total: u32,
}

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
    pub status: String,
    pub progress: JobProgress,
    pub log: Vec<String>,
    pub error: String,
    pub file: String,
    pub created_at: String,
    pub started_at: String,
    pub finished_at: String,
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
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TaskPayload {
    pub url: String,
    pub quality: String,
    pub text: String,
    pub direction: String,
    pub save_file: String,
    pub command: String,
    pub work_dir: String,
    pub shell: String,
    pub timeout_seconds: u64,
    pub env: String,
    pub message: String,
}

impl Default for TaskPayload {
    fn default() -> Self {
        Self {
            url: String::new(),
            quality: "best".into(),
            text: String::new(),
            direction: "auto".into(),
            save_file: String::new(),
            command: String::new(),
            work_dir: String::new(),
            shell: "cmd".into(),
            timeout_seconds: 600,
            env: String::new(),
            message: String::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub task_type: String,
    pub cron: String,
    #[serde(default = "default_schedule")]
    pub schedule: String,
    #[serde(default)]
    pub run_at: String,
    pub enabled: bool,
    pub payload: TaskPayload,
    pub created_at: String,
    pub updated_at: String,
    pub last_run_at: String,
    pub last_status: String,
    pub last_output: String,
    pub last_error: String,
    pub last_job_id: String,
    pub running: bool,
    pub next_run: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TaskInput {
    pub name: String,
    #[serde(rename = "type")]
    pub task_type: String,
    pub cron: String,
    #[serde(default = "default_schedule")]
    pub schedule: String,
    #[serde(default)]
    pub run_at: String,
    pub enabled: bool,
    pub payload: TaskPayload,
}

impl Default for TaskInput {
    fn default() -> Self {
        Self {
            name: String::new(),
            task_type: "command".into(),
            cron: "0 3 * * *".into(),
            schedule: default_schedule(),
            run_at: String::new(),
            enabled: true,
            payload: TaskPayload::default(),
        }
    }
}

fn default_schedule() -> String {
    "cron".into()
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct TaskRun {
    pub id: String,
    pub task_id: String,
    pub trigger: String,
    pub status: String,
    pub started_at: String,
    pub finished_at: String,
    pub duration_ms: u64,
    pub exit_code: Option<i32>,
    pub output: String,
    pub error: String,
    pub job_id: String,
    pub command: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct AutostartStatus {
    pub supported: bool,
    pub enabled: bool,
    pub path: String,
    pub expected_path: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AppSnapshot {
    pub version: String,
    pub platform: String,
    pub config: AppConfig,
    pub secrets: SecretStatus,
    pub warnings: Vec<String>,
    pub tools: ToolsStatus,
    pub tasks: Vec<Task>,
    pub jobs: Vec<Job>,
    pub providers: Vec<ProviderInfo>,
    pub autostart: AutostartStatus,
    pub tasks_paused: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ConfigResult {
    pub config: AppConfig,
    pub secrets: SecretStatus,
    pub tools: ToolsStatus,
    pub providers: Vec<ProviderInfo>,
}
