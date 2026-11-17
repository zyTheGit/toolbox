//! 凭据管理命令。

use tauri::State;

use crate::core::secrets::{SecretEntryInput, SecretEntryView};
use crate::state::AppState;

type CmdResult<T> = Result<T, String>;

#[tauri::command]
pub fn secrets_list(state: State<'_, AppState>) -> CmdResult<Vec<SecretEntryView>> {
    Ok(state.secrets.list())
}

#[tauri::command]
pub fn secret_save(
    input: SecretEntryInput,
    state: State<'_, AppState>,
) -> CmdResult<Vec<SecretEntryView>> {
    state.secrets.save_entry(input)?;
    Ok(state.secrets.list())
}

#[tauri::command]
pub fn secret_delete(id: String, state: State<'_, AppState>) -> CmdResult<Vec<SecretEntryView>> {
    state.secrets.delete_entry(&id)?;
    Ok(state.secrets.list())
}

/// 仅在用户二次确认后由前端调用，返回明文供复制。
#[tauri::command]
pub fn secret_reveal(id: String, state: State<'_, AppState>) -> CmdResult<String> {
    state.secrets.reveal(&id)
}
