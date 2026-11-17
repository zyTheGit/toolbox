use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use parking_lot::Mutex;
use tauri::AppHandle;
use uuid::Uuid;

use crate::core::config::expand_home;
use crate::core::persist;
use crate::models::{AppConfig, DownloadRequest, Job, JobProgress};
use crate::state::{push_warning, AppPaths, SharedConfig, Warnings};

mod args;
mod naming;
mod parse;
mod state;
mod worker;

pub(crate) use self::args::redact_text;
use self::args::validate_extra_args;
use self::naming::{job_name, now_string, sanitize_file_name};
use self::state::{is_active, mark_interrupted};
use crate::core::proc::kill_tree;

#[cfg(test)]
mod tests;

#[derive(Clone, Default)]
struct JobOptions {
    referer: String,
    cookies_browser: String,
    proxy: String,
    extra_args: String,
}

#[derive(Default)]
struct Inner {
    jobs: Vec<Job>,
    running: bool,
    cancel_ids: HashSet<String>,
    children: HashMap<String, u32>,
    last_emit: HashMap<String, Instant>,
    shutdown: bool,
}

#[derive(Clone)]
pub struct Downloader {
    app: AppHandle,
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
        let mut jobs = match persist::load_json::<Vec<Job>>(&paths.jobs_file) {
            Ok(Some(jobs)) => jobs,
            Ok(None) => Vec::new(),
            Err(err) => {
                push_warning(&warnings, err);
                Vec::new()
            }
        };

        // Restart policy: never resume downloads automatically. Queued and
        // running jobs become interrupted and keep their parameters for retry.
        let mut changed = false;
        let stamp = now_string();
        for job in jobs.iter_mut() {
            for line in job.log.iter_mut() {
                *line = redact_text(line);
            }
            if mark_interrupted(job, &stamp) {
                changed = true;
            }
        }

        let downloader = Self {
            app,
            paths,
            config,
            warnings,
            inner: Arc::new(Mutex::new(Inner {
                jobs,
                ..Inner::default()
            })),
            save_order: Arc::new(Mutex::new(())),
        };
        if changed {
            downloader.save();
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
        let referer = request.referer.trim().to_string();
        let cookies_browser = request.cookies_browser.trim().to_string();
        let proxy = request.proxy.trim().to_string();
        let extra_args = request.extra_args.trim().to_string();
        validate_extra_args(&extra_args, config.security.allow_commands)?;

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
                    referer: referer.clone(),
                    cookies_browser: cookies_browser.clone(),
                    proxy: proxy.clone(),
                    extra_args: extra_args.clone(),
                    status: "queued".into(),
                    progress: JobProgress::default(),
                    log: Vec::new(),
                    error: String::new(),
                    file: String::new(),
                    created_at: stamp.clone(),
                    started_at: String::new(),
                    finished_at: String::new(),
                };
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

    pub fn remove(&self, id: &str) -> Result<(), String> {
        {
            let mut inner = self.inner.lock();
            let Some(index) = inner.jobs.iter().position(|item| item.id == id) else {
                return Err("[CONFLICT] 任务不存在".into());
            };
            if is_active(&inner.jobs[index]) {
                return Err("[CONFLICT] 请先取消任务".into());
            }
            inner.jobs.remove(index);
        }
        self.emit_removed(id);
        self.save();
        Ok(())
    }

    /// Re-queue an interrupted or failed job with its original parameters.
    pub fn retry(&self, id: &str) -> Result<Job, String> {
        let job = {
            let mut inner = self.inner.lock();
            let Some(job) = inner.jobs.iter_mut().find(|item| item.id == id) else {
                return Err("[CONFLICT] 任务不存在".into());
            };
            if is_active(job) {
                return Err("[CONFLICT] 任务正在排队或运行，无需重试".into());
            }
            job.status = "queued".into();
            job.error.clear();
            job.file.clear();
            job.progress = JobProgress::default();
            job.log.clear();
            job.started_at.clear();
            job.finished_at.clear();
            job.clone()
        };
        self.emit_job(&job);
        self.save();
        self.ensure_worker();
        Ok(job)
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
        jobs.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        jobs
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
