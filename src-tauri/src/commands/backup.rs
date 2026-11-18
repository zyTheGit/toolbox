//! 备份导入 / 导出命令与重启入口。

use tauri::AppHandle;

use crate::core;
use crate::models::ImportOutcome;
use crate::state::AppState;

type CmdResult<T> = Result<T, String>;

/// 导出备份。`include_commands` 默认 false：命令、环境变量、输出与凭据均脱敏。
#[tauri::command]
pub async fn data_export(
    include_commands: bool,
    state: tauri::State<'_, AppState>,
) -> CmdResult<String> {
    let picked = tauri::async_runtime::spawn_blocking(|| {
        let name = format!(
            "toolbox-backup-{}.json",
            chrono::Local::now().format("%Y%m%d-%H%M%S")
        );
        crate::dialogs::save_file("导出 Toolbox 备份", &name)
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
    core::backup::export_to(&path, &state, include_commands)
}

/// 导入备份并返回恢复结果；不再自动重启，由前端“重启应用”按钮调用
/// `restart_app`。用户取消时返回全零结果（`backupPath` 为空）。
#[tauri::command]
pub async fn data_import(state: tauri::State<'_, AppState>) -> CmdResult<ImportOutcome> {
    let picked = tauri::async_runtime::spawn_blocking(|| {
        crate::dialogs::pick_file("选择 Toolbox 备份")
    })
    .await
    .map_err(|e| format!("[TOOL] 打开文件选择器失败: {e}"))?;
    let Some(path) = picked else {
        return Ok(ImportOutcome::canceled());
    };

    core::backup::import_from(&path, &state)
}

/// 用户确认后重启应用（备份恢复完成后生效）。
#[tauri::command]
pub fn restart_app(app: AppHandle) -> CmdResult<()> {
    app.restart()
}
