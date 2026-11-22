//! 应用配置模型。

use serde::{Deserialize, Serialize};

use super::LlmConfig;

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

/// 下载事件的细分开关；总开关仍是 `NotifyConfig::download`。
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase", default)]
pub struct NotifyEvents {
    pub success: bool,
    pub failed: bool,
    pub all_done: bool,
}

impl Default for NotifyEvents {
    fn default() -> Self {
        Self {
            success: true,
            failed: true,
            all_done: false,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct NotifyConfig {
    pub download: bool,
    pub task: bool,
    #[serde(default)]
    pub events: NotifyEvents,
    #[serde(default)]
    pub sound: bool,
}

impl Default for NotifyConfig {
    fn default() -> Self {
        Self {
            download: true,
            task: true,
            events: NotifyEvents::default(),
            sound: false,
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

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ShortcutsConfig {
    pub translate_submit: String,
    pub db_run: String,
}

impl Default for ShortcutsConfig {
    fn default() -> Self {
        Self {
            translate_submit: "CommandOrControl+Enter".into(),
            db_run: "CommandOrControl+Enter".into(),
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
    #[serde(default)]
    pub shortcuts: ShortcutsConfig,
    #[serde(default)]
    pub llm: LlmConfig,
}

/// 保存配置时版本不一致：携带磁盘上的最新配置，供前端二选一。
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ConfigConflictError {
    pub code: String,
    pub message: String,
    pub config: AppConfig,
    pub config_revision: u64,
}

impl ConfigConflictError {
    pub fn new(config: AppConfig, config_revision: u64) -> Self {
        Self {
            code: "CONFLICT".into(),
            message: "配置已被其他页面修改。请选择保留当前编辑，或改用磁盘上的最新配置。".into(),
            config,
            config_revision,
        }
    }
}

/// 配置保存错误：冲突返回结构化数据，其余保持原来的字符串错误。
#[derive(Serialize, Clone, Debug)]
#[serde(untagged)]
pub enum ConfigSaveError {
    Conflict(ConfigConflictError),
    Message(String),
}

impl From<String> for ConfigSaveError {
    fn from(value: String) -> Self {
        Self::Message(value)
    }
}
