//! 备份导入结果模型。

use serde::{Deserialize, Serialize};

/// 备份导入结果：前端据此展示恢复数量、待确认命令任务，并提示重启。
///
/// `backup_path` 为空表示用户在文件选择器中取消，前端应直接忽略。
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct ImportOutcome {
    pub restored_config: bool,
    pub restored_history: usize,
    pub restored_tasks: usize,
    pub restored_jobs: usize,
    pub disabled_command_tasks: usize,
    pub backup_path: String,
    pub restart_required: bool,
}

impl ImportOutcome {
    /// 用户取消选择文件：所有计数为 0，`backup_path` 为空。
    pub fn canceled() -> Self {
        Self::default()
    }
}
