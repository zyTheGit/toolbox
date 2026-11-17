use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};

use parking_lot::Mutex;
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use uuid::Uuid;

use crate::core::config::expand_home;
use crate::core::persist;
use crate::core::tools;
use crate::models::{AppConfig, DownloadRequest, Job, JobProgress, ToolStatus};
use crate::state::{push_warning, AppPaths, SharedConfig, Warnings};

const MAX_LOG_LINES: usize = 400;
const KEEP_TERMINAL: usize = 100;
const EMIT_INTERVAL: Duration = Duration::from_millis(500);
const TIME_FORMAT: &str = "%Y-%m-%d %H:%M:%S";
const NO_WINDOW: u32 = 0x0800_0000;
const MEDIA_EXTS: [&str; 14] = [
    "mp4", "mkv", "webm", "ts", "m4v", "mov", "flv", "avi", "mp3", "m4a", "aac", "opus", "ogg",
    "wav",
];

/// yt-dlp options that can start another process or load extra code. They are
/// only allowed when the command-execution switch is on.
const DANGEROUS_ARGS: [&str; 9] = [
    "--exec",
    "--exec-before-download",
    "--config-location",
    "--plugin-dirs",
    "--load-plugins",
    "--downloader",
    "--external-downloader",
    "--update",
    "--update-to",
];

const SECRET_VALUE_ARGS: [&str; 3] = ["--password", "--username", "--video-password"];

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

    fn ensure_worker(&self) {
        {
            let mut inner = self.inner.lock();
            if inner.running || inner.shutdown {
                return;
            }
            inner.running = true;
        }
        let worker = self.clone();
        tauri::async_runtime::spawn(async move {
            worker.worker_loop().await;
        });
    }

    async fn worker_loop(self) {
        let mut running = tokio::task::JoinSet::new();
        loop {
            let limit = self.concurrency_limit();
            while running.len() < limit {
                let Some(job) = self.claim_queued() else {
                    break;
                };
                self.emit_job(&job);
                self.save();
                let worker = self.clone();
                let id = job.id.clone();
                running.spawn(async move {
                    worker.run_job(&id).await;
                });
            }

            if running.is_empty() {
                let mut inner = self.inner.lock();
                if inner.shutdown {
                    inner.running = false;
                    return;
                }
                if inner.jobs.iter().any(|job| job.status == "queued") {
                    drop(inner);
                    continue;
                }
                inner.running = false;
                return;
            }

            tokio::select! {
                _ = running.join_next() => {}
                _ = tokio::time::sleep(Duration::from_millis(400)) => {}
            }
        }
    }

    fn claim_queued(&self) -> Option<Job> {
        let mut inner = self.inner.lock();
        if inner.shutdown {
            return None;
        }
        let job = inner.jobs.iter_mut().find(|job| job.status == "queued")?;
        job.status = "running".into();
        job.started_at = now_string();
        job.finished_at.clear();
        job.error.clear();
        job.file.clear();
        job.progress = JobProgress::default();
        job.log.clear();
        let snapshot = job.clone();
        inner.last_emit.insert(snapshot.id.clone(), Instant::now());
        Some(snapshot)
    }

    fn concurrency_limit(&self) -> usize {
        self.config.read().download_concurrency.clamp(1, 5) as usize
    }

    async fn run_job(&self, id: &str) {
        let config = self.config.read().clone();
        let paths = self.paths.clone();
        let for_tools = config.clone();
        let (ytdlp, ffmpeg): (ToolStatus, ToolStatus) =
            tauri::async_runtime::spawn_blocking(move || {
                let status = tools::get_status(&for_tools, &paths, false);
                (status.ytdlp, status.ffmpeg)
            })
            .await
            .unwrap_or_default();

        if !ytdlp.ok || ytdlp.path.trim().is_empty() {
            self.fail_pending(id, "[TOOL] 未找到 yt-dlp，请先在设置中安装");
            return;
        }

        let Some(job) = self.job_snapshot(id) else {
            return;
        };
        let options = JobOptions {
            referer: job.referer.clone(),
            cookies_browser: job.cookies_browser.clone(),
            proxy: job.proxy.clone(),
            extra_args: job.extra_args.clone(),
        };

        if let Err(err) = validate_extra_args(&options.extra_args, config.security.allow_commands) {
            self.finish_job(id, Finish::Failed(err));
            return;
        }

        if let Err(err) = std::fs::create_dir_all(&job.out_dir) {
            self.finish_job(id, Finish::Failed(format!("[PATH] 输出目录不可用: {err}")));
            return;
        }

        let output_template = output_template(&job);
        let threads = effective_threads(&job, &config);
        let mut args: Vec<String> = vec![
            "--ignore-config".into(),
            "--newline".into(),
            "--no-mtime".into(),
            "--windows-filenames".into(),
            "--concurrent-fragments".into(),
            threads.to_string(),
            "--retries".into(),
            "5".into(),
            "--fragment-retries".into(),
            "10".into(),
            "-o".into(),
            output_template,
            "-f".into(),
            format_for(&job.quality).into(),
        ];

        if ffmpeg.ok && !ffmpeg.path.trim().is_empty() {
            if let Some(parent) = Path::new(&ffmpeg.path).parent() {
                args.push("--ffmpeg-location".into());
                args.push(parent.to_string_lossy().to_string());
            }
        }
        if !options.referer.is_empty() {
            args.push("--referer".into());
            args.push(options.referer.clone());
        }
        if !options.cookies_browser.is_empty() {
            args.push("--cookies-from-browser".into());
            args.push(options.cookies_browser.clone());
        }
        let proxy = if options.proxy.is_empty() {
            config.proxy.trim().to_string()
        } else {
            options.proxy.clone()
        };
        if !proxy.is_empty() {
            args.push("--proxy".into());
            args.push(proxy);
        }
        if !options.extra_args.is_empty() {
            args.extend(split_args(&options.extra_args));
        }
        args.push(job.url.clone());

        let mut command = Command::new(&ytdlp.path);
        command
            .args(&args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(windows)]
        {
            command.creation_flags(NO_WINDOW);
        }
        #[cfg(unix)]
        {
            command.process_group(0);
        }

        let mut child = match command.spawn() {
            Ok(child) => child,
            Err(err) => {
                self.finish_job(id, Finish::Failed(format!("[TOOL] 启动 yt-dlp 失败: {err}")));
                return;
            }
        };

        if let Some(pid) = child.id() {
            let mut inner = self.inner.lock();
            inner.children.insert(id.to_string(), pid);
            if inner.cancel_ids.contains(id) {
                kill_tree(pid);
            }
        }

        let rendered = render_redacted(&args);
        self.apply_line(id, &format!("$ yt-dlp {rendered}"));

        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        let (_, _, wait_result) = tokio::join!(
            read_lines(stdout, self.clone(), id.to_string()),
            read_lines(stderr, self.clone(), id.to_string()),
            child.wait(),
        );

        let canceled = self.inner.lock().cancel_ids.contains(id);
        if canceled {
            self.finish_job(id, Finish::Canceled);
            return;
        }

        match wait_result {
            Ok(status) if status.success() => self.finish_job(id, Finish::Completed),
            Ok(status) => {
                let code = status.code();
                let message = self
                    .job_snapshot(id)
                    .map(|job| failure_message(&job, code))
                    .unwrap_or_else(|| "yt-dlp 下载失败".into());
                self.finish_job(id, Finish::Failed(tag_failure(message)));
            }
            Err(err) => {
                self.finish_job(id, Finish::Failed(format!("[TOOL] yt-dlp 运行失败: {err}")));
            }
        }
    }

    fn apply_line(&self, id: &str, line: &str) {
        let snapshot = {
            let mut inner = self.inner.lock();
            let Inner { jobs, last_emit, .. } = &mut *inner;
            let Some(job) = jobs.iter_mut().find(|item| item.id == id) else {
                return;
            };
            if job.status != "running" {
                return;
            }

            if job.log.len() >= MAX_LOG_LINES {
                let excess = job.log.len() + 1 - MAX_LOG_LINES;
                job.log.drain(0..excess);
            }
            job.log.push(redact_text(line));

            if let Some(progress) = parse_progress(line) {
                job.progress = progress;
            } else if let Some(file) = parse_file(line) {
                job.file = file;
            }

            let stamp = Instant::now();
            if let Some(previous) = last_emit.get(id) {
                if stamp.duration_since(*previous) < EMIT_INTERVAL {
                    return;
                }
            }
            last_emit.insert(id.to_string(), stamp);
            job.clone()
        };
        self.emit_job(&snapshot);
    }

    fn finish_job(&self, id: &str, outcome: Finish) {
        let mut snapshot = {
            let mut inner = self.inner.lock();
            let Inner {
                jobs,
                cancel_ids,
                children,
                last_emit,
                ..
            } = &mut *inner;
            children.remove(id);
            cancel_ids.remove(id);
            last_emit.remove(id);

            let Some(job) = jobs.iter_mut().find(|item| item.id == id) else {
                return;
            };
            if job.status != "running" {
                return;
            }

            match outcome {
                Finish::Completed => {
                    job.status = "completed".into();
                    job.progress.percent = 100.0;
                    job.progress.speed.clear();
                    job.progress.eta.clear();
                }
                Finish::Canceled => {
                    job.status = "canceled".into();
                    job.error = "已取消".into();
                }
                Finish::Failed(message) => {
                    job.status = "failed".into();
                    job.error = message;
                }
            }
            job.finished_at = now_string();
            job.clone()
        };

        if snapshot.status == "completed" && snapshot.file.trim().is_empty() {
            snapshot.file = scan_output_file(&snapshot);
            let mut inner = self.inner.lock();
            if let Some(job) = inner.jobs.iter_mut().find(|item| item.id == id) {
                job.file = snapshot.file.clone();
            }
        }

        self.emit_job(&snapshot);
        self.save();

        let notify_enabled = self.config.read().notify.download;
        if notify_enabled && matches!(snapshot.status.as_str(), "completed" | "failed") {
            let label = if snapshot.name.trim().is_empty() {
                std::path::Path::new(&snapshot.file)
                    .file_name()
                    .map(|value| value.to_string_lossy().to_string())
                    .unwrap_or_else(|| "m3u8 任务".to_string())
            } else {
                snapshot.name.trim().to_string()
            };
            let label: String = label.chars().take(60).collect();
            let label = if label.is_empty() {
                "m3u8 任务".to_string()
            } else {
                label
            };
            if snapshot.status == "completed" {
                crate::core::notify::notify(&self.app, "download", "下载完成", &label);
            } else {
                crate::core::notify::notify(
                    &self.app,
                    "download",
                    "下载失败",
                    &format!("{label}：点击查看原因"),
                );
            }
        }
    }

    fn fail_pending(&self, current_id: &str, message: &str) {
        let updated = {
            let mut inner = self.inner.lock();
            let stamp = now_string();
            let mut updated = Vec::new();
            for job in inner.jobs.iter_mut() {
                let target = job.id == current_id || job.status == "queued";
                let active = job.status == "queued" || job.status == "running";
                if target && active {
                    job.status = "failed".into();
                    job.error = message.to_string();
                    job.finished_at = stamp.clone();
                    updated.push(job.clone());
                }
            }
            inner.cancel_ids.remove(current_id);
            inner.last_emit.remove(current_id);
            inner.children.remove(current_id);
            updated
        };

        for job in &updated {
            self.emit_job(job);
        }
        self.save();
    }

    fn job_snapshot(&self, id: &str) -> Option<Job> {
        self.inner
            .lock()
            .jobs
            .iter()
            .find(|item| item.id == id)
            .cloned()
    }

    fn emit_job(&self, job: &Job) {
        let _ = self.app.emit("job-updated", job);
    }

    fn emit_removed(&self, id: &str) {
        let _ = self.app.emit("job-removed", id.to_string());
    }

    fn save(&self) {
        let _order = self.save_order.lock();
        let jobs = self.inner.lock().jobs.clone();
        self.persist(&jobs);
    }

    fn persist(&self, jobs: &[Job]) {
        let mut terminal_kept = 0usize;
        let kept: Vec<&Job> = jobs
            .iter()
            .filter(|job| {
                if is_active(job) {
                    return true;
                }
                if terminal_kept < KEEP_TERMINAL {
                    terminal_kept += 1;
                    true
                } else {
                    false
                }
            })
            .collect();

        if let Err(err) = persist::save_json(&self.paths.jobs_file, &kept) {
            push_warning(&self.warnings, format!("保存下载任务失败: {err}"));
        }
    }
}

fn is_active(job: &Job) -> bool {
    job.status == "queued" || job.status == "running"
}

/// Restart policy: queued/running jobs are never resumed silently; they become
/// interrupted and keep their parameters so the user can retry.
fn mark_interrupted(job: &mut Job, stamp: &str) -> bool {
    if !is_active(job) {
        return false;
    }
    job.status = "interrupted".into();
    job.error = "应用退出，任务已中断，可重试".into();
    job.finished_at = stamp.to_string();
    job.progress.speed.clear();
    job.progress.eta.clear();
    true
}

fn validate_extra_args(extra: &str, allow_commands: bool) -> Result<(), String> {
    if extra.trim().is_empty() || allow_commands {
        return Ok(());
    }
    for token in split_args(extra) {
        let flag = token.split('=').next().unwrap_or(token.as_str()).trim();
        if DANGEROUS_ARGS
            .iter()
            .any(|item| item.eq_ignore_ascii_case(flag))
        {
            return Err(format!(
                "[PARAM] 附加参数包含需要开启命令执行的安全选项：{flag}"
            ));
        }
    }
    Ok(())
}

fn render_redacted(args: &[String]) -> String {
    let mut out: Vec<String> = Vec::with_capacity(args.len());
    let mut mask_next = false;
    let mut header_next = false;

    for arg in args {
        let (flag, inline) = split_flag(arg);
        if mask_next {
            out.push("***".into());
            mask_next = false;
            continue;
        }
        if header_next {
            out.push(redact_header(arg));
            header_next = false;
            continue;
        }
        if is_header_flag(flag) {
            match inline {
                Some(value) => out.push(format!("{flag}={}", redact_header(value))),
                None => {
                    out.push(arg.clone());
                    header_next = true;
                }
            }
            continue;
        }
        if SECRET_VALUE_ARGS
            .iter()
            .any(|item| item.eq_ignore_ascii_case(flag))
        {
            match inline {
                Some(_) => out.push(format!("{flag}=***")),
                None => {
                    out.push(arg.clone());
                    mask_next = true;
                }
            }
            continue;
        }
        out.push(redact_text(arg));
    }

    out.iter()
        .map(|arg| {
            if arg.contains(' ') {
                format!("\"{arg}\"")
            } else {
                arg.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn split_flag(arg: &str) -> (&str, Option<&str>) {
    match arg.split_once('=') {
        Some((flag, value)) if flag.starts_with('-') => (flag, Some(value)),
        _ => (arg, None),
    }
}

fn is_header_flag(flag: &str) -> bool {
    flag.eq_ignore_ascii_case("--add-header") || flag.eq_ignore_ascii_case("--header")
}

fn redact_header(value: &str) -> String {
    let name = value.split(':').next().unwrap_or_default().trim().to_lowercase();
    if name == "authorization"
        || name == "cookie"
        || name == "set-cookie"
        || name == "x-api-key"
        || name.contains("token")
        || name.contains("secret")
    {
        "Authorization: ***".into()
    } else {
        redact_text(value)
    }
}

/// Mask credentials in URLs (query values and userinfo) and keep the rest of
/// the line readable. Whitespace is normalized to single spaces.
pub(crate) fn redact_text(input: &str) -> String {
    if !input.contains("://") {
        return input.to_string();
    }
    input
        .split_whitespace()
        .map(redact_token)
        .collect::<Vec<_>>()
        .join(" ")
}

fn redact_token(token: &str) -> String {
    let trimmed = token.trim_matches(|ch| matches!(ch, '"' | '\'' | '(' | ')' | '[' | ']'));
    if !trimmed.contains("://") {
        return token.to_string();
    }
    let Ok(mut url) = reqwest::Url::parse(trimmed) else {
        return token.to_string();
    };
    if url.password().is_some() {
        let _ = url.set_password(Some("***"));
    }
    let pairs: Vec<(String, String)> = url
        .query_pairs()
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect();
    if pairs.iter().any(|(key, _)| is_sensitive_key(key)) {
        url.query_pairs_mut().clear().extend_pairs(
            pairs
                .iter()
                .map(|(key, value)| (key.as_str(), if is_sensitive_key(key) { "***" } else { value.as_str() })),
        );
    }
    if token.starts_with('"') && token.ends_with('"') {
        format!("\"{url}\"")
    } else {
        url.to_string()
    }
}

fn is_sensitive_key(key: &str) -> bool {
    let key = key.to_ascii_lowercase();
    [
        "token",
        "key",
        "sig",
        "sign",
        "auth",
        "secret",
        "password",
        "passwd",
        "session",
        "cookie",
        "expire",
        "credential",
    ]
    .iter()
    .any(|item| key.contains(item))
}

fn now_string() -> String {
    chrono::Local::now().format(TIME_FORMAT).to_string()
}

fn format_for(quality: &str) -> &'static str {
    match quality {
        "1080" => "bv*[height<=1080]+ba/b[height<=1080]",
        "720" => "bv*[height<=720]+ba/b[height<=720]",
        "480" => "bv*[height<=480]+ba/b[height<=480]",
        "audio" => "bestaudio/best",
        _ => "bv*+ba/b",
    }
}

fn job_name(base: Option<&str>, index: usize, total: usize) -> String {
    let Some(base) = base else {
        return String::new();
    };
    if total > 1 {
        format!("{base} {:02}", index + 1)
    } else {
        base.to_string()
    }
}

fn output_template(job: &Job) -> String {
    let template = if job.name.trim().is_empty() {
        "%(title)s [%(id)s].%(ext)s".to_string()
    } else {
        format!("{}.%(ext)s", escape_template(&job.name))
    };
    Path::new(&job.out_dir)
        .join(template)
        .to_string_lossy()
        .to_string()
}

fn effective_threads(job: &Job, config: &AppConfig) -> u32 {
    let threads = if job.threads == 0 {
        config.fragment_threads
    } else {
        job.threads
    };
    threads.clamp(1, 16)
}

fn sanitize_file_name(input: &str) -> String {
    let cleaned: String = input
        .chars()
        .map(|ch| {
            if ch.is_control() || matches!(ch, '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|')
            {
                '_'
            } else {
                ch
            }
        })
        .collect();
    let without_ext = strip_media_ext(cleaned.trim());
    let trimmed = without_ext.trim().trim_end_matches(['.', ' ']).trim();
    let mut name: String = trimmed.chars().take(120).collect();
    if is_reserved_name(&name) {
        name.insert(0, '_');
    }
    name
}

fn strip_media_ext(name: &str) -> &str {
    let lower = name.to_ascii_lowercase();
    for ext in MEDIA_EXTS {
        let suffix = format!(".{ext}");
        if lower.ends_with(&suffix) {
            return &name[..name.len() - suffix.len()];
        }
    }
    name
}

fn is_reserved_name(name: &str) -> bool {
    let upper = name.to_ascii_uppercase();
    let stem = upper.split('.').next().unwrap_or_default();
    matches!(stem, "CON" | "PRN" | "AUX" | "NUL")
        || (stem.len() == 4
            && (stem.starts_with("COM") || stem.starts_with("LPT"))
            && stem.as_bytes()[3].is_ascii_digit())
}

fn escape_template(name: &str) -> String {
    name.replace('%', "%%")
}

fn kill_tree(pid: u32) {
    if cfg!(windows) {
        let mut command = std::process::Command::new("taskkill");
        command
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(NO_WINDOW);
        }
        let _ = command.spawn();
    } else {
        let _ = std::process::Command::new("kill")
            .args(["-9", &format!("-{pid}")])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
    }
}

async fn read_lines<R>(reader: Option<R>, downloader: Downloader, id: String)
where
    R: tokio::io::AsyncRead + Unpin,
{
    let Some(reader) = reader else {
        return;
    };
    let mut reader = BufReader::new(reader);
    let mut buffer = Vec::new();
    loop {
        buffer.clear();
        match reader.read_until(b'\n', &mut buffer).await {
            Ok(0) => break,
            Ok(_) => {
                let text = decode_line(&buffer);
                let cleaned = strip_ansi(&text);
                let line = cleaned.trim_end_matches(['\r', '\n']);
                if !line.trim().is_empty() {
                    downloader.apply_line(&id, line);
                }
            }
            Err(_) => break,
        }
    }
}

fn decode_line(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(text) => text.to_string(),
        Err(_) => {
            let (decoded, _, _) = encoding_rs::GBK.decode(bytes);
            decoded.into_owned()
        }
    }
}

fn strip_ansi(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' {
            if chars.peek() == Some(&'[') {
                chars.next();
                while let Some(&next) = chars.peek() {
                    chars.next();
                    if next.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
            continue;
        }
        output.push(ch);
    }
    output
}

fn parse_progress(line: &str) -> Option<JobProgress> {
    if !line.starts_with("[download]") {
        return None;
    }
    let tokens: Vec<&str> = line.split_whitespace().collect();
    let mut percent: Option<f64> = None;
    let mut total = String::new();
    let mut speed = String::new();
    let mut eta = String::new();

    for (index, token) in tokens.iter().enumerate() {
        if percent.is_none() {
            if let Some(value) = token.strip_suffix('%') {
                if let Ok(parsed) = value.parse::<f64>() {
                    percent = Some(parsed);
                }
            }
        }
        match *token {
            "of" => {
                let mut cursor = index + 1;
                if tokens.get(cursor) == Some(&"~") {
                    cursor += 1;
                }
                if let Some(value) = tokens.get(cursor) {
                    if looks_like_size(value) {
                        total = (*value).to_string();
                    }
                }
            }
            "at" => {
                if let Some(value) = tokens.get(index + 1) {
                    speed = clean_metric(value);
                }
            }
            "ETA" => {
                if let Some(value) = tokens.get(index + 1) {
                    eta = clean_metric(value);
                }
            }
            _ => {}
        }
    }

    let percent = percent?;
    let downloaded = if percent >= 100.0 {
        total.clone()
    } else {
        String::new()
    };
    Some(JobProgress {
        percent,
        speed,
        eta,
        downloaded,
        total,
    })
}

fn looks_like_size(token: &str) -> bool {
    match token.chars().next() {
        Some(ch) if ch.is_ascii_digit() => {}
        _ => return false,
    }
    token.chars().any(|ch| ch.is_ascii_alphabetic())
}

fn clean_metric(value: &str) -> String {
    if value.eq_ignore_ascii_case("unknown") || value == "?" || value == "N/A" {
        String::new()
    } else {
        value.to_string()
    }
}

fn parse_file(line: &str) -> Option<String> {
    if let Some(rest) = line.strip_prefix("[download] Destination: ") {
        return Some(unquote(rest));
    }
    if let Some(rest) = line.strip_prefix("[Merger] Merging formats into ") {
        return Some(unquote(rest));
    }
    if let Some(rest) = line.strip_prefix("[download] ") {
        if let Some(name) = rest.strip_suffix(" has already been downloaded") {
            return Some(unquote(name));
        }
    }
    None
}

fn unquote(value: &str) -> String {
    let trimmed = value.trim();
    let trimmed = trimmed.strip_prefix('"').unwrap_or(trimmed);
    let trimmed = trimmed.strip_suffix('"').unwrap_or(trimmed);
    trimmed.trim().to_string()
}

fn split_args(input: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    let mut has_token = false;
    let mut chars = input.chars();

    while let Some(ch) = chars.next() {
        match quote {
            Some(active) => {
                if ch == '\\' {
                    if let Some(next) = chars.next() {
                        current.push(next);
                    }
                } else if ch == active {
                    quote = None;
                } else {
                    current.push(ch);
                }
            }
            None => {
                if ch == '"' || ch == '\'' {
                    quote = Some(ch);
                    has_token = true;
                } else if ch == '\\' {
                    if let Some(next) = chars.next() {
                        current.push(next);
                        has_token = true;
                    }
                } else if ch.is_whitespace() {
                    if has_token {
                        args.push(std::mem::take(&mut current));
                        has_token = false;
                    }
                } else {
                    current.push(ch);
                    has_token = true;
                }
            }
        }
    }
    if has_token {
        args.push(current);
    }
    args
}

fn failure_message(job: &Job, code: Option<i32>) -> String {
    for line in job.log.iter().rev() {
        let text = line.trim();
        if text.starts_with("ERROR:") {
            return truncate_chars(text, 500);
        }
    }

    let mut tail: Vec<String> = job
        .log
        .iter()
        .rev()
        .filter(|line| {
            let text = line.trim();
            !text.is_empty() && !text.starts_with("[download]")
        })
        .take(3)
        .map(|line| line.trim().to_string())
        .collect();
    tail.reverse();

    if !tail.is_empty() {
        return truncate_chars(&tail.join(" / "), 500);
    }

    match code {
        Some(value) => format!("yt-dlp 退出码 {value}"),
        None => "yt-dlp 异常退出".into(),
    }
}

fn tag_failure(message: String) -> String {
    if message.starts_with('[') {
        return message;
    }
    let lower = message.to_ascii_lowercase();
    let code = if lower.contains("timed out") || lower.contains("timeout") {
        "TIMEOUT"
    } else if lower.contains("permission denied") || lower.contains("access is denied") {
        "PERMISSION"
    } else if lower.contains("no space") || lower.contains("disk full") {
        "PATH"
    } else if lower.contains("unsupported url") || lower.contains("invalid url") {
        "PARAM"
    } else if lower.contains("unable to download")
        || lower.contains("connection")
        || lower.contains("http error")
        || lower.contains("network")
        || lower.contains("unable to connect")
    {
        "NETWORK"
    } else if lower.contains("ffmpeg") && lower.contains("not found") {
        "TOOL"
    } else {
        return message;
    };
    format!("[{code}] {message}")
}

fn truncate_chars(value: &str, max: usize) -> String {
    if value.chars().count() <= max {
        value.to_string()
    } else {
        value.chars().take(max).collect()
    }
}

fn scan_output_file(job: &Job) -> String {
    let dir = Path::new(&job.out_dir);
    let since: SystemTime = match parse_time(&job.started_at) {
        Some(stamp) => (stamp - chrono::Duration::seconds(10)).into(),
        None => (chrono::Local::now() - chrono::Duration::hours(1)).into(),
    };

    let mut best: Option<(SystemTime, PathBuf)> = None;
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() || !is_media(&path) {
                continue;
            }
            let Ok(metadata) = entry.metadata() else {
                continue;
            };
            let Ok(modified) = metadata.modified() else {
                continue;
            };
            if modified < since {
                continue;
            }
            if best.as_ref().map(|(time, _)| modified > *time).unwrap_or(true) {
                best = Some((modified, path));
            }
        }
    }

    best.map(|(_, path)| path.to_string_lossy().to_string())
        .unwrap_or_default()
}

fn parse_time(value: &str) -> Option<chrono::DateTime<chrono::Local>> {
    use chrono::TimeZone;
    chrono::NaiveDateTime::parse_from_str(value, TIME_FORMAT)
        .ok()
        .and_then(|naive| chrono::Local.from_local_datetime(&naive).single())
}

fn is_media(path: &Path) -> bool {
    let ext = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    MEDIA_EXTS.contains(&ext.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_dangerous_extra_args_when_commands_disabled() {
        assert!(validate_extra_args("--limit-rate 5M", false).is_ok());
        assert!(validate_extra_args("--exec \"rm -rf\"", false).is_err());
        assert!(validate_extra_args("--exec-before-download=calc", false).is_err());
        assert!(validate_extra_args("--X=1 --update-to nightly", false).is_err());
        // Allowed once the command-execution switch is on.
        assert!(validate_extra_args("--exec calc", true).is_ok());
    }

    #[test]
    fn redacts_secret_arguments_in_rendered_command() {
        let args = vec![
            "--referer".to_string(),
            "https://site/a?token=secret&page=2".to_string(),
            "--add-header".to_string(),
            "Authorization: Bearer abc".to_string(),
            "--password".to_string(),
            "hunter2".to_string(),
        ];
        let rendered = render_redacted(&args);
        assert!(!rendered.contains("secret"));
        assert!(!rendered.contains("hunter2"));
        assert!(!rendered.contains("Bearer abc"));
        assert!(rendered.contains("page=2"));
        assert!(rendered.contains("--password ***"));
    }

    #[test]
    fn redacts_urls_in_log_lines() {
        let line = "[info] downloading https://cdn.example/v.m3u8?sign=abc&expires=99&q=1 now";
        let redacted = redact_text(line);
        assert!(!redacted.contains("sign=abc"));
        assert!(!redacted.contains("expires=99"));
        assert!(redacted.contains("q=1"));
        assert!(redacted.contains("now"));

        let proxy = redact_text("--proxy socks5://user:hunter2@127.0.0.1:7891");
        assert!(!proxy.contains("hunter2"));
        assert!(proxy.contains("user:"));

        assert_eq!(
            redact_text("[download]  12.3% of 1MiB"),
            "[download]  12.3% of 1MiB"
        );
    }

    #[test]
    fn parses_download_progress() {
        let progress =
            parse_progress("[download]  12.3% of ~ 45.67MiB at 1.23MiB/s ETA 00:12").unwrap();
        assert!((progress.percent - 12.3).abs() < f64::EPSILON);
        assert_eq!(progress.total, "45.67MiB");
        assert_eq!(progress.speed, "1.23MiB/s");
        assert_eq!(progress.eta, "00:12");
        assert_eq!(progress.downloaded, "");

        let done = parse_progress("[download] 100% of 9.00MiB in 00:00:05 at 2.00MiB/s").unwrap();
        assert_eq!(done.percent, 100.0);
        assert_eq!(done.downloaded, "9.00MiB");

        assert!(parse_progress("[download] Destination: video.mp4").is_none());
    }

    #[test]
    fn parses_output_files() {
        assert_eq!(
            parse_file("[download] Destination: C:\\tmp\\a.mp4").unwrap(),
            "C:\\tmp\\a.mp4"
        );
        assert_eq!(
            parse_file("[Merger] Merging formats into \"C:\\tmp\\a.mkv\"").unwrap(),
            "C:\\tmp\\a.mkv"
        );
        assert_eq!(
            parse_file("[download] C:\\tmp\\a.mp4 has already been downloaded").unwrap(),
            "C:\\tmp\\a.mp4"
        );
    }

    #[test]
    fn splits_extra_args() {
        assert_eq!(
            split_args("--limit-rate 5M --add-header \"Referer: a b\" --x 'c d'"),
            vec!["--limit-rate", "5M", "--add-header", "Referer: a b", "--x", "c d"]
        );
        assert_eq!(split_args("  "), Vec::<String>::new());
        assert_eq!(split_args("a\\ b"), vec!["a b"]);
    }

    #[test]
    fn formats_quality() {
        assert_eq!(format_for("best"), "bv*+ba/b");
        assert_eq!(format_for("1080"), "bv*[height<=1080]+ba/b[height<=1080]");
        assert_eq!(format_for("unknown"), "bv*+ba/b");
        assert_eq!(format_for("audio"), "bestaudio/best");
    }

    #[test]
    fn strips_ansi_codes() {
        assert_eq!(strip_ansi("\u{1b}[32mok\u{1b}[0m"), "ok");
        assert_eq!(strip_ansi("plain"), "plain");
    }

    #[test]
    fn sanitizes_custom_file_names() {
        assert_eq!(sanitize_file_name(" 我的视频 "), "我的视频");
        assert_eq!(
            sanitize_file_name("a/b\\c:d*e?f\"g<h>i|j"),
            "a_b_c_d_e_f_g_h_i_j"
        );
        assert_eq!(sanitize_file_name("clip.mp4"), "clip");
        assert_eq!(sanitize_file_name("clip.MKV"), "clip");
        assert_eq!(sanitize_file_name("  ..mp4 "), "");
        assert_eq!(sanitize_file_name("..."), "");
        assert_eq!(sanitize_file_name("CON"), "_CON");
        assert_eq!(sanitize_file_name("lpt1.txt"), "_lpt1.txt");
        assert_eq!(sanitize_file_name("100% done"), "100% done");
    }

    #[test]
    fn escapes_percent_for_output_template() {
        assert_eq!(escape_template("100% done"), "100%% done");
        assert_eq!(escape_template("plain"), "plain");
    }

    #[test]
    fn assigns_batch_names() {
        assert_eq!(job_name(Some("片段"), 0, 1), "片段");
        assert_eq!(job_name(Some("片段"), 0, 3), "片段 01");
        assert_eq!(job_name(Some("片段"), 2, 3), "片段 03");
        assert_eq!(job_name(None, 0, 3), "");
    }

    #[test]
    fn restart_marks_active_jobs_interrupted_and_keeps_parameters() {
        let mut job = Job {
            status: "running".into(),
            name: "视频".into(),
            referer: "https://example.com".into(),
            cookies_browser: "edge".into(),
            extra_args: "--limit-rate 1M".into(),
            threads: 8,
            ..Job::default()
        };
        job.progress.speed = "1 MB/s".into();

        assert!(mark_interrupted(&mut job, "2026-01-01 00:00:00"));
        assert_eq!(job.status, "interrupted");
        assert_eq!(job.finished_at, "2026-01-01 00:00:00");
        assert!(job.error.contains("可重试"));
        assert!(job.progress.speed.is_empty());
        assert_eq!(job.name, "视频");
        assert_eq!(job.referer, "https://example.com");
        assert_eq!(job.cookies_browser, "edge");
        assert_eq!(job.extra_args, "--limit-rate 1M");
        assert_eq!(job.threads, 8);

        assert!(!mark_interrupted(&mut job, "2026-01-02 00:00:00"));
    }

    #[test]
    fn builds_output_template() {
        let mut job = Job {
            out_dir: "C:\\out".into(),
            ..Job::default()
        };
        assert!(output_template(&job).ends_with("%(title)s [%(id)s].%(ext)s"));

        job.name = "我的视频".into();
        assert!(output_template(&job).ends_with("我的视频.%(ext)s"));

        job.name = "50% 内容".into();
        assert!(output_template(&job).ends_with("50%% 内容.%(ext)s"));
    }

    #[test]
    fn parses_download_request_options_over_ipc() {
        let request: DownloadRequest = serde_json::from_str(
            r#"{"urls":["https://example.com/a.m3u8"],"quality":"best","fileName":"我的视频","threads":8}"#,
        )
        .unwrap();
        assert_eq!(request.file_name.as_deref(), Some("我的视频"));
        assert_eq!(request.threads, Some(8));
    }

    #[test]
    fn persists_name_and_threads_without_breaking_old_jobs() {
        let mut job = Job::default();
        job.name = "视频".into();
        job.threads = 8;
        let text = serde_json::to_string(&job).unwrap();
        assert!(text.contains("\"name\":\"视频\""));
        assert!(text.contains("\"threads\":8"));

        let legacy: Job = serde_json::from_str(
            r#"{"id":"1","url":"u","quality":"best","outDir":"d","status":"queued","progress":{"percent":0,"speed":"","eta":"","downloaded":"","total":""},"log":[],"error":"","file":"","createdAt":"","startedAt":"","finishedAt":""}"#,
        )
        .unwrap();
        assert_eq!(legacy.name, "");
        assert_eq!(legacy.threads, 0);
    }

    #[test]
    fn resolves_effective_threads() {
        let mut job = Job::default();
        let mut config = AppConfig::default();
        config.fragment_threads = 6;
        assert_eq!(effective_threads(&job, &config), 6);

        job.threads = 12;
        assert_eq!(effective_threads(&job, &config), 12);

        job.threads = 99;
        assert_eq!(effective_threads(&job, &config), 16);

        job.threads = 0;
        config.fragment_threads = 0;
        assert_eq!(effective_threads(&job, &config), 1);
    }
}
