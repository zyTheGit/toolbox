//! 数据模型：按领域拆分，统一从 `models::` 重导出，调用方引用路径不变。

mod config;
mod download;
mod snapshot;
mod task;
mod translate;

pub use config::{
    AppConfig, BaiduConfig, DeepLConfig, HistoryConfig, NotifyConfig, OpenAiConfig,
    QuickTranslateConfig, SecurityConfig, StartupConfig, ToolPaths, TranslateConfig,
    YoudaoConfig, CONFIG_SCHEMA_VERSION,
};
pub use download::{
    DownloadRequest, Job, JobProgress, ToolInstallResult, ToolProgress, ToolStatus, ToolsStatus,
};
pub use snapshot::{AppSnapshot, AutostartStatus, ConfigResult};
pub use task::{Task, TaskInput, TaskPayload, TaskRun};
pub use translate::{HistoryEntry, HistoryPage, HistoryQuery, ProviderInfo, TranslateRequest, TranslateResult};
