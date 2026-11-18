use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
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
    pub restore_notice_file: PathBuf,
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
            restore_notice_file: data_dir.join("restore-notice.txt"),
            data_dir,
        }
    }

    pub fn ensure_dirs(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.data_dir)?;
        std::fs::create_dir_all(&self.bin_dir)?;
        std::fs::create_dir_all(&self.runs_dir)?;
        self.restrict_dir_permissions()?;
        Ok(())
    }

    /// Unix：数据目录与子目录收紧为 0700（已存在的目录同样收紧，
    /// 不能只依赖首次创建时的 umask）；Windows 继续使用 DPAPI。
    #[cfg(unix)]
    fn restrict_dir_permissions(&self) -> std::io::Result<()> {
        use std::os::unix::fs::PermissionsExt;

        for dir in [&self.data_dir, &self.bin_dir, &self.runs_dir] {
            let mode = std::fs::metadata(dir)?.permissions().mode() & 0o777;
            if mode != 0o700 {
                std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))?;
            }
        }
        Ok(())
    }

    #[cfg(not(unix))]
    fn restrict_dir_permissions(&self) -> std::io::Result<()> {
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
    /// 备份恢复闸门：恢复期间与恢复成功后（等待重启）保持只读。
    pub restoring: AtomicBool,
    /// 配置版本号：每次成功保存自增，用于分段保存的期望版本校验。
    pub config_revision: AtomicU64,
    /// 串行化配置保存，保证“校验版本 → 合并 → 落盘 → 回写内存”是原子的。
    pub config_save: Mutex<()>,
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

    pub fn config_revision(&self) -> u64 {
        self.config_revision.load(Ordering::SeqCst)
    }

    pub fn bump_config_revision(&self) -> u64 {
        self.config_revision.fetch_add(1, Ordering::SeqCst) + 1
    }

    /// 进入备份恢复：同一时间只允许一个恢复流程。
    pub fn begin_restore(&self) -> Result<(), String> {
        if self.restoring.swap(true, Ordering::SeqCst) {
            return Err("[CONFLICT] 已有备份恢复任务正在执行，请稍候".into());
        }
        Ok(())
    }

    /// 恢复失败或被取消后解除只读；成功时保持到应用重启。
    pub fn end_restore(&self) {
        self.restoring.store(false, Ordering::SeqCst);
    }

    pub fn is_restoring(&self) -> bool {
        self.restoring.load(Ordering::SeqCst)
    }

    /// 写命令的统一闸门：恢复期间与恢复后（未重启）一律拒绝写入。
    pub fn ensure_writable(&self) -> Result<(), String> {
        if self.is_restoring() {
            return Err("[CONFLICT] 备份恢复期间数据只读，请重启应用后再修改".into());
        }
        Ok(())
    }

    /// 读取并删除一次性的备份恢复提示（启动后首次快照展示）。
    pub fn take_restore_notice(&self) -> Option<String> {
        let path = &self.paths.restore_notice_file;
        let text = std::fs::read_to_string(path).ok()?;
        let _ = std::fs::remove_file(path);
        let text = text.trim().to_string();
        if text.is_empty() {
            None
        } else {
            Some(text)
        }
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    fn mode(path: &PathBuf) -> u32 {
        std::fs::metadata(path).unwrap().permissions().mode() & 0o777
    }

    #[test]
    fn data_dirs_are_private_even_when_preexisting() {
        let dir = std::env::temp_dir().join(format!("toolbox-paths-{}", uuid::Uuid::new_v4()));
        let paths = AppPaths::from_data_dir(dir.clone());
        paths.ensure_dirs().unwrap();

        // 模拟旧版本创建的 0755 目录：再次 ensure_dirs 时必须收紧。
        for sub in [&dir, &paths.bin_dir, &paths.runs_dir] {
            std::fs::set_permissions(sub, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        paths.ensure_dirs().unwrap();

        assert_eq!(mode(&dir), 0o700);
        assert_eq!(mode(&paths.bin_dir), 0o700);
        assert_eq!(mode(&paths.runs_dir), 0o700);
        let _ = std::fs::remove_dir_all(dir);
    }
}
