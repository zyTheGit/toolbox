//! 备份恢复的落盘事务：恢复前快照、原子写、逐文件回滚。

use std::path::PathBuf;

use crate::core::persist;
use crate::models::{AppConfig, HistoryEntry, Job, Task};
use crate::state::AppPaths;

/// 已脱敏、可直接落盘的恢复数据。
pub(super) struct RestorePlan {
    pub(super) config: AppConfig,
    pub(super) history: Vec<HistoryEntry>,
    pub(super) tasks: Vec<Task>,
    pub(super) jobs: Vec<Job>,
}

const SNAPSHOT_FILES: [&str; 4] = ["config.json", "history.json", "jobs.json", "tasks.json"];

/// 执行恢复：先把现有数据复制到 `restore-backup-<timestamp>/`，再原子写入
/// 四个数据文件；任一文件失败时逐文件回滚，保证原数据字节不变。
pub(super) fn apply(paths: &AppPaths, plan: &RestorePlan) -> Result<String, String> {
    let targets = vec![
        (
            paths.config_file.clone(),
            serialize("配置", &plan.config)?,
        ),
        (
            paths.history_file.clone(),
            serialize("历史", &plan.history)?,
        ),
        (paths.jobs_file.clone(), serialize("下载记录", &plan.jobs)?),
        (paths.tasks_file.clone(), serialize("任务", &plan.tasks)?),
    ];

    let snapshot_dir = paths.data_dir.join(format!(
        "restore-backup-{}",
        chrono::Local::now().format("%Y%m%d-%H%M%S")
    ));
    snapshot_current(paths, &snapshot_dir)?;
    write_targets(&targets)?;
    Ok(snapshot_dir.to_string_lossy().to_string())
}

fn serialize<T: serde::Serialize + ?Sized>(label: &str, value: &T) -> Result<Vec<u8>, String> {
    serde_json::to_vec_pretty(value)
        .map_err(|err| format!("[DATA] {label}序列化失败：{err}"))
}

fn snapshot_current(paths: &AppPaths, dir: &std::path::Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|err| {
        format!(
            "[DATA] 创建恢复前快照目录失败：{} {err}",
            dir.display()
        )
    })?;
    let sources = [
        &paths.config_file,
        &paths.history_file,
        &paths.jobs_file,
        &paths.tasks_file,
    ];
    for (source, name) in sources.iter().zip(SNAPSHOT_FILES.iter()) {
        let Some(bytes) = persist::load_bytes(source)? else {
            continue;
        };
        persist::write_bytes(&dir.join(name), &bytes).map_err(|err| {
            format!("[DATA] 备份现有数据失败（未做任何修改）：{err}")
        })?;
    }
    Ok(())
}

/// 按顺序原子写入；失败时把已写入的文件恢复为原始字节（原本不存在的文件删除）。
fn write_targets(targets: &[(PathBuf, Vec<u8>)]) -> Result<(), String> {
    let originals: Vec<Option<Vec<u8>>> = targets
        .iter()
        .map(|(path, _)| persist::load_bytes(path).ok().flatten())
        .collect();

    let mut written: Vec<usize> = Vec::new();
    for (index, (path, bytes)) in targets.iter().enumerate() {
        match persist::write_bytes(path, bytes) {
            Ok(()) => written.push(index),
            Err(err) => {
                for previous in written.iter().rev() {
                    let rollback_path = &targets[*previous].0;
                    match &originals[*previous] {
                        Some(bytes) => {
                            let _ = persist::write_bytes(rollback_path, bytes);
                        }
                        None => {
                            let _ = std::fs::remove_file(rollback_path);
                        }
                    }
                }
                let name = path
                    .file_name()
                    .map(|value| value.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.to_string_lossy().to_string());
                return Err(format!(
                    "[DATA] 写入 {name} 失败，已回滚全部文件：{err}"
                ));
            }
        }
    }
    Ok(())
}

/// 导入完成后写入一次性提示文件，启动后首次快照读取并展示。
pub(super) fn write_restore_notice(paths: &AppPaths, disabled: usize, backup_dir: &str) -> Result<(), String> {
    let detail = if disabled > 0 {
        format!("已关闭外部命令执行，{disabled} 个命令任务待确认。")
    } else {
        "已关闭外部命令执行。".to_string()
    };
    let text = format!(
        "备份已恢复：{detail}请到「定时任务」核对命令内容后再手动启用。恢复前数据快照：{backup_dir}"
    );
    persist::write_bytes(&paths.restore_notice_file, text.as_bytes())
}

/// 供测试使用的默认配置（与运行期一致地做一次规整）。
#[cfg(test)]
pub(super) fn normalized_config(paths: &AppPaths) -> AppConfig {
    let mut config = AppConfig::default();
    crate::core::config::normalize(&mut config, paths);
    config
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::TaskPayload;

    fn temp_paths() -> AppPaths {
        let dir = std::env::temp_dir().join(format!(
            "toolbox-restore-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        AppPaths::from_data_dir(dir)
    }

    fn sample_plan(paths: &AppPaths) -> RestorePlan {
        RestorePlan {
            config: normalized_config(paths),
            history: Vec::new(),
            tasks: vec![Task {
                id: "restored".into(),
                name: "恢复的任务".into(),
                task_type: "command".into(),
                enabled: false,
                payload: TaskPayload {
                    command: "echo restored".into(),
                    ..TaskPayload::default()
                },
                ..Task::default()
            }],
            jobs: Vec::new(),
        }
    }

    #[test]
    fn apply_snapshots_originals_and_writes_new_data() {
        let paths = temp_paths();
        std::fs::write(&paths.config_file, b"{\"old\":true}").unwrap();
        std::fs::write(&paths.tasks_file, b"[]").unwrap();

        let backup_dir = apply(&paths, &sample_plan(&paths)).unwrap();

        let config_text = std::fs::read_to_string(&paths.config_file).unwrap();
        // 旧测试用 `!contains("old")` 会误伤合法字段（如 toolDownloadMirror 的小写形式），
        // 这里只拒绝原始占位内容，并确认已写入新配置。
        assert!(
            !config_text.contains("\"old\":true") && !config_text.contains("\"old\": true"),
            "旧配置未被替换：{config_text}"
        );
        assert!(config_text.contains("\"schemaVersion\""));
        let tasks: Vec<Task> = persist::load_json(&paths.tasks_file).unwrap().unwrap();
        assert_eq!(tasks[0].id, "restored");

        let snapshot = PathBuf::from(&backup_dir);
        assert!(snapshot.starts_with(&paths.data_dir));
        assert_eq!(
            std::fs::read(snapshot.join("config.json")).unwrap(),
            b"{\"old\":true}"
        );
        assert_eq!(std::fs::read(snapshot.join("tasks.json")).unwrap(), b"[]");
        // 恢复前不存在的文件不写入快照。
        assert!(!snapshot.join("jobs.json").exists());
    }

    #[test]
    fn write_failure_rolls_back_previous_files() {
        let paths = temp_paths();
        std::fs::write(&paths.config_file, b"ORIGINAL-CONFIG").unwrap();
        std::fs::write(&paths.history_file, b"ORIGINAL-HISTORY").unwrap();
        // 让第三个写入目标（jobs.json）是一个目录，rename 必然失败。
        std::fs::create_dir_all(&paths.jobs_file).unwrap();

        let targets = vec![
            (paths.config_file.clone(), b"NEW-CONFIG".to_vec()),
            (paths.history_file.clone(), b"NEW-HISTORY".to_vec()),
            (paths.jobs_file.clone(), b"NEW-JOBS".to_vec()),
        ];
        let err = write_targets(&targets).unwrap_err();
        assert!(err.contains("已回滚"));

        assert_eq!(std::fs::read(&paths.config_file).unwrap(), b"ORIGINAL-CONFIG");
        assert_eq!(std::fs::read(&paths.history_file).unwrap(), b"ORIGINAL-HISTORY");
        assert!(paths.jobs_file.is_dir());
    }

    #[test]
    fn rollback_removes_files_that_did_not_exist_before() {
        let paths = temp_paths();
        let missing = paths.config_file.clone();
        assert!(!missing.exists());
        std::fs::create_dir_all(&paths.history_file).unwrap();

        let targets = vec![
            (missing.clone(), b"NEW".to_vec()),
            (paths.history_file.clone(), b"NEW".to_vec()),
        ];
        assert!(write_targets(&targets).is_err());
        assert!(!missing.exists(), "原本不存在的文件必须在回滚时删除");
    }

    #[test]
    fn restore_notice_mentions_disabled_command_tasks() {
        let paths = temp_paths();
        write_restore_notice(&paths, 3, "C:/data/restore-backup-x").unwrap();
        let text = std::fs::read_to_string(&paths.restore_notice_file).unwrap();
        assert!(text.contains("3 个命令任务待确认"));
        assert!(text.contains("已关闭外部命令执行"));
    }
}
