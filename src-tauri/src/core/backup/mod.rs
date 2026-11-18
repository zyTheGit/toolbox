//! 备份导入 / 导出。
//!
//! 导出默认脱敏（命令、环境变量、输出、URL 凭据、Header Token），
//! 导入把备份当作不可信输入：关闭命令执行、禁用命令任务、清空敏感字段，
//! 并以“恢复前快照 + 原子写 + 逐文件回滚”的方式落盘。

mod restore;
mod sanitize;

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::models::{AppConfig, HistoryEntry, HistoryQuery, ImportOutcome, Job, Task};
use crate::state::{push_warning, AppState};

pub use sanitize::diagnostics_value;

pub const BUNDLE_SCHEMA_VERSION: u32 = 1;
/// 备份文件大小上限（20 MiB），超出直接拒绝，避免把整包读入内存。
pub const MAX_BUNDLE_BYTES: u64 = 20 * 1024 * 1024;
/// 历史条数上限。
pub const MAX_HISTORY_ENTRIES: usize = 5_000;
/// 任务数量上限（与调度器 MAX_TASKS 保持一致）。
pub const MAX_TASKS: usize = 50;
/// 下载记录数量上限。
pub const MAX_JOBS: usize = 500;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BackupBundle {
    pub schema_version: u32,
    pub app_version: String,
    pub exported_at: String,
    pub config: AppConfig,
    pub tasks: Vec<Task>,
    pub history: Vec<HistoryEntry>,
    pub jobs: Vec<Job>,
}

impl BackupBundle {
    fn new(
        config: AppConfig,
        tasks: Vec<Task>,
        history: Vec<HistoryEntry>,
        jobs: Vec<Job>,
        include_commands: bool,
    ) -> Self {
        sanitize::build_bundle(config, tasks, history, jobs, include_commands)
    }
}

/// 从运行期状态收集数据并脱敏。默认不包含命令与输出。
pub fn build(state: &AppState, include_commands: bool) -> BackupBundle {
    let history = state
        .history
        .query(&HistoryQuery {
            limit: u32::MAX,
            ..HistoryQuery::default()
        })
        .entries;
    BackupBundle::new(
        state.view_config(),
        state.scheduler.list(),
        history,
        state.downloader.list(),
        include_commands,
    )
}

pub fn export_to(path: &Path, state: &AppState, include_commands: bool) -> Result<String, String> {
    let bundle = build(state, include_commands);
    crate::core::persist::save_json(path, &bundle)?;
    Ok(path.to_string_lossy().to_string())
}

/// 导入备份：校验 → 脱敏 → 停写 → 快照 → 原子写 → 重载内存。
///
/// 成功后保持只读直到用户重启（`AppState.restoring` 不清除）；
/// 失败时逐文件回滚并恢复正常调度。
pub fn import_from(path: &Path, state: &AppState) -> Result<ImportOutcome, String> {
    let bundle = read_bundle(path)?;
    let (plan, disabled) = sanitize::sanitize_import(bundle, &state.paths)?;

    if state.downloader.active_count() > 0 || state.scheduler.active_count() > 0 {
        return Err("[CONFLICT] 有任务正在运行，请先停止后再恢复备份".into());
    }

    state.begin_restore()?;
    // 先停止 tick / worker，避免旧内存快照在恢复后写回磁盘。
    state.scheduler.shutdown();
    state.downloader.shutdown();

    let backup_dir = match restore::apply(&state.paths, &plan) {
        Ok(dir) => dir,
        Err(err) => {
            let _ = state.scheduler.reload_from_disk();
            let _ = state.downloader.reload_from_disk(false);
            state.scheduler.start();
            state.end_restore();
            return Err(err);
        }
    };

    // 导入后重载内存态；成功后保持停写，等待用户点击“重启应用”。
    let reloaded = state
        .scheduler
        .reload_from_disk()
        .and_then(|_| state.downloader.reload_from_disk(true));
    if let Err(err) = reloaded {
        push_warning(
            &state.warnings,
            format!("恢复完成但重载内存失败，请重启应用：{err}"),
        );
    }
    if let Err(err) = restore::write_restore_notice(&state.paths, disabled, &backup_dir) {
        push_warning(&state.warnings, format!("写入恢复提示失败：{err}"));
    }

    Ok(ImportOutcome {
        restored_config: true,
        restored_history: plan.history.len(),
        restored_tasks: plan.tasks.len(),
        restored_jobs: plan.jobs.len(),
        disabled_command_tasks: disabled,
        backup_path: backup_dir,
        restart_required: true,
    })
}

fn read_bundle(path: &Path) -> Result<BackupBundle, String> {
    let meta = fs::metadata(path).map_err(|err| format!("[DATA] 读取备份失败：{err}"))?;
    ensure_file_size(meta.len())?;
    let raw = fs::read(path).map_err(|err| format!("[DATA] 读取备份失败：{err}"))?;
    ensure_file_size(raw.len() as u64)?;

    let bundle: BackupBundle =
        serde_json::from_slice(&raw).map_err(|err| format!("[DATA] 备份内容无法解析：{err}"))?;
    if bundle.schema_version != BUNDLE_SCHEMA_VERSION {
        return Err(format!(
            "[DATA] 备份版本不兼容（文件 {}，当前支持 {}）",
            bundle.schema_version, BUNDLE_SCHEMA_VERSION
        ));
    }
    Ok(bundle)
}

fn ensure_file_size(len: u64) -> Result<(), String> {
    if len > MAX_BUNDLE_BYTES {
        return Err(format!(
            "[DATA] 备份文件过大（{len} 字节，上限 {MAX_BUNDLE_BYTES} 字节），已拒绝导入"
        ));
    }
    Ok(())
}

/// 条数上限校验：超出直接拒绝并给出中文原因。
pub(super) fn check_limits(bundle: &BackupBundle) -> Result<(), String> {
    if bundle.history.len() > MAX_HISTORY_ENTRIES {
        return Err(format!(
            "[DATA] 备份包含 {} 条翻译历史，超过上限 {MAX_HISTORY_ENTRIES}，已拒绝导入",
            bundle.history.len()
        ));
    }
    if bundle.tasks.len() > MAX_TASKS {
        return Err(format!(
            "[DATA] 备份包含 {} 个定时任务，超过上限 {MAX_TASKS}，已拒绝导入",
            bundle.tasks.len()
        ));
    }
    if bundle.jobs.len() > MAX_JOBS {
        return Err(format!(
            "[DATA] 备份包含 {} 条下载记录，超过上限 {MAX_JOBS}，已拒绝导入",
            bundle.jobs.len()
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::AppPaths;

    fn temp_paths() -> AppPaths {
        let dir = std::env::temp_dir().join(format!("toolbox-import-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        AppPaths::from_data_dir(dir)
    }

    fn empty_bundle() -> BackupBundle {
        BackupBundle {
            schema_version: BUNDLE_SCHEMA_VERSION,
            app_version: "0.1.0".into(),
            exported_at: String::new(),
            config: AppConfig::default(),
            tasks: Vec::new(),
            history: Vec::new(),
            jobs: Vec::new(),
        }
    }

    #[test]
    fn rejects_oversized_files() {
        assert!(ensure_file_size(MAX_BUNDLE_BYTES).is_ok());
        let err = ensure_file_size(MAX_BUNDLE_BYTES + 1).unwrap_err();
        assert!(err.contains("过大"));
    }

    #[test]
    fn rejects_too_many_entries() {
        let mut bundle = empty_bundle();
        bundle.tasks = vec![Task::default(); MAX_TASKS + 1];
        assert!(check_limits(&bundle).unwrap_err().contains("定时任务"));

        let mut bundle = empty_bundle();
        bundle.jobs = vec![Job::default(); MAX_JOBS + 1];
        assert!(check_limits(&bundle).unwrap_err().contains("下载记录"));

        let mut bundle = empty_bundle();
        bundle.history = vec![HistoryEntry::default(); MAX_HISTORY_ENTRIES + 1];
        assert!(check_limits(&bundle).unwrap_err().contains("翻译历史"));
    }

    #[test]
    fn rejects_unknown_bundle_version() {
        let mut bundle = empty_bundle();
        bundle.schema_version = BUNDLE_SCHEMA_VERSION + 1;
        let json = serde_json::to_vec(&bundle).unwrap();
        let parsed: BackupBundle = serde_json::from_slice(&json).unwrap();
        assert_ne!(parsed.schema_version, BUNDLE_SCHEMA_VERSION);
    }

    /// 端到端（不含 AppState）：恶意备份经脱敏后落盘，命令开关关闭、
    /// 命令任务禁用待确认，敏感字段 0 命中。
    #[test]
    fn canary_import_writes_disabled_tasks_and_no_secrets() {
        let paths = temp_paths();
        let (plan, disabled) =
            sanitize::sanitize_import(sanitize::tests::canary_bundle(), &paths).unwrap();
        assert_eq!(disabled, 1);
        restore::apply(&paths, &plan).unwrap();

        let config: AppConfig =
            crate::core::persist::load_json(&paths.config_file).unwrap().unwrap();
        assert!(!config.security.allow_commands);
        assert!(config.proxy.is_empty());
        assert!(config.translate.openai.api_key.is_empty());

        let tasks: Vec<Task> =
            crate::core::persist::load_json(&paths.tasks_file).unwrap().unwrap();
        assert_eq!(tasks.len(), 1);
        assert!(!tasks[0].enabled);
        assert_eq!(tasks[0].payload.command, sanitize::tests::CANARY_COMMAND);

        let config_text = std::fs::read_to_string(&paths.config_file).unwrap();
        let task_text = std::fs::read_to_string(&paths.tasks_file).unwrap();
        let job_text = std::fs::read_to_string(&paths.jobs_file).unwrap();
        for text in [&config_text, &task_text, &job_text] {
            for canary in [
                sanitize::tests::CANARY_ENV,
                sanitize::tests::CANARY_QUERY,
                sanitize::tests::CANARY_OUTPUT,
                sanitize::tests::CANARY_API_KEY,
                sanitize::tests::CANARY_COOKIE,
                sanitize::tests::CANARY_PROXY_PASS,
                sanitize::tests::CANARY_HEADER,
            ] {
                assert!(!text.contains(canary), "落盘数据仍含 {canary}");
            }
        }
    }
}
