use std::str::FromStr;

use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

use crate::models::AppConfig;

/// Register (or clear) the optional global quick-translate shortcut. The
/// clipboard is only used as a preview: the frontend must never translate
/// automatically, which keeps stale or sensitive clipboard content under the
/// user's control.
pub fn apply(app: &AppHandle, config: &AppConfig) -> Result<(), String> {
    let manager = app.global_shortcut();
    let _ = manager.unregister_all();

    if !config.quick_translate.enabled {
        return Ok(());
    }

    let text = config.quick_translate.shortcut.trim();
    if text.is_empty() {
        return Err("[PARAM] 启用快捷翻译时需要填写快捷键".into());
    }
    let shortcut =
        Shortcut::from_str(text).map_err(|err| format!("[PARAM] 快捷键格式无效：{err}"))?;
    let handle = app.clone();
    manager
        .on_shortcut(shortcut, move |_app, _shortcut, event| {
            if event.state() != ShortcutState::Pressed {
                return;
            }
            let Ok(value) = handle.clipboard().read_text() else {
                return;
            };
            let preview: String = value.trim().chars().take(4000).collect();
            if preview.is_empty() {
                return;
            }
            if let Some(window) = handle.get_webview_window("main") {
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
            let _ = handle.emit("quick-translate", preview);
        })
        .map_err(|err| format!("[CONFLICT] 注册快捷键失败（可能被其他程序占用）：{err}"))
}