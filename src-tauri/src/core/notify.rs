use std::collections::HashMap;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::OnceLock;

use parking_lot::Mutex;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_notification::NotificationExt;

#[derive(Clone)]
struct Target {
    page: &'static str,
    task_id: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct NavPayload {
    page: &'static str,
    task_id: Option<String>,
}

static NEXT_ID: AtomicI32 = AtomicI32::new(1);
static TARGETS: OnceLock<Mutex<HashMap<i32, Target>>> = OnceLock::new();

fn targets() -> &'static Mutex<HashMap<i32, Target>> {
    TARGETS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Notify only while the main window is hidden or minimized, so foreground use
/// stays quiet. Notification text is intentionally generic: no URLs, commands,
/// keys or full paths.
pub fn notify(
    app: &AppHandle,
    page: &'static str,
    task_id: Option<&str>,
    title: &str,
    body: &str,
) {
    if !window_hidden(app) {
        return;
    }
    let _ = show(app, page, task_id, title, body);
}

/// 提醒类通知前台也要展示；点击后定位到对应任务。
pub fn notify_task(app: &AppHandle, task_id: &str, title: &str, body: &str) -> Result<(), String> {
    show(app, "tasks", Some(task_id), title, body)
}

fn show(
    app: &AppHandle,
    page: &'static str,
    task_id: Option<&str>,
    title: &str,
    body: &str,
) -> Result<(), String> {
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    {
        let mut map = targets().lock();
        if map.len() >= 64 {
            map.clear();
        }
        map.insert(
            id,
            Target {
                page,
                task_id: task_id.map(str::to_string),
            },
        );
    }
    app.notification()
        .builder()
        .id(id)
        .title(title)
        .body(body)
        .show()
        .map_err(|err| err.to_string())
}

fn window_hidden(app: &AppHandle) -> bool {
    match app.get_webview_window("main") {
        Some(window) => {
            let visible = window.is_visible().unwrap_or(true);
            let minimized = window.is_minimized().unwrap_or(false);
            !visible || minimized
        }
        None => true,
    }
}

pub fn handle_tap(app: &AppHandle, id: Option<i32>) {
    let target = id
        .and_then(|id| targets().lock().remove(&id))
        .unwrap_or(Target {
            page: "translate",
            task_id: None,
        });
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
    let _ = app.emit(
        "navigate",
        NavPayload {
            page: target.page,
            task_id: target.task_id,
        },
    );
}
