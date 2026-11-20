//! 检查新版本与打开发布页。

use crate::core;
use crate::state::AppState;

type CmdResult<T> = Result<T, String>;

#[tauri::command]
pub async fn check_for_update(
    state: tauri::State<'_, AppState>,
) -> CmdResult<core::update::UpdateCheckDto> {
    let config = state.config();
    core::update::check(&config, env!("CARGO_PKG_VERSION")).await
}

/// 用系统浏览器打开发布页。地址虽由界面回传，仍在 Rust 侧按前缀校验，
/// 界面无法把“打开”指向任意 URL 或协议。
#[tauri::command]
pub async fn open_release_page(url: String) -> CmdResult<()> {
    core::update::ensure_release_page(&url)?;
    open::that(&url).map_err(|e| format!("[PATH] 打开浏览器失败: {e}"))
}
