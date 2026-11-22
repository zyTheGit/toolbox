pub mod ai;
pub mod backup;
pub mod db;
pub mod download;
pub mod secrets;
pub mod task;
pub mod update;

use std::path::PathBuf;

use tauri::AppHandle;

use crate::core;
use crate::core::config::ConfigSection;
use crate::core::secrets::SecretPatch;
use crate::models::{
    AppConfig, AppSnapshot, AutostartStatus, ConfigConflictError, ConfigResult, ConfigSaveError,
    HistoryPage, HistoryQuery, ToolInstallResult, ToolsStatus, TranslateRequest, TranslateResult,
};
use crate::scheduler::autostart;
use crate::state::{push_warning, AppState};

type CmdResult<T> = Result<T, String>;
type ConfigCmdResult = Result<ConfigResult, ConfigSaveError>;

#[tauri::command]
pub async fn get_app_snapshot(state: tauri::State<'_, AppState>) -> CmdResult<AppSnapshot> {
    let config = state.config();
    let view = state.view_config();
    // 备份恢复后的一次性提示：读取即删除，并并入 warnings 通道展示。
    let restore_notice = state.take_restore_notice();
    if let Some(notice) = restore_notice.clone() {
        push_warning(&state.warnings, notice);
    }
    // 密钥迁移 / 权限收紧的一次性提示，同样在首次快照展示。
    if let Some(notice) = state.secrets.take_migration_notice() {
        push_warning(&state.warnings, notice);
    }
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
        config_revision: state.config_revision(),
        restore_notice,
    })
}

/// 分段保存配置。`base_revision` 与磁盘版本不一致时返回结构化冲突，
/// 由前端在“保留我的 / 用磁盘的”之间选择，绝不静默整包覆盖。
#[tauri::command]
pub async fn save_config(
    app: AppHandle,
    config: AppConfig,
    secret_patch: Option<SecretPatch>,
    section: Option<String>,
    base_revision: Option<u64>,
    state: tauri::State<'_, AppState>,
) -> ConfigCmdResult {
    state.ensure_writable()?;
    let section = ConfigSection::parse(section.as_deref())?;

    let save_guard = state.config_save.lock();
    if let Some(base) = base_revision {
        let current = state.config_revision();
        if base != current {
            return Err(ConfigSaveError::Conflict(ConfigConflictError::new(
                state.view_config(),
                current,
            )));
        }
    }

    let current = state.config.read().clone();
    let mut merged = core::config::merge_section(&current, config, section);
    core::config::normalize(&mut merged, &state.paths);
    core::config::validate(&merged)?;

    let previous = state.config();
    // 新快捷键注册失败时旧快捷键已被注销，必须立即恢复，避免保存失败后快捷翻译不可用。
    if let Err(err) = core::quick::apply(&app, &merged) {
        let _ = core::quick::apply(&app, &previous);
        return Err(err.into());
    }

    let stored = (|| -> Result<(), String> {
        // Keys embedded in the payload (older UI) move into the secret store.
        state.secrets.migrate_inline(&mut merged)?;
        if let Some(patch) = secret_patch.as_ref() {
            state.secrets.apply_patch(patch)?;
        }
        core::config::save(&state.paths, &merged)
    })();

    if let Err(err) = stored {
        let _ = core::quick::apply(&app, &previous);
        return Err(err.into());
    }
    *state.config.write() = merged;
    let revision = state.bump_config_revision();
    drop(save_guard);

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
        config_revision: revision,
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
        crate::dialogs::pick_folder().map(|path| path.to_string_lossy().to_string())
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
