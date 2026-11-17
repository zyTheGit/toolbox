//! LLM 调试台命令。

use tauri::State;

use crate::core;
use crate::models::{LlmRequest, LlmResponse, LlmUsageSummary};
use crate::state::{push_warning, AppState};

type CmdResult<T> = Result<T, String>;

#[tauri::command]
pub async fn llm_run(
    request: LlmRequest,
    state: State<'_, AppState>,
) -> CmdResult<Vec<LlmResponse>> {
    let config = state.config();
    core::llm::validate_request(&request)?;
    let responses = core::llm::run(&request, &config, &state.secrets).await;
    if let Err(err) = state.llm_usage.record(&responses) {
        push_warning(&state.warnings, err);
    }
    Ok(responses)
}

#[tauri::command]
pub fn llm_usage(days: Option<u32>, state: State<'_, AppState>) -> CmdResult<LlmUsageSummary> {
    Ok(state.llm_usage.summary(days.unwrap_or(7)))
}
