pub mod secrets;

use std::path::PathBuf;

use tauri::{AppHandle, Emitter};

use crate::core;
use crate::core::secrets::SecretPatch;
use crate::models::{
    AppConfig, AppSnapshot, AutostartStatus, ConfigResult, DownloadRequest, HistoryPage,
    HistoryQuery, Job, Task, TaskInput, TaskRun, ToolInstallResult, ToolsStatus, TranslateRequest,
    TranslateResult,
};
use crate::scheduler::autostart;
use crate::state::{push_warning, AppState};

type CmdResult<T> = Result<T, String>;

#[tauri::command]
pub async fn get_app_snapshot(state: tauri::State<'_, AppState>) -> CmdResult<AppSnapshot> {
    let config = state.config();
    let view = state.view_config();
    let for_tools = config.clone();
    let paths = state.paths.clone();
    let tools = tauri::async_runtime::spawn_blocking(move || {
        core::tools::get_status(&for_tools, &paths, false)
    })
    .await
    .map_err(|e| format!("[DATA] 读取工具状态失败: {e}"))?;

    Ok(AppSnapshot {
        version: env!("CARGO_PKG_VERSION").into(),
        platform: std::env::consts::OS.into(),
        providers: core::translate::providers(&config),
        tasks: state.scheduler.list(),
        jobs: state.downloader.list(),
        autostart: autostart::status(),
        tasks_paused: state.scheduler.paused(),
        config: view,
        secrets: state.secrets.status(),
        warnings: state.warnings(),
        tools,
    })
}

#[tauri::command]
pub async fn save_config(
    app: AppHandle,
    mut config: AppConfig,
    secret_patch: Option<SecretPatch>,
    state: tauri::State<'_, AppState>,
) -> CmdResult<ConfigResult> {
    core::config::normalize(&mut config, &state.paths);
    core::config::validate(&config)?;

    let previous = state.config();
    core::quick::apply(&app, &config)?;

    let stored = (|| -> Result<(), String> {
        // Keys embedded in the payload (older UI) move into the secret store.
        state.secrets.migrate_inline(&mut config)?;
        if let Some(patch) = secret_patch.as_ref() {
            state.secrets.apply_patch(patch)?;
        }
        core::config::save(&state.paths, &config)
    })();

    if let Err(err) = stored {
        let _ = core::quick::apply(&app, &previous);
        return Err(err);
    }
    *state.config.write() = config;

    let realized = state.config();
    let for_tools = realized.clone();
    let paths = state.paths.clone();
    let tools = tauri::async_runtime::spawn_blocking(move || {
        core::tools::get_status(&for_tools, &paths, false)
    })
    .await
    .map_err(|e| format!("[DATA] 读取工具状态失败: {e}"))?;

    Ok(ConfigResult {
        providers: core::translate::providers(&realized),
        config: state.view_config(),
        secrets: state.secrets.status(),
        tools,
    })
}

#[tauri::command]
pub async fn get_tools(force: bool, state: tauri::State<'_, AppState>) -> CmdResult<ToolsStatus> {
    let config = state.config();
    let paths = state.paths.clone();
    tauri::async_runtime::spawn_blocking(move || {
        core::tools::get_status(&config, &paths, force)
    })
    .await
    .map_err(|e| format!("读取工具状态失败: {e}"))
}

#[tauri::command]
pub async fn install_tool(
    app: AppHandle,
    tool: String,
    state: tauri::State<'_, AppState>,
) -> CmdResult<ToolInstallResult> {
    {
        let mut installing = state.installing.lock();
        if !installing.insert(tool.clone()) {
            return Err("[CONFLICT] 该工具正在安装，请稍候".into());
        }
    }

    let config = state.config();
    let result = core::tools::install_tool(&app, &tool, &config, &state.paths).await;
    state.installing.lock().remove(&tool);
    result
}

#[tauri::command]
pub async fn open_folder(
    path: Option<String>,
    state: tauri::State<'_, AppState>,
) -> CmdResult<String> {
    let target = path
        .filter(|value| !value.trim().is_empty())
        .map(|value| core::config::expand_home(value.trim()))
        .unwrap_or_else(|| state.config().download_dir);

    let target = if target.trim().is_empty() {
        state.paths.downloads_dir.to_string_lossy().to_string()
    } else {
        target
    };

    let folder = PathBuf::from(&target);
    std::fs::create_dir_all(&folder).map_err(|e| format!("[PATH] 创建目录失败: {e}"))?;
    open::that(&folder).map_err(|e| format!("[PATH] 打开目录失败: {e}"))?;
    Ok(target)
}

#[tauri::command]
pub async fn pick_folder() -> CmdResult<Option<String>> {
    tauri::async_runtime::spawn_blocking(|| {
        rfd::FileDialog::new()
            .pick_folder()
            .map(|path| path.to_string_lossy().to_string())
    })
    .await
    .map_err(|e| format!("[TOOL] 打开文件选择器失败: {e}"))
}

#[tauri::command]
pub async fn translate(
    request: TranslateRequest,
    state: tauri::State<'_, AppState>,
) -> CmdResult<TranslateResult> {
    let text = request.text.trim().to_string();
    if text.is_empty() {
        return Err("[PARAM] 请输入要翻译的内容".into());
    }

    let config = state.config();
    let payload = TranslateRequest {
        text: text.clone(),
        direction: request.direction,
        provider: request.provider,
    };
    let result = core::translate::translate(&payload, &config).await?;

    if config.history.enabled {
        if let Err(err) = state
            .history
            .add(&text, &result, config.history.max_entries)
        {
            push_warning(&state.warnings, format!("保存翻译历史失败: {err}"));
        }
    }

    Ok(result)
}

#[tauri::command]
pub fn history_query(
    query: HistoryQuery,
    state: tauri::State<'_, AppState>,
) -> CmdResult<HistoryPage> {
    Ok(state.history.query(&query))
}

#[tauri::command]
pub fn history_delete(id: String, state: tauri::State<'_, AppState>) -> CmdResult<()> {
    state.history.delete(&id)
}

#[tauri::command]
pub fn history_clear(state: tauri::State<'_, AppState>) -> CmdResult<()> {
    state.history.clear()
}

#[tauri::command]
pub fn history_set_favorite(
    id: String,
    favorite: bool,
    state: tauri::State<'_, AppState>,
) -> CmdResult<()> {
    state.history.set_favorite(&id, favorite)
}

#[tauri::command]
pub fn download_enqueue(
    request: DownloadRequest,
    state: tauri::State<'_, AppState>,
) -> CmdResult<Vec<Job>> {
    let config = state.config();
    state.downloader.enqueue(&request, &config)
}

#[tauri::command]
pub fn job_cancel(id: String, state: tauri::State<'_, AppState>) -> CmdResult<()> {
    state.downloader.cancel(&id)
}

#[tauri::command]
pub fn job_retry(id: String, state: tauri::State<'_, AppState>) -> CmdResult<Job> {
    state.downloader.retry(&id)
}

#[tauri::command]
pub fn job_remove(id: String, state: tauri::State<'_, AppState>) -> CmdResult<()> {
    state.downloader.remove(&id)
}

#[tauri::command]
pub fn job_open(id: String, state: tauri::State<'_, AppState>) -> CmdResult<String> {
    state.downloader.open(&id)
}

#[tauri::command]
pub fn tasks_list(state: tauri::State<'_, AppState>) -> CmdResult<Vec<Task>> {
    Ok(state.scheduler.list())
}

#[tauri::command]
pub fn task_create(input: TaskInput, state: tauri::State<'_, AppState>) -> CmdResult<Task> {
    state.scheduler.create(input)
}

#[tauri::command]
pub fn task_update(
    id: String,
    input: TaskInput,
    state: tauri::State<'_, AppState>,
) -> CmdResult<Task> {
    state.scheduler.update(&id, input)
}

#[tauri::command]
pub fn task_delete(id: String, state: tauri::State<'_, AppState>) -> CmdResult<()> {
    state.scheduler.delete(&id)
}

#[tauri::command]
pub fn task_run_now(id: String, state: tauri::State<'_, AppState>) -> CmdResult<TaskRun> {
    state.scheduler.run_now(&id)
}

#[tauri::command]
pub fn task_cancel(id: String, state: tauri::State<'_, AppState>) -> CmdResult<()> {
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

#[tauri::command]
pub fn autostart_get() -> CmdResult<AutostartStatus> {
    Ok(autostart::status())
}

#[tauri::command]
pub fn autostart_set(enabled: bool) -> CmdResult<AutostartStatus> {
    autostart::set_enabled(enabled)
}

/// 返回应用实际使用的数据目录（跨平台，随系统而异）。
#[tauri::command]
pub fn get_data_dir(state: tauri::State<'_, AppState>) -> CmdResult<String> {
    Ok(state.paths.data_dir.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn data_export(state: tauri::State<'_, AppState>) -> CmdResult<String> {
    let picked = tauri::async_runtime::spawn_blocking(|| {
        rfd::FileDialog::new()
            .set_title("导出 Toolbox 备份")
            .set_file_name(format!(
                "toolbox-backup-{}.json",
                chrono::Local::now().format("%Y%m%d-%H%M%S")
            ))
            .add_filter("JSON", &["json"])
            .save_file()
    })
    .await
    .map_err(|e| format!("[TOOL] 打开保存对话框失败: {e}"))?;
    let Some(path) = picked else {
        return Ok(String::new());
    };
    let path = if path.extension().is_none() {
        path.with_extension("json")
    } else {
        path
    };
    core::backup::export_to(&path, &state)
}

/// Writes the restored files and restarts: in-memory config/history/tasks are
/// stale after an import, and letting them live on would overwrite the restore.
#[tauri::command]
pub async fn data_import(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> CmdResult<String> {
    let picked = tauri::async_runtime::spawn_blocking(|| {
        rfd::FileDialog::new()
            .set_title("选择 Toolbox 备份")
            .add_filter("JSON", &["json"])
            .pick_file()
    })
    .await
    .map_err(|e| format!("[TOOL] 打开文件选择器失败: {e}"))?;
    let Some(path) = picked else {
        return Ok(String::new());
    };

    let summary = core::backup::import_from(&path, &state)?;
    let handle = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(800));
        handle.restart();
    });
    Ok(format!("{summary}，应用即将重启"))
}
