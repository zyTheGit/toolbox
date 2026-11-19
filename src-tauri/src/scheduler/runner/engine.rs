//! 调度引擎：tick 循环、任务执行与生命周期事件。

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

use chrono::Local;
use tauri::Emitter;

use crate::models::{Task, TaskRun};
use crate::state::push_warning;

use super::command::{self, ExecOutcome};
use super::schedule::{
    advance_after_skip, build_run, close_once, is_once, now_string, parse_next, refresh_next,
    NOW_FORMAT,
};
use super::{ActiveRun, Core};

const MAX_COMMAND_QUEUE: usize = 5;
const DUE_GRACE_SECONDS: i64 = 90;

pub(super) fn tick(core: &Arc<Core>) {
    if core.paused.load(Ordering::SeqCst) || core.stopped.load(Ordering::SeqCst) { return; }
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
    pub(super) fn emit_tasks(&self) {
        let tasks = self.tasks.lock().clone();
        let _ = self.app.emit("tasks-updated", tasks);
    }

    pub(super) fn save_and_emit_tasks(&self) {
        // 持 tasks 锁写盘：锁外旧快照会覆盖并发 create / delete / finish_run 的新状态。
        let tasks = self.tasks.lock();
        if self.stopped.load(Ordering::SeqCst) {
            return;
        }
        if let Err(err) = self.store.save(tasks.as_slice()) {
            push_warning(&self.warnings, err);
        }
        let snapshot = (*tasks).clone();
        drop(tasks);
        let _ = self.app.emit("tasks-updated", snapshot);
    }

    pub(super) fn emit_run(&self, run: &TaskRun) {
        let _ = self.app.emit("task-run-updated", run.clone());
    }

    pub(super) fn append_run_and_emit(&self, run: &TaskRun) {
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

    pub(super) fn start_run(self: &Arc<Self>, task: Task, run: TaskRun, reserved: bool) {
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
                Some(&task.id),
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

    match crate::core::notify::notify_task(&core.app, &task.id, &title, &body) {
        Ok(()) => ExecOutcome {
            status: "success",
            exit_code: None,
            lines: vec![format!("提醒：{body}")],
            error: String::new(),
            job_id: String::new(),
        },
        Err(err) => {
            #[cfg(not(test))]
            crate::dialogs::info_later(title, &body);
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
    let spec = match command::build_spec(&task.payload) {
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

pub(super) fn try_reserve_queue(counter: &AtomicUsize) -> bool {
    // counter includes the running command, so allow one extra in-flight slot.
    let previous = counter.fetch_add(1, Ordering::SeqCst);
    if previous > MAX_COMMAND_QUEUE {
        counter.fetch_sub(1, Ordering::SeqCst);
        false
    } else {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
