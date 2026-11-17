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
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ConfigResult {
    pub config: AppConfig,
    pub secrets: SecretStatus,
    pub tools: ToolsStatus,
    pub providers: Vec<ProviderInfo>,
}
