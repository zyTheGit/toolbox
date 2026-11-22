use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use parking_lot::Mutex;
use tauri::AppHandle;
use uuid::Uuid;

use crate::core::config::expand_home;
use crate::core::persist;
use crate::models::{AppConfig, DownloadRequest, Job};
use crate::state::{push_warning, AppPaths, SharedConfig, Warnings};

mod args;
mod cleanup;
mod credentials;
mod naming;
mod parse;
mod retry;
mod state;
mod worker;

pub(crate) use self::args::redact_text;
use self::args::validate_extra_args;
use self::credentials::{adopt_loaded_jobs, strip_job, JobCredentials};
use self::naming::{job_name, normalize_subtitles, now_string, sanitize_file_name};
use self::state::{is_active, mark_interrupted};
use crate::core::proc::kill_tree;

#[cfg(test)]
mod cleanup_tests;
#[cfg(test)]
mod credentials_tests;
#[cfg(test)]
mod tests;

#[derive(Default)]
struct Inner {
    jobs: Vec<Job>,
    /// 仅进程内持有的下载凭据；绝不落盘，应用退出即丢失。
    credentials: HashMap<String, JobCredentials>,
    running: bool,
    cancel_ids: HashSet<String>,
    children: HashMap<String, u32>,
    last_emit: HashMap<String, Instant>,
    shutdown: bool,
}

#[derive(Clone)]
pub struct Downloader {
    app: Option<AppHandle>,
    paths: AppPaths,
    config: SharedConfig,
    warnings: Warnings,
    inner: Arc<Mutex<Inner>>,
    save_order: Arc<Mutex<()>>,
}

enum Finish {
    Completed,
    Canceled,
    Failed(String),
}

impl Downloader {
    pub fn new(
        app: AppHandle,
        paths: AppPaths,
        config: SharedConfig,
        warnings: Warnings,
    ) -> Self {
        Self::build(Some(app), paths, config, warnings)
    }

    /// 单元测试用构建器：没有 AppHandle，不发射事件、不启动 worker。
    #[cfg(test)]
    pub(super) fn for_test(
        paths: AppPaths,
        config: SharedConfig,
        warnings: Warnings,
    ) -> Self {
        Self::build(None, paths, config, warnings)
    }

    fn build(
        app: Option<AppHandle>,
        paths: AppPaths,
        config: SharedConfig,
        warnings: Warnings,
    ) -> Self {
        let mut jobs = match read_jobs(&paths.jobs_file) {
            Ok(Some(jobs)) => jobs,
            Ok(None) => Vec::new(),
            Err(err) => {
                push_warning(&warnings, err);
                Vec::new()
            }
        };

        // Restart policy: never resume downloads automatically. Queued and
        // running jobs become interrupted and keep their parameters for retry.
        // 旧版明文凭据在此迁入内存表，并立即原子写回已剥离的 jobs.json。
        let mut credentials = HashMap::new();
        let (changed, migrated) = adopt_loaded_jobs(&mut jobs, &mut credentials, &now_string());

        let downloader = Self {
            app,
            paths,
            config,
            warnings,
            inner: Arc::new(Mutex::new(Inner {
                jobs,
                credentials,
                ..Inner::default()
            })),
            save_order: Arc::new(Mutex::new(())),
        };
        if changed || migrated > 0 {
            downloader.save();
        }
        if migrated > 0 {
            push_warning(
                &downloader.warnings,
                format!(
                    "检测到 {migrated} 条下载记录含明文凭据，已迁入内存并从 jobs.json 清除；重启后重试需要重新填写凭据"
                ),
            );
        }
        downloader
    }

    pub fn enqueue(
        &self,
        request: &DownloadRequest,
        config: &AppConfig,
    ) -> Result<Vec<Job>, String> {
        let mut seen = HashSet::new();
        let mut urls = Vec::new();
        for raw in &request.urls {
            let url = raw.trim();
            if url.is_empty() {
                continue;
            }
            let Ok(parsed) = reqwest::Url::parse(url) else {
                continue;
            };
            if parsed.scheme() != "http" && parsed.scheme() != "https" {
                continue;
            }
            if !seen.insert(url.to_string()) {
                continue;
            }
            urls.push(url.to_string());
        }
        if urls.is_empty() {
            return Err("[PARAM] 请填写有效的下载地址（仅支持 http/https）".into());
        }

        let out_dir = match request
            .out_dir
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            Some(value) => expand_home(value),
            None => {
                let configured = config.download_dir.trim();
                if configured.is_empty() {
                    self.paths.downloads_dir.to_string_lossy().to_string()
                } else {
                    configured.to_string()
                }
            }
        };
        std::fs::create_dir_all(&out_dir)
            .map_err(|e| format!("[PATH] 创建下载目录失败: {e}"))?;

        let quality = if request.quality.trim().is_empty() {
            "best".to_string()
        } else {
            request.quality.trim().to_string()
        };
        let subtitles = normalize_subtitles(&request.subtitles);
        let subtitle_only = request.subtitle_only && !subtitles.is_empty(); // 未选字幕时“仅字幕”无意义
        // 凭据只进入内存表；Job 上的四个字段一律留空，避免落盘与事件外泄。
        let credentials = JobCredentials::from_request(request);
        validate_extra_args(&credentials.extra_args, config.security.allow_commands)?;
        let has_credentials = !credentials.is_empty();

        let stamp = now_string();
        let base_name = request
            .file_name
            .as_deref()
            .map(sanitize_file_name)
            .filter(|value| !value.is_empty());
        let threads = request
            .threads
            .unwrap_or(config.fragment_threads)
            .clamp(1, 16);
        let total = urls.len();

        let created = {
            let mut inner = self.inner.lock();
            let mut created = Vec::new();
            for (index, url) in urls.into_iter().enumerate() {
                let name = job_name(base_name.as_deref(), index, total);
                let job = Job {
                    id: Uuid::new_v4().to_string(),
                    url,
                    quality: quality.clone(),
                    out_dir: out_dir.clone(),
                    name,
                    threads,
                    referer: String::new(),
                    cookies_browser: String::new(),
                    proxy: String::new(),
                    extra_args: String::new(),
                    subtitles: subtitles.clone(),
                    subtitle_only,
                    status: "queued".into(),
                    created_at: stamp.clone(),
                    has_credentials,
                    ..Job::default()
                };
                if has_credentials {
                    inner
                        .credentials
                        .insert(job.id.clone(), credentials.clone());
                }
                created.push(job);
            }
            inner.jobs.splice(0..0, created.clone());
            created
        };

        for job in &created {
            self.emit_job(job);
        }
        self.save();
        self.ensure_worker();
        Ok(created)
    }

    pub fn cancel(&self, id: &str) -> Result<(), String> {
        let (updated, pid) = {
            let mut inner = self.inner.lock();
            let Inner {
                jobs,
                cancel_ids,
                children,
                ..
            } = &mut *inner;
            let Some(job) = jobs.iter_mut().find(|item| item.id == id) else {
                return Err("[CONFLICT] 任务不存在".into());
            };
            match job.status.as_str() {
                "queued" => {
                    job.status = "canceled".into();
                    job.error = "已取消".into();
                    job.finished_at = now_string();
                    let snapshot = job.clone();
                    (Some(snapshot), None)
                }
                "running" => {
                    cancel_ids.insert(id.to_string());
                    let pid = children.get(id).copied();
                    (None, pid)
                }
                _ => return Err("[CONFLICT] 任务已结束，无法取消".into()),
            }
        };

        if let Some(pid) = pid {
            kill_tree(pid);
        }
        if let Some(job) = updated {
            self.emit_job(&job);
            self.save();
        }
        Ok(())
    }

    /// 移除记录并清理该任务的未完成临时文件，返回删除的文件数。
    pub fn remove(&self, id: &str) -> Result<usize, String> {
        let snapshot = {
            let mut inner = self.inner.lock();
            let Some(index) = inner.jobs.iter().position(|item| item.id == id) else {
                return Err("[CONFLICT] 任务不存在".into());
            };
            if is_active(&inner.jobs[index]) {
                return Err("[CONFLICT] 请先取消任务".into());
            }
            let job = inner.jobs.remove(index);
            // 记录被移除后，内存凭据同步清理，避免残留。
            inner.credentials.remove(id);
            job
        };
        self.emit_removed(id);
        self.save();
        Ok(self.cleanup_temps_for(&snapshot))
    }

    pub fn open(&self, id: &str) -> Result<String, String> {
        let (file, out_dir) = {
            let inner = self.inner.lock();
            let Some(job) = inner.jobs.iter().find(|item| item.id == id) else {
                return Err("[CONFLICT] 任务不存在".into());
            };
            (job.file.clone(), job.out_dir.clone())
        };

        let target = if !file.trim().is_empty() {
            Path::new(&file)
                .parent()
                .map(Path::to_path_buf)
                .filter(|path| !path.as_os_str().is_empty())
                .unwrap_or_else(|| PathBuf::from(&out_dir))
        } else if !out_dir.trim().is_empty() {
            PathBuf::from(&out_dir)
        } else {
            self.paths.downloads_dir.clone()
        };

        if !target.exists() {
            std::fs::create_dir_all(&target).map_err(|e| format!("[PATH] 创建目录失败: {e}"))?;
        }
        open::that(&target).map_err(|e| format!("[PATH] 打开目录失败: {e}"))?;
        Ok(target.to_string_lossy().to_string())
    }

    pub fn list(&self) -> Vec<Job> {
        let mut jobs = self.inner.lock().jobs.clone();
        // 防御性剥离：列表 / 快照永不含凭据。
        for job in jobs.iter_mut() {
            strip_job(job);
        }
        jobs.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        jobs
    }

    /// 备份恢复后从磁盘重新载入下载记录（不自动继续下载）。
    ///
    /// `keep_stopped` 为 true 时保持停写状态（恢复成功后等待重启）；
    /// 为 false 时恢复正常调度（恢复失败回滚后）。
    pub fn reload_from_disk(&self, keep_stopped: bool) -> Result<(), String> {
        let mut jobs = read_jobs(&self.paths.jobs_file)?.unwrap_or_default();
        // 恢复的记录同样走“明文凭据迁入内存”的路径；写盘交由后续保存或下次启动处理，
        // 避免干扰备份恢复的回滚语义。
        let mut credentials = HashMap::new();
        adopt_loaded_jobs(&mut jobs, &mut credentials, &now_string());
        let mut inner = self.inner.lock();
        inner.jobs = jobs;
        inner.credentials = credentials;
        inner.cancel_ids.clear();
        inner.children.clear();
        inner.last_emit.clear();
        inner.running = false;
        inner.shutdown = keep_stopped;
        Ok(())
    }

    pub fn active_count(&self) -> usize {
        self.inner.lock().jobs.iter().filter(|job| is_active(job)).count()
    }

    pub fn shutdown(&self) {
        let (pids, updated) = {
            let mut inner = self.inner.lock();
            inner.shutdown = true;
            let pids: Vec<u32> = inner.children.values().copied().collect();
            inner.children.clear();
            inner.cancel_ids.clear();
            inner.last_emit.clear();
            let stamp = now_string();
            let mut updated = Vec::new();
            for job in inner.jobs.iter_mut() {
                if mark_interrupted(job, &stamp) {
                    updated.push(job.clone());
                }
            }
            (pids, updated)
        };

        for pid in pids {
            kill_tree(pid);
        }
        for job in &updated {
            self.emit_job(job);
        }
        self.save();
    }
}

/// 读取下载记录文件；`Ok(None)` 表示文件尚不存在。
///
/// 启动初始化与备份恢复后的重载共用这一入口，避免两套加载逻辑。
fn read_jobs(path: &Path) -> Result<Option<Vec<Job>>, String> {
    persist::load_json::<Vec<Job>>(path)
}
