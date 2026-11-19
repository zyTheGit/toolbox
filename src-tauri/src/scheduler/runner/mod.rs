//! 定时任务调度器：任务管理与对外 API。
//!
//! 调度循环与执行逻辑在 engine，输入规整与时间计算在 schedule。

mod engine;
mod schedule;

use engine::{sort_tasks, tick, try_reserve_queue};
use schedule::{
    build_run, command_of, compute_next, new_id, normalize_input, now_string, preview_cron,
};

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use parking_lot::Mutex;
use tauri::AppHandle;
use tokio::sync::Semaphore;

use crate::models::{Task, TaskInput, TaskRun};
use crate::state::{push_warning, AppPaths, SharedConfig, Warnings};

use super::command::{self};
use super::task_store::TaskStore;

const MAX_TASKS: usize = 50;

#[derive(Clone)]
pub(super) struct ActiveRun {
    pub(super) pid: Option<u32>,
    pub(super) canceled: Arc<AtomicBool>,
}

pub(super) struct Core {
    pub(super) app: AppHandle,
    pub(super) config: SharedConfig,
    pub(super) store: TaskStore,
    pub(super) warnings: Warnings,
    pub(super) tasks: Mutex<Vec<Task>>,
    pub(super) active: Mutex<HashMap<String, ActiveRun>>,
    pub(super) paused: AtomicBool,
    pub(super) stopped: AtomicBool,
    pub(super) command_sem: Semaphore,
    pub(super) command_queue: AtomicUsize,
}

pub struct Scheduler {
    core: Arc<Core>,
    ticker: Mutex<Option<tauri::async_runtime::JoinHandle<()>>>,
}

impl Scheduler {
    pub fn new(app: AppHandle, paths: AppPaths, config: SharedConfig, warnings: Warnings) -> Self {
        let _ = paths.ensure_dirs();
        let store = TaskStore::new(&paths);
        let mut tasks = match store.load() {
            Ok(tasks) => tasks,
            Err(err) => {
                push_warning(&warnings, err);
                Vec::new()
            }
        };
        let mut changed = false;
        let mut recovered_runs: Vec<TaskRun> = Vec::new();
        let now = now_string();

        for task in tasks.iter_mut() {
            if task.running {
                task.running = false;
                task.last_status = "interrupted".into();
                task.last_error = "应用退出导致中断".into();
                recovered_runs.push(TaskRun {
                    id: new_id(),
                    task_id: task.id.clone(),
                    trigger: "interrupted".into(),
                    status: "interrupted".into(),
                    started_at: now.clone(),
                    finished_at: now.clone(),
                    output: task.last_output.clone(),
                    error: "应用退出导致中断".into(),
                    job_id: task.last_job_id.clone(),
                    command: command_of(task),
                    ..TaskRun::default()
                });
                changed = true;
            }
            let next = if task.enabled {
                compute_next(task)
            } else {
                None
            };
            if next != task.next_run {
                task.next_run = next;
                changed = true;
            }
        }

        for run in &recovered_runs {
            if let Err(err) = store.append_run(run) {
                push_warning(&warnings, err);
            }
        }
        if changed {
            if let Err(err) = store.save(&tasks) {
                push_warning(&warnings, err);
            }
        }

        let core = Arc::new(Core {
            app,
            config,
            store,
            warnings,
            tasks: Mutex::new(tasks),
            active: Mutex::new(HashMap::new()),
            paused: AtomicBool::new(false),
            stopped: AtomicBool::new(false),
            command_sem: Semaphore::new(1),
            command_queue: AtomicUsize::new(0),
        });

        Self {
            core,
            ticker: Mutex::new(None),
        }
    }

    pub fn start(&self) {
        let mut guard = self.ticker.lock();
        if guard.is_some() {
            return;
        }
        self.core.stopped.store(false, Ordering::SeqCst);
        let core = self.core.clone();
        *guard = Some(tauri::async_runtime::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(1));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            loop {
                interval.tick().await;
                if core.stopped.load(Ordering::SeqCst) {
                    break;
                }
                tick(&core);
            }
        }));
        drop(guard);
        self.core.emit_tasks();
    }

    pub fn shutdown(&self) {
        self.core.stopped.store(true, Ordering::SeqCst);
        self.ticker.lock().take();

        let active: Vec<ActiveRun> = self
            .core
            .active
            .lock()
            .values()
            .cloned()
            .collect();
        for run in active {
            run.canceled.store(true, Ordering::SeqCst);
            if let Some(pid) = run.pid {
                command::kill_tree(pid);
            }
        }

        {
            let mut tasks = self.core.tasks.lock();
            for task in tasks.iter_mut() {
                if task.running {
                    task.running = false;
                    task.last_status = "interrupted".into();
                    task.last_error = "应用退出导致中断".into();
                }
            }
            // 持锁写盘，避免与 create / update / delete 交错后旧快照覆盖新状态。
            if let Err(err) = self.core.store.save(tasks.as_slice()) {
                push_warning(&self.core.warnings, err);
            }
        }
    }

    pub fn list(&self) -> Vec<Task> {
        let mut tasks = self.core.tasks.lock().clone();
        sort_tasks(&mut tasks);
        tasks
    }

    /// 从磁盘重新载入任务到内存（备份恢复后调用）。
    ///
    /// 只替换内存快照，不回写磁盘；恢复前已 `shutdown` 停写，避免 tick 用
    /// 旧数据覆盖刚恢复的文件。恢复成功后保持停止，直到应用重启。
    pub fn reload_from_disk(&self) -> Result<(), String> {
        let tasks = self.core.store.load()?;
        *self.core.tasks.lock() = tasks;
        self.core.emit_tasks();
        Ok(())
    }

    pub fn create(&self, input: TaskInput) -> Result<Task, String> {
        let fields = normalize_input(&input)?;
        self.ensure_command_allowed(&fields.task_type)?;

        let mut tasks = self.core.tasks.lock();
        if tasks.len() >= MAX_TASKS {
            return Err(format!("[CONFLICT] 定时任务数量已达上限（{MAX_TASKS}）"));
        }
        let previous = tasks.clone();
        let now = now_string();
        let mut task = Task {
            id: new_id(),
            name: fields.name,
            task_type: fields.task_type,
            cron: fields.cron,
            schedule: fields.schedule,
            run_at: fields.run_at,
            enabled: fields.enabled,
            payload: fields.payload,
            created_at: now.clone(),
            updated_at: now,
            revision: 1,
            ..Task::default()
        };
        task.next_run = if task.enabled {
            compute_next(&task)
        } else {
            None
        };
        tasks.push(task.clone());
        if let Err(err) = self.core.store.save(&tasks) {
            *tasks = previous;
            return Err(err);
        }
        drop(tasks);
        self.core.emit_tasks();
        Ok(task)
    }

    pub fn update(&self, id: &str, input: TaskInput) -> Result<Task, String> {
        let fields = normalize_input(&input)?;
        self.ensure_command_allowed(&fields.task_type)?;

        let mut tasks = self.core.tasks.lock();
        let previous = tasks.clone();
        let task = {
            let task = tasks
                .iter_mut()
                .find(|task| task.id == id)
                .ok_or_else(|| "任务不存在".to_string())?;
            if task.running {
                return Err("[CONFLICT] 任务运行中，暂不能编辑".into());
            }
            task.name = fields.name;
            task.task_type = fields.task_type;
            task.cron = fields.cron;
            task.schedule = fields.schedule;
            task.run_at = fields.run_at;
            task.enabled = fields.enabled;
            task.payload = fields.payload;
            task.next_run = if task.enabled {
                compute_next(task)
            } else {
                None
            };
            task.updated_at = now_string();
            task.revision = task.revision.wrapping_add(1);
            task.clone()
        };
        if let Err(err) = self.core.store.save(&tasks) {
            *tasks = previous;
            return Err(err);
        }
        drop(tasks);
        self.core.emit_tasks();
        Ok(task)
    }

    pub fn delete(&self, id: &str) -> Result<(), String> {
        let mut tasks = self.core.tasks.lock();
        let previous = tasks.clone();
        let index = tasks
            .iter()
            .position(|task| task.id == id)
            .ok_or_else(|| "任务不存在".to_string())?;
        if tasks[index].running {
            return Err("[CONFLICT] 任务运行中，暂不能删除".into());
        }
        tasks.remove(index);
        if let Err(err) = self.core.store.save(&tasks) {
            *tasks = previous;
            return Err(err);
        }
        drop(tasks);
        self.core.store.remove_runs(id);
        self.core.emit_tasks();
        Ok(())
    }

    /// A disabled security switch must stop the task at every entry point, not
    /// only when the scheduler tick or run_now is reached.
    fn ensure_command_allowed(&self, task_type: &str) -> Result<(), String> {
        if task_type == "command" && !self.core.config.read().security.allow_commands {
            return Err("[CONFLICT] 执行命令已在设置中关闭，请先在设置中开启".into());
        }
        Ok(())
    }

    pub fn run_now(&self, id: &str) -> Result<TaskRun, String> {
        let allow_commands = self.core.config.read().security.allow_commands;
        let started_at = now_string();

        let (run, task_snapshot, reserved, skipped) = {
            let mut tasks = self.core.tasks.lock();
            let task = tasks
                .iter_mut()
                .find(|task| task.id == id)
                .ok_or_else(|| "任务不存在".to_string())?;
            if task.running {
                return Err("[CONFLICT] 任务正在运行，暂不能重复执行".into());
            }
            if task.task_type == "command" && !allow_commands {
                return Err("[CONFLICT] 执行命令已在设置中关闭".into());
            }

            if task.task_type == "command" && !try_reserve_queue(&self.core.command_queue) {
                let mut run = build_run(task, "manual", &started_at, "skipped");
                run.error = "命令队列已满".into();
                task.last_status = "skipped".into();
                task.last_error = "命令队列已满".into();
                (run, task.clone(), false, true)
            } else {
                let reserved = task.task_type == "command";
                let run = build_run(task, "manual", &started_at, "running");
                task.running = true;
                task.last_run_at = started_at.clone();
                task.last_status = "running".into();
                task.last_error.clear();
                (run, task.clone(), reserved, false)
            }
        };

        self.core.save_and_emit_tasks();

        if skipped {
            self.core.append_run_and_emit(&run);
            return Ok(run);
        }

        self.core.emit_run(&run);
        self.core.start_run(task_snapshot, run.clone(), reserved);
        Ok(run)
    }

    pub fn cancel(&self, id: &str) -> Result<(), String> {
        let entry = self.core.active.lock().get(id).cloned();
        let entry = entry.ok_or_else(|| "任务当前没有正在执行的运行".to_string())?;
        entry.canceled.store(true, Ordering::SeqCst);
        if let Some(pid) = entry.pid {
            command::kill_tree(pid);
        }
        Ok(())
    }

    pub fn runs(&self, task_id: &str) -> Result<Vec<TaskRun>, String> {
        self.core.store.load_runs(task_id)
    }

    pub fn set_paused(&self, paused: bool) -> bool {
        self.core.paused.store(paused, Ordering::SeqCst);
        paused
    }

    pub fn paused(&self) -> bool {
        self.core.paused.load(Ordering::SeqCst)
    }

    pub fn active_count(&self) -> usize {
        self.core
            .tasks
            .lock()
            .iter()
            .filter(|task| task.running)
            .count()
    }

    pub fn preview(&self, expr: &str) -> Result<Vec<String>, String> {
        preview_cron(expr, 5)
    }
}
