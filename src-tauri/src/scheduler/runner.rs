use std::collections::HashMap;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::{DateTime, Local, NaiveDateTime};
use croner::Cron;
use parking_lot::Mutex;
use tauri::{AppHandle, Emitter};
use tokio::sync::Semaphore;
use uuid::Uuid;

use crate::models::{Task, TaskInput, TaskPayload, TaskRun};
use crate::state::{push_warning, AppPaths, SharedConfig, Warnings};

use super::command::{self, CommandSpec, ExecOutcome};
use super::task_store::TaskStore;

const MAX_TASKS: usize = 50;
const MAX_COMMAND_QUEUE: usize = 5;
const DUE_GRACE_SECONDS: i64 = 90;
const NOW_FORMAT: &str = "%Y-%m-%d %H:%M:%S";
const NEXT_FORMAT: &str = "%Y-%m-%d %H:%M:%S";
const LEGACY_NEXT_FORMAT: &str = "%Y-%m-%d %H:%M";

#[derive(Clone)]
struct ActiveRun {
    pid: Option<u32>,
    canceled: Arc<AtomicBool>,
}

struct Core {
    app: AppHandle,
    config: SharedConfig,
    store: TaskStore,
    warnings: Warnings,
    tasks: Mutex<Vec<Task>>,
    active: Mutex<HashMap<String, ActiveRun>>,
    paused: AtomicBool,
    stopped: AtomicBool,
    command_sem: Semaphore,
    command_queue: AtomicUsize,
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
        }
        let snapshot = self.core.tasks.lock().clone();
        if let Err(err) = self.core.store.save(&snapshot) {
            push_warning(&self.core.warnings, err);
        }
    }

    pub fn list(&self) -> Vec<Task> {
        let mut tasks = self.core.tasks.lock().clone();
        tasks.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        tasks
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

fn tick(core: &Arc<Core>) {
    if core.paused.load(Ordering::SeqCst) || core.stopped.load(Ordering::SeqCst) {
        return;
    }
    let now = Local::now();
    let now_naive = now.naive_local();
    let started_at = now.format(NOW_FORMAT).to_string();
    let allow_commands = core.config.read().security.allow_commands;

    let mut triggers: Vec<(Task, TaskRun, bool)> = Vec::new();
    let mut skipped_runs: Vec<TaskRun> = Vec::new();
    let mut changed = false;

    {
        let mut tasks = core.tasks.lock();
        for task in tasks.iter_mut() {
            if !task.enabled {
                if task.next_run.is_some() {
                    task.next_run = None;
                    changed = true;
                }
                continue;
            }

            let due = match task.next_run.as_deref().and_then(parse_next) {
                Some(due) => due,
                None => {
                    changed |= refresh_next(task);
                    continue;
                }
            };

            if now_naive < due {
                continue;
            }

            let late = now_naive.signed_duration_since(due).num_seconds();
            if late > DUE_GRACE_SECONDS {
                if is_once(task) {
                    let mut run = build_run(task, "schedule", &started_at, "skipped");
                    run.error = "已错过指定时间（应用未运行）".into();
                    skipped_runs.push(run);
                    task.last_status = "skipped".into();
                    task.last_error = "已错过指定时间（应用未运行）".into();
                    close_once(task);
                } else {
                    refresh_next(task);
                }
                changed = true;
                continue;
            }

            if task.task_type == "command" && !allow_commands {
                if is_once(task) {
                    let mut run = build_run(task, "schedule", &started_at, "skipped");
                    run.error = "执行命令已在设置中关闭".into();
                    skipped_runs.push(run);
                    task.last_status = "skipped".into();
                    task.last_error = "执行命令已在设置中关闭".into();
                    close_once(task);
                    changed = true;
                } else {
                    changed |= refresh_next(task);
                }
                continue;
            }

            if task.running {
                let mut run = build_run(task, "schedule", &started_at, "skipped");
                run.error = "上一次运行尚未结束".into();
                skipped_runs.push(run);
                task.last_status = "skipped".into();
                task.last_error = "上一次运行尚未结束".into();
                advance_after_skip(task);
                changed = true;
                continue;
            }

            let mut reserved = false;
            if task.task_type == "command" {
                if !try_reserve_queue(&core.command_queue) {
                    let mut run = build_run(task, "schedule", &started_at, "skipped");
                    run.error = "命令队列已满".into();
                    skipped_runs.push(run);
                    task.last_status = "skipped".into();
                    task.last_error = "命令队列已满".into();
                    advance_after_skip(task);
                    changed = true;
                    continue;
                }
                reserved = true;
            }

            let run = build_run(task, "schedule", &started_at, "running");
            task.running = true;
            task.last_run_at = started_at.clone();
            task.last_status = "running".into();
            task.last_error.clear();
            if is_once(task) {
                close_once(task);
            } else {
                refresh_next(task);
            }
            triggers.push((task.clone(), run, reserved));
            changed = true;
        }
    }

    if changed {
        core.save_and_emit_tasks();
    }
    for run in skipped_runs {
        core.append_run_and_emit(&run);
    }
    for (task, run, reserved) in triggers {
        core.emit_run(&run);
        core.start_run(task, run, reserved);
    }
}

impl Core {
    fn emit_tasks(&self) {
        let tasks = self.tasks.lock().clone();
        let _ = self.app.emit("tasks-updated", tasks);
    }

    fn save_and_emit_tasks(&self) {
        let tasks = self.tasks.lock().clone();
        if let Err(err) = self.store.save(&tasks) {
            push_warning(&self.warnings, err);
        }
        let _ = self.app.emit("tasks-updated", tasks);
    }

    fn emit_run(&self, run: &TaskRun) {
        let _ = self.app.emit("task-run-updated", run.clone());
    }

    fn notify(&self, title: &str, body: &str) -> Result<(), String> {
        use tauri_plugin_notification::NotificationExt;
        self.app
            .notification()
            .builder()
            .title(title)
            .body(body)
            .show()
            .map_err(|err| err.to_string())
    }

    fn append_run_and_emit(&self, run: &TaskRun) {
        if let Err(err) = self.store.append_run(run) {
            push_warning(&self.warnings, err);
        }
        self.emit_run(run);
    }

    fn set_active_pid(&self, task_id: &str, pid: u32) {
        if let Some(entry) = self.active.lock().get_mut(task_id) {
            entry.pid = Some(pid);
        }
    }

    fn start_run(self: &Arc<Self>, task: Task, run: TaskRun, reserved: bool) {
        let canceled = Arc::new(AtomicBool::new(false));
        self.active.lock().insert(
            task.id.clone(),
            ActiveRun {
                pid: None,
                canceled: canceled.clone(),
            },
        );

        let core = self.clone();
        tauri::async_runtime::spawn(async move {
            let started = Instant::now();
            let outcome = match task.task_type.as_str() {
                "command" => execute_command(&core, &task, &run, canceled.clone(), reserved).await,
                "reminder" => execute_reminder(&core, &task).await,
                _ => ExecOutcome::failed("未知任务类型"),
            };
            let outcome = core.adjust_for_stop(outcome);
            core.finish_run(&task, run, outcome, started);
        });
    }

    fn adjust_for_stop(&self, mut outcome: ExecOutcome) -> ExecOutcome {
        if self.stopped.load(Ordering::SeqCst) && outcome.status != "success" {
            outcome.status = "interrupted";
            outcome.error = "应用退出导致中断".into();
        }
        outcome
    }

    fn finish_run(&self, task: &Task, mut run: TaskRun, outcome: ExecOutcome, started: Instant) {
        let finished_at = now_string();
        let output = outcome.lines.join("\n");
        let status = outcome.status.to_string();
        if !outcome.job_id.is_empty() {
            run.job_id = outcome.job_id.clone();
        }

        {
            let mut tasks = self.tasks.lock();
            if let Some(stored) = tasks.iter_mut().find(|item| item.id == task.id) {
                stored.running = false;
                stored.last_status = status.clone();
                stored.last_output = output.clone();
                stored.last_error = outcome.error.clone();
                if !run.job_id.is_empty() {
                    stored.last_job_id = run.job_id.clone();
                }
            }
        }
        self.save_and_emit_tasks();

        run.status = status.clone();
        run.finished_at = finished_at;
        run.duration_ms = started.elapsed().as_millis() as u64;
        run.exit_code = outcome.exit_code;
        run.output = output;
        run.error = outcome.error;
        self.append_run_and_emit(&run);

        if self.config.read().notify.task
            && task.task_type == "command"
            && matches!(status.as_str(), "success" | "failed")
        {
            let label: String = if task.name.trim().is_empty() {
                "定时任务".to_string()
            } else {
                task.name.trim().chars().take(60).collect()
            };
            let title = if status == "success" {
                "任务完成"
            } else {
                "任务失败"
            };
            crate::core::notify::notify(
                &self.app,
                "tasks",
                title,
                &format!("{label}：点击查看运行记录"),
            );
        }

        self.active.lock().remove(&task.id);
    }
}

async fn execute_reminder(core: &Arc<Core>, task: &Task) -> ExecOutcome {
    let title = if task.name.trim().is_empty() {
        "Toolbox 提醒".to_string()
    } else {
        task.name.trim().to_string()
    };
    let message = task.payload.message.trim();
    let body = if message.is_empty() {
        "定时提醒".to_string()
    } else {
        message.to_string()
    };

    match core.notify(&title, &body) {
        Ok(()) => ExecOutcome {
            status: "success",
            exit_code: None,
            lines: vec![format!("提醒：{body}")],
            error: String::new(),
            job_id: String::new(),
        },
        Err(err) => {
            let dialog_title = title;
            let dialog_body = body.clone();
            std::thread::spawn(move || {
                let _ = rfd::MessageDialog::new()
                    .set_level(rfd::MessageLevel::Info)
                    .set_title(&dialog_title)
                    .set_description(&dialog_body)
                    .show();
            });
            ExecOutcome {
                status: "success",
                exit_code: None,
                lines: vec![
                    format!("提醒（系统通知不可用，已弹窗）：{body}"),
                    format!("系统通知错误：{err}"),
                ],
                error: String::new(),
                job_id: String::new(),
            }
        }
    }
}

async fn execute_command(
    core: &Arc<Core>,
    task: &Task,
    run: &TaskRun,
    canceled: Arc<AtomicBool>,
    reserved: bool,
) -> ExecOutcome {
    let spec: CommandSpec = match command::build_spec(&task.payload) {
        Ok(spec) => spec,
        Err(err) => {
            if reserved {
                core.command_queue.fetch_sub(1, Ordering::SeqCst);
            }
            return ExecOutcome::failed(err);
        }
    };

    if canceled.load(Ordering::SeqCst) {
        if reserved {
            core.command_queue.fetch_sub(1, Ordering::SeqCst);
        }
        return ExecOutcome::canceled();
    }

    let permit = match core.command_sem.acquire().await {
        Ok(permit) => permit,
        Err(_) => {
            if reserved {
                core.command_queue.fetch_sub(1, Ordering::SeqCst);
            }
            return ExecOutcome::failed("命令执行器不可用");
        }
    };
    if reserved {
        core.command_queue.fetch_sub(1, Ordering::SeqCst);
    }

    if canceled.load(Ordering::SeqCst) || core.stopped.load(Ordering::SeqCst) {
        drop(permit);
        return ExecOutcome::canceled();
    }

    let app = core.app.clone();
    let run_template = run.clone();
    let task_id = task.id.clone();
    let core_for_pid = core.clone();
    let outcome = command::run_command(
        spec,
        canceled,
        move |pid| core_for_pid.set_active_pid(&task_id, pid),
        move |lines| {
            let mut partial = run_template.clone();
            partial.status = "running".into();
            partial.output = lines.join("\n");
            let _ = app.emit("task-run-updated", partial);
        },
    )
    .await;

    drop(permit);
    outcome
}

fn try_reserve_queue(counter: &AtomicUsize) -> bool {
    // counter includes the running command, so allow one extra in-flight slot.
    let previous = counter.fetch_add(1, Ordering::SeqCst);
    if previous > MAX_COMMAND_QUEUE {
        counter.fetch_sub(1, Ordering::SeqCst);
        false
    } else {
        true
    }
}

struct TaskFields {
    name: String,
    task_type: String,
    cron: String,
    schedule: String,
    run_at: String,
    enabled: bool,
    payload: TaskPayload,
}

fn normalize_input(input: &TaskInput) -> Result<TaskFields, String> {
    let name = input.name.trim();
    if name.is_empty() {
        return Err("[PARAM] 请填写任务名称".into());
    }

    let task_type = input.task_type.trim().to_ascii_lowercase();
    if !matches!(task_type.as_str(), "command" | "reminder") {
        return Err("[PARAM] 任务类型仅支持终端或提醒".into());
    }

    let schedule = match input.schedule.trim() {
        "" | "cron" => "cron".to_string(),
        "once" => "once".to_string(),
        _ => return Err("[PARAM] 触发方式无效".into()),
    };

    let (cron, run_at) = if schedule == "once" {
        let run_at = input.run_at.trim();
        let parsed = parse_run_at(run_at)
            .ok_or_else(|| "[PARAM] 指定时间格式应为 YYYY-MM-DD HH:MM:SS".to_string())?;
        if input.enabled && parsed <= Local::now().naive_local() {
            return Err("[PARAM] 指定时间必须晚于当前时间".into());
        }
        (String::new(), run_at.to_string())
    } else {
        let cron = input.cron.trim().to_string();
        parse_cron(&cron)?;
        (cron, String::new())
    };

    let mut payload = input.payload.clone();
    match task_type.as_str() {
        "command" => {
            if payload.command.trim().is_empty() {
                return Err("[PARAM] 命令任务需要填写命令内容".into());
            }
            payload.command = payload.command.trim().to_string();
            payload.shell = if payload.shell.eq_ignore_ascii_case("powershell") {
                "powershell".into()
            } else {
                "cmd".into()
            };
            payload.work_dir = payload.work_dir.trim().to_string();
            if payload.timeout_seconds == 0 {
                payload.timeout_seconds = 600;
            }
            payload.env = payload.env.replace("\r\n", "\n");
            command::parse_env(&payload.env)?;
        }
        "reminder" => {
            payload.message = payload.message.trim().to_string();
        }
        _ => return Err("[PARAM] 任务类型无效".into()),
    }

    Ok(TaskFields {
        name: name.to_string(),
        task_type,
        cron,
        schedule,
        run_at,
        enabled: input.enabled,
        payload,
    })
}

fn parse_cron(expr: &str) -> Result<Cron, String> {
    let expr = expr.trim();
    if expr.is_empty() {
        return Err("[PARAM] cron 表达式不能为空".into());
    }
    if expr.split_whitespace().count() != 5 {
        return Err("[PARAM] cron 表达式需要 5 段（分 时 日 月 周）".into());
    }
    Cron::from_str(expr).map_err(|err| format!("[PARAM] cron 表达式无效: {err}"))
}

fn is_once(task: &Task) -> bool {
    task.schedule == "once"
}

fn close_once(task: &mut Task) {
    task.enabled = false;
    task.next_run = None;
}

fn advance_after_skip(task: &mut Task) {
    if is_once(task) {
        close_once(task);
    } else {
        refresh_next(task);
    }
}

fn parse_run_at(value: &str) -> Option<NaiveDateTime> {
    NaiveDateTime::parse_from_str(value.trim(), NEXT_FORMAT).ok()
}

fn compute_next(task: &Task) -> Option<String> {
    if !task.enabled {
        return None;
    }
    if is_once(task) {
        return parse_run_at(&task.run_at).map(|time| time.format(NEXT_FORMAT).to_string());
    }
    compute_cron_next(&task.cron)
}

fn compute_cron_next(expr: &str) -> Option<String> {
    let cron = parse_cron(expr).ok()?;
    let next = cron.find_next_occurrence(&Local::now(), false).ok()?;
    Some(format_next(&next))
}

fn refresh_next(task: &mut Task) -> bool {
    let next = compute_next(task);
    if task.next_run == next {
        false
    } else {
        task.next_run = next;
        true
    }
}

fn preview_cron(expr: &str, count: usize) -> Result<Vec<String>, String> {
    let cron = parse_cron(expr)?;
    let mut cursor = Local::now();
    let mut occurrences = Vec::with_capacity(count);
    for _ in 0..count {
        let next = cron
            .find_next_occurrence(&cursor, false)
            .map_err(|err| format!("cron 表达式无效: {err}"))?;
        occurrences.push(format_next(&next));
        cursor = next;
    }
    Ok(occurrences)
}

fn format_next(time: &DateTime<Local>) -> String {
    time.format(NEXT_FORMAT).to_string()
}

fn parse_next(value: &str) -> Option<NaiveDateTime> {
    let value = value.trim();
    NaiveDateTime::parse_from_str(value, NEXT_FORMAT)
        .or_else(|_| NaiveDateTime::parse_from_str(value, LEGACY_NEXT_FORMAT))
        .ok()
}

fn now_string() -> String {
    Local::now().format(NOW_FORMAT).to_string()
}

fn new_id() -> String {
    Uuid::new_v4().to_string()
}

fn command_of(task: &Task) -> String {
    if task.task_type == "command" {
        task.payload.command.clone()
    } else {
        String::new()
    }
}

fn build_run(task: &Task, trigger: &str, started_at: &str, status: &str) -> TaskRun {
    TaskRun {
        id: new_id(),
        task_id: task.id.clone(),
        trigger: trigger.to_string(),
        status: status.to_string(),
        started_at: started_at.to_string(),
        finished_at: if status == "running" {
            String::new()
        } else {
            started_at.to_string()
        },
        command: command_of(task),
        ..TaskRun::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Timelike;

    #[test]
    fn five_field_cron_computes_future_time() {
        let now = Local::now();
        let cron = parse_cron("0 3 * * *").expect("cron should parse");
        let next = cron
            .find_next_occurrence(&now, false)
            .expect("next occurrence");
        assert!(next > now);
        assert_eq!(next.hour(), 3);
        assert_eq!(next.minute(), 0);
        assert_eq!(next.second(), 0);
    }

    #[test]
    fn cron_requires_exactly_five_fields() {
        assert!(parse_cron("0 0 3 * * *").is_err());
        assert!(parse_cron("0 3 * *").is_err());
        assert!(parse_cron("").is_err());
        assert!(parse_cron("nope nope nope nope nope").is_err());
    }

    #[test]
    fn preview_returns_five_timestamp_strings() {
        let preview = preview_cron("*/5 * * * *", 5).expect("preview");
        assert_eq!(preview.len(), 5);
        for item in &preview {
            assert_eq!(item.len(), 19, "unexpected format: {item}");
            assert!(parse_next(item).is_some());
        }
        let mut sorted = preview.clone();
        sorted.sort();
        assert_eq!(sorted, preview);
    }

    #[test]
    fn next_time_format_round_trips_for_stored_value() {
        let next = compute_cron_next("0 3 * * *").expect("next");
        let parsed = parse_next(&next).expect("stored next should parse");
        assert_eq!(next, parsed.format(NEXT_FORMAT).to_string());
    }

    #[test]
    fn legacy_minute_precision_next_run_still_parses() {
        assert!(parse_next("2026-05-15 17:00").is_some());
        assert!(parse_next("2026-05-15 17:00:30").is_some());
        assert!(parse_next("2026-05-15").is_none());
    }

    #[test]
    fn once_schedule_keeps_exact_second() {
        let run_at = (Local::now() + chrono::Duration::days(180))
            .format(NEXT_FORMAT)
            .to_string();
        let task = Task {
            task_type: "reminder".into(),
            schedule: "once".into(),
            run_at: run_at.clone(),
            enabled: true,
            ..Task::default()
        };
        assert_eq!(compute_next(&task), Some(run_at));
    }

    #[test]
    fn once_schedule_requires_seconds() {
        assert!(parse_run_at("2027-05-15 17:00").is_none());
        assert!(parse_run_at("2027-05-15 17:00:30").is_some());
    }

    #[test]
    fn past_once_time_is_rejected_when_enabled() {
        let input = TaskInput {
            name: "续费服务器".into(),
            task_type: "reminder".into(),
            schedule: "once".into(),
            run_at: "2020-01-01 00:00:00".into(),
            enabled: true,
            ..TaskInput::default()
        };
        assert!(normalize_input(&input).is_err());

        let disabled = TaskInput {
            enabled: false,
            ..input
        };
        assert!(normalize_input(&disabled).is_ok());
    }

    #[test]
    fn unsupported_task_type_is_rejected() {
        let base = TaskInput {
            name: "旧任务".into(),
            enabled: false,
            ..TaskInput::default()
        };
        for task_type in ["download", "translate", "other"] {
            let input = TaskInput {
                task_type: task_type.into(),
                ..base.clone()
            };
            assert!(normalize_input(&input).is_err(), "{task_type} should be rejected");
        }
    }

    #[test]
    fn command_queue_caps_waiting_at_five() {
        let counter = AtomicUsize::new(0);
        for _ in 0..=MAX_COMMAND_QUEUE {
            assert!(try_reserve_queue(&counter));
        }
        assert_eq!(counter.load(Ordering::SeqCst), MAX_COMMAND_QUEUE + 1);
        assert!(!try_reserve_queue(&counter));
        assert_eq!(counter.load(Ordering::SeqCst), MAX_COMMAND_QUEUE + 1);
    }
}
