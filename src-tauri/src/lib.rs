mod commands;
#[doc(hidden)]
pub mod core;
#[doc(hidden)]
pub mod models;
mod scheduler;
mod state;

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use parking_lot::{Mutex, RwLock};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, WindowEvent};
use tauri_plugin_notification::NotificationExt;

use models::AppConfig;
use state::{AppPaths, AppState, SharedConfig};

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_main(app);
        }))
        .setup(|app| {
            let paths = AppPaths::resolve(app.handle())?;
            paths.ensure_dirs()?;

            let warnings: state::Warnings = Arc::new(Mutex::new(Vec::new()));

            let (mut config, config_warning) = core::config::load(&paths);
            if let Some(message) = config_warning {
                state::push_warning(&warnings, message);
            }
            core::config::normalize(&mut config, &paths);

            let (secrets, secret_warning) = core::secrets::SecretStore::load(paths.secrets_file.clone());
            if let Some(message) = secret_warning {
                state::push_warning(&warnings, message);
            }
            if let Err(err) = secrets.migrate_inline(&mut config) {
                state::push_warning(&warnings, format!("迁移旧密钥失败: {err}"));
            }
            for message in secrets.expiring_warnings() {
                state::push_warning(&warnings, message);
            }

            let shared: SharedConfig = Arc::new(RwLock::new(config.clone()));
            let (history, history_warning) =
                core::history::HistoryStore::new(paths.history_file.clone());
            if let Some(message) = history_warning {
                state::push_warning(&warnings, message);
            }
            let history = Arc::new(history);
            let downloader = Arc::new(core::downloader::Downloader::new(
                app.handle().clone(),
                paths.clone(),
                shared.clone(),
                warnings.clone(),
            ));
            let scheduler = Arc::new(scheduler::Scheduler::new(
                app.handle().clone(),
                paths.clone(),
                shared.clone(),
                warnings.clone(),
            ));

            app.manage(AppState {
                paths,
                config: shared,
                secrets: Arc::new(secrets),
                warnings,
                history,
                downloader,
                scheduler,
                installing: Mutex::new(HashSet::new()),
            });

            let state = app.state::<AppState>();
            state.scheduler.start();

            if let Err(err) = core::quick::apply(app.handle(), &state.config()) {
                state::push_warning(&state.warnings, format!("快捷翻译不可用: {err}"));
            }

            let notify_handle = app.handle().clone();
            if let Err(err) = app.notification().on_action(move |performed| {
                if performed.action_id() == "tap" {
                    let id = performed.notification().map(|item| item.id());
                    core::notify::handle_tap(&notify_handle, id);
                }
            }) {
                eprintln!("register notification action handler failed: {err}");
            }

            build_tray(app.handle())?;
            apply_startup_visibility(app.handle(), &config);

            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_app_snapshot,
            commands::save_config,
            commands::get_tools,
            commands::install_tool,
            commands::open_folder,
            commands::pick_folder,
            commands::translate,
            commands::history_query,
            commands::history_delete,
            commands::history_clear,
            commands::history_set_favorite,
            commands::download_enqueue,
            commands::job_cancel,
            commands::job_retry,
            commands::job_remove,
            commands::job_open,
            commands::tasks_list,
            commands::task_create,
            commands::task_update,
            commands::task_delete,
            commands::task_run_now,
            commands::task_cancel,
            commands::task_runs,
            commands::tasks_set_paused,
            commands::cron_preview,
            commands::autostart_get,
            commands::autostart_set,
            commands::get_data_dir,
            commands::data_export,
            commands::data_import,
            commands::secrets::secrets_list,
            commands::secrets::secret_save,
            commands::secrets::secret_delete,
            commands::secrets::secret_reveal
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn build_tray(app: &AppHandle) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "打开主界面", true, None::<&str>)?;
    let pause = MenuItem::with_id(
        app,
        "toggle-pause",
        "暂停 / 恢复全部定时任务",
        true,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&show, &pause, &separator, &quit])?;

    let mut builder = TrayIconBuilder::with_id("main-tray")
        .tooltip("Toolbox")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => show_main(app),
            "toggle-pause" => {
                let state = app.state::<AppState>();
                let paused = state.scheduler.set_paused(!state.scheduler.paused());
                let _ = app.emit("tasks-paused", paused);
            }
            "quit" => quit_app(app),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main(tray.app_handle());
            }
        });

    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}

fn show_main(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

fn quit_app(app: &AppHandle) {
    let state = app.state::<AppState>();
    let active = state.downloader.active_count() + state.scheduler.active_count();
    if active > 0 {
        let confirmed = rfd::MessageDialog::new()
            .set_level(rfd::MessageLevel::Warning)
            .set_title("退出 Toolbox")
            .set_description(format!("有 {active} 个任务正在执行，退出会终止它们。确定退出？"))
            .set_buttons(rfd::MessageButtons::YesNo)
            .show();
        if !matches!(confirmed, rfd::MessageDialogResult::Yes) {
            return;
        }
    }
    state.scheduler.shutdown();
    state.downloader.shutdown();
    app.exit(0);
}

fn apply_startup_visibility(app: &AppHandle, config: &AppConfig) {
    let auto_started = std::env::args().any(|arg| arg == "--autostart");
    if !auto_started {
        show_main(app);
        return;
    }
    if config.startup.start_minimized {
        return;
    }
    let delay = config.startup.delay_seconds;
    if delay == 0 {
        show_main(app);
        return;
    }
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(delay as u64));
        show_main(&app);
    });
}
