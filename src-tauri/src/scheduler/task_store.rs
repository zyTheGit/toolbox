use std::path::PathBuf;

use parking_lot::Mutex;

use crate::core::persist;
use crate::models::{Task, TaskRun};
use crate::state::AppPaths;

const MAX_RUNS_PER_TASK: usize = 20;

pub struct TaskStore {
    tasks_file: PathBuf,
    runs_dir: PathBuf,
    write_lock: Mutex<()>,
}

impl TaskStore {
    pub fn new(paths: &AppPaths) -> Self {
        Self {
            tasks_file: paths.tasks_file.clone(),
            runs_dir: paths.runs_dir.clone(),
            write_lock: Mutex::new(()),
        }
    }

    pub fn load(&self) -> Result<Vec<Task>, String> {
        Ok(persist::load_json::<Vec<Task>>(&self.tasks_file)?.unwrap_or_default())
    }

    pub fn save(&self, tasks: &[Task]) -> Result<(), String> {
        let _guard = self.write_lock.lock();
        std::fs::create_dir_all(&self.runs_dir)
            .map_err(|err| format!("创建运行记录目录失败: {err}"))?;
        persist::save_json(&self.tasks_file, tasks)
    }

    pub fn load_runs(&self, task_id: &str) -> Result<Vec<TaskRun>, String> {
        let Some(path) = self.run_path(task_id) else {
            return Ok(Vec::new());
        };
        let mut runs = persist::load_json::<Vec<TaskRun>>(&path)?.unwrap_or_default();
        runs.truncate(MAX_RUNS_PER_TASK);
        Ok(runs)
    }

    pub fn append_run(&self, run: &TaskRun) -> Result<(), String> {
        let Some(path) = self.run_path(&run.task_id) else {
            return Err("任务 ID 无效，无法写入运行记录".into());
        };
        let _guard = self.write_lock.lock();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|err| format!("创建运行记录目录失败: {err}"))?;
        }
        let mut runs = persist::load_json::<Vec<TaskRun>>(&path)?.unwrap_or_default();
        runs.insert(0, run.clone());
        runs.truncate(MAX_RUNS_PER_TASK);
        persist::save_json(&path, &runs)
    }

    pub fn remove_runs(&self, task_id: &str) {
        if let Some(path) = self.run_path(task_id) {
            let _ = std::fs::remove_file(path);
        }
    }

    fn run_path(&self, task_id: &str) -> Option<PathBuf> {
        if task_id.is_empty()
            || !task_id
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
        {
            return None;
        }
        Some(self.runs_dir.join(format!("{task_id}.json")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::TaskPayload;

    fn temp_paths() -> AppPaths {
        AppPaths::from_data_dir(
            std::env::temp_dir().join(format!("toolbox-task-reload-{}", uuid::Uuid::new_v4())),
        )
    }

    /// 备份恢复后的重载路径：磁盘内容被替换后，store 必须读回新任务，
    /// 且不得把内存旧数据写回。
    #[test]
    fn reload_reads_restored_tasks_without_writing() {
        let paths = temp_paths();
        std::fs::create_dir_all(&paths.data_dir).unwrap();
        let store = TaskStore::new(&paths);

        let old = Task {
            id: "old".into(),
            name: "旧任务".into(),
            ..Task::default()
        };
        store.save(&[old]).unwrap();

        let restored = Task {
            id: "restored".into(),
            name: "来自备份的命令任务".into(),
            task_type: "command".into(),
            enabled: false,
            payload: TaskPayload {
                command: "echo hello".into(),
                ..TaskPayload::default()
            },
            ..Task::default()
        };
        persist::save_json(&paths.tasks_file, &[restored.clone()]).unwrap();

        let loaded = store.load().unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].id, "restored");
        assert!(!loaded[0].enabled);

        let on_disk: Vec<Task> = persist::load_json(&paths.tasks_file).unwrap().unwrap();
        assert_eq!(on_disk[0].id, "restored");
    }
}
