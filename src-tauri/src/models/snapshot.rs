//! 启动状态与应用快照模型。

use serde::{Deserialize, Serialize};

use crate::core::secrets::SecretStatus;

use super::{
    config::AppConfig, download::{Job, ToolsStatus}, task::Task, translate::ProviderInfo,
};

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
    /// 配置版本号，用于保存前的期望版本校验（0 表示尚未启用）。
    #[serde(default)]
    pub config_revision: u64,
    /// 备份恢复后的首次提示（启动时读取一次后删除），提醒用户检查命令任务。
    #[serde(default)]
    pub restore_notice: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ConfigResult {
    pub config: AppConfig,
    pub secrets: SecretStatus,
    pub tools: ToolsStatus,
    pub providers: Vec<ProviderInfo>,
    /// 保存成功后的配置版本号（0 表示尚未启用）。
    #[serde(default)]
    pub config_revision: u64,
}
