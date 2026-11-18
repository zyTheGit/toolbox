//! 定时任务命令。

use tauri::{AppHandle, Emitter};

use crate::models::{Task, TaskInput, TaskRun};
use crate::state::AppState;

type CmdResult<T> = Result<T, String>;

#[tauri::command]
pub fn tasks_list(state: tauri::State<'_, AppState>) -> CmdResult<Vec<Task>> {
    Ok(state.scheduler.list())
}

#[tauri::command]
pub fn task_create(input: TaskInput, state: tauri::State<'_, AppState>) -> CmdResult<Task> {
    state.ensure_writable()?;
    state.scheduler.create(input)
}

#[tauri::command]
pub fn task_update(
    id: String,
    input: TaskInput,
    state: tauri::State<'_, AppState>,
) -> CmdResult<Task> {
    state.ensure_writable()?;
    state.scheduler.update(&id, input)
}

#[tauri::command]
pub fn task_delete(id: String, state: tauri::State<'_, AppState>) -> CmdResult<()> {
    state.ensure_writable()?;
    state.scheduler.delete(&id)
}

#[tauri::command]
pub fn task_run_now(id: String, state: tauri::State<'_, AppState>) -> CmdResult<TaskRun> {
    state.ensure_writable()?;
    state.scheduler.run_now(&id)
}

#[tauri::command]
pub fn task_cancel(id: String, state: tauri::State<'_, AppState>) -> CmdResult<()> {
    state.ensure_writable()?;
    state.scheduler.cancel(&id)
}

#[tauri::command]
pub fn task_runs(task_id: String, state: tauri::State<'_, AppState>) -> CmdResult<Vec<TaskRun>> {
    state.scheduler.runs(&task_id)
}

#[tauri::command]
pub fn tasks_set_paused(
    paused: bool,
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> CmdResult<bool> {
    let value = state.scheduler.set_paused(paused);
    app.emit("tasks-paused", value).map_err(|e| e.to_string())?;
    Ok(value)
}

#[tauri::command]
pub fn cron_preview(expr: String, state: tauri::State<'_, AppState>) -> CmdResult<Vec<String>> {
    state.scheduler.preview(&expr)
}
