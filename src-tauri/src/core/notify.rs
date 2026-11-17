use std::collections::HashMap;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::OnceLock;

use parking_lot::Mutex;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_notification::NotificationExt;

static NEXT_ID: AtomicI32 = AtomicI32::new(1);
static TARGETS: OnceLock<Mutex<HashMap<i32, &'static str>>> = OnceLock::new();

fn targets() -> &'static Mutex<HashMap<i32, &'static str>> {
    TARGETS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Notify only while the main window is hidden or minimized, so foreground use
/// stays quiet. Notification text is intentionally generic: no URLs, commands,
/// keys or full paths.
pub fn notify(app: &AppHandle, page: &'static str, title: &str, body: &str) {
    if !window_hidden(app) {
        return;
    }
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    {
        let mut map = targets().lock();
        if map.len() >= 64 {
            map.clear();
        }
        map.insert(id, page);
    }
    let _ = app
        .notification()
        .builder()
        .id(id)
        .title(title)
        .body(body)
        .show();
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
    let page = id
        .and_then(|id| targets().lock().remove(&id))
        .unwrap_or("translate");
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
    let _ = app.emit("navigate", page);
}