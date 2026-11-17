use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

use parking_lot::{Mutex, RwLock};
use tauri::{AppHandle, Manager};

use crate::core::db::DbManager;
use crate::core::downloader::Downloader;
use crate::core::history::HistoryStore;
use crate::core::llm::LlmUsageStore;
use crate::core::secrets::SecretStore;
use crate::models::AppConfig;
use crate::scheduler::Scheduler;

pub type SharedConfig = Arc<RwLock<AppConfig>>;
pub type Warnings = Arc<Mutex<Vec<String>>>;

pub fn push_warning(warnings: &Warnings, message: impl Into<String>) {
    let message = message.into();
    let mut warnings = warnings.lock();
    if !warnings.contains(&message) {
        warnings.push(message);
        let excess = warnings.len().saturating_sub(20);
        if excess > 0 {
            warnings.drain(0..excess);
        }
    }
}

#[derive(Clone, Debug)]
pub struct AppPaths {
    pub data_dir: PathBuf,
    pub bin_dir: PathBuf,
    pub downloads_dir: PathBuf,
    pub config_file: PathBuf,
    pub history_file: PathBuf,
    pub jobs_file: PathBuf,
    pub tasks_file: PathBuf,
    pub secrets_file: PathBuf,
    pub runs_dir: PathBuf,
    pub llm_usage_file: PathBuf,
}

impl AppPaths {
    pub fn resolve(app: &AppHandle) -> tauri::Result<Self> {
        let data_dir = app.path().data_dir()?.join("Toolbox");
        Ok(Self::from_data_dir(data_dir))
    }

    pub fn from_data_dir(data_dir: PathBuf) -> Self {
        Self {
            bin_dir: data_dir.join("bin"),
            downloads_dir: data_dir.join("downloads"),
            config_file: data_dir.join("config.json"),
            history_file: data_dir.join("history.json"),
            jobs_file: data_dir.join("jobs.json"),
            tasks_file: data_dir.join("tasks.json"),
            secrets_file: data_dir.join("secrets.bin"),
            runs_dir: data_dir.join("runs"),
            llm_usage_file: data_dir.join("llm-usage.json"),
            data_dir,
        }
    }

    pub fn ensure_dirs(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.data_dir)?;
        std::fs::create_dir_all(&self.bin_dir)?;
        std::fs::create_dir_all(&self.runs_dir)?;
        Ok(())
    }
}

pub struct AppState {
    pub paths: AppPaths,
    pub config: SharedConfig,
    pub secrets: Arc<SecretStore>,
    pub llm_usage: Arc<LlmUsageStore>,
    pub db: Arc<DbManager>,
    pub warnings: Warnings,
    pub history: Arc<HistoryStore>,
    pub downloader: Arc<Downloader>,
    pub scheduler: Arc<Scheduler>,
    pub installing: Mutex<HashSet<String>>,
}

impl AppState {
    /// Realized config (secrets included) for internal use only.
    pub fn config(&self) -> AppConfig {
        let mut config = self.config.read().clone();
        self.secrets.apply_to(&mut config);
        config
    }

    /// Masked config safe to hand to the WebView.
    pub fn view_config(&self) -> AppConfig {
        let mut config = self.config.read().clone();
        crate::core::secrets::mask_config(&mut config);
        config
    }

    pub fn warnings(&self) -> Vec<String> {
        self.warnings.lock().clone()
    }
}
