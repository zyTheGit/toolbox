use std::path::Path;
use std::process::Stdio;
use std::time::{Duration, Instant};

use tokio::process::Command;

use crate::core::tools;
use crate::models::{Job, JobProgress, ToolStatus};

use super::args::{render_redacted, validate_extra_args};
use super::naming::{effective_threads, extra_flags_for, format_for, now_string, output_template};
use super::parse::{failure_message, read_lines, split_args, tag_failure};
use super::{Downloader, Finish, JobOptions};

const NO_WINDOW: u32 = 0x0800_0000;

impl Downloader {
    pub(super) fn ensure_worker(&self) {
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
        // 按档位附加参数：视频统一合并为 MP4，音频提取为 MP3。
        for flag in extra_flags_for(&job.quality) {
            args.push((*flag).into());
        }

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
}

pub(super) fn kill_tree(pid: u32) {
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
