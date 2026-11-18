//! 下载任务命令。

use crate::models::{DownloadRequest, Job, JobRetryRequest};
use crate::state::AppState;

type CmdResult<T> = Result<T, String>;

#[tauri::command]
pub fn download_enqueue(
    request: DownloadRequest,
    state: tauri::State<'_, AppState>,
) -> CmdResult<Vec<Job>> {
    state.ensure_writable()?;
    let config = state.config();
    state.downloader.enqueue(&request, &config)
}

#[tauri::command]
pub fn job_cancel(id: String, state: tauri::State<'_, AppState>) -> CmdResult<()> {
    state.ensure_writable()?;
    state.downloader.cancel(&id)
}

/// 重试任务。`request` 可补填重启后丢失的内存凭据；
/// 需要凭据但既无内存记录也未补填时返回 `[CREDENTIALS]`。
#[tauri::command]
pub fn job_retry(
    id: String,
    request: Option<JobRetryRequest>,
    state: tauri::State<'_, AppState>,
) -> CmdResult<Job> {
    state.ensure_writable()?;
    state.downloader.retry(&id, request)
}

#[tauri::command]
pub fn job_remove(id: String, state: tauri::State<'_, AppState>) -> CmdResult<()> {
    state.ensure_writable()?;
    state.downloader.remove(&id)
}

#[tauri::command]
pub fn job_open(id: String, state: tauri::State<'_, AppState>) -> CmdResult<String> {
    state.downloader.open(&id)
}
