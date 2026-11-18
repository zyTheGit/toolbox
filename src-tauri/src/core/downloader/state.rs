use std::time::{Duration, Instant};

use tauri::Emitter;

use crate::core::persist;
use crate::models::Job;
use crate::state::push_warning;

use super::args::redact_text;
use super::credentials::{strip_job, JobCredentials};
use super::naming::now_string;
use super::parse::{parse_file, parse_progress, scan_output_file};
use super::{Downloader, Finish, Inner};

const MAX_LOG_LINES: usize = 400;
const KEEP_TERMINAL: usize = 100;
const EMIT_INTERVAL: Duration = Duration::from_millis(500);

impl Downloader {
    pub(super) fn apply_line(&self, id: &str, line: &str) {
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

    pub(super) fn finish_job(&self, id: &str, outcome: Finish) {
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
                if let Some(app) = &self.app {
                    crate::core::notify::notify(app, "download", "下载完成", &label);
                }
            } else if let Some(app) = &self.app {
                crate::core::notify::notify(
                    app,
                    "download",
                    "下载失败",
                    &format!("{label}：点击查看原因"),
                );
            }
        }
    }

    pub(super) fn fail_pending(&self, current_id: &str, message: &str) {
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

    pub(super) fn job_snapshot(&self, id: &str) -> Option<Job> {
        self.inner
            .lock()
            .jobs
            .iter()
            .find(|item| item.id == id)
            .cloned()
    }

    /// 从内存凭据表取出执行凭据；`None` 表示该任务没有留存凭据。
    pub(super) fn job_credentials(&self, id: &str) -> Option<JobCredentials> {
        self.inner.lock().credentials.get(id).cloned()
    }

    pub(super) fn emit_job(&self, job: &Job) {
        let Some(app) = &self.app else {
            return;
        };
        let mut payload = job.clone();
        strip_job(&mut payload);
        let _ = app.emit("job-updated", &payload);
    }

    pub(super) fn emit_removed(&self, id: &str) {
        let Some(app) = &self.app else {
            return;
        };
        let _ = app.emit("job-removed", id.to_string());
    }

    pub(super) fn save(&self) {
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

        // 落盘前的最后一道防线：即使未来有别的入口构造 Job，也不会把凭据写进 jobs.json。
        let sanitized: Vec<Job> = kept
            .into_iter()
            .map(|job| {
                let mut copy = job.clone();
                strip_job(&mut copy);
                copy
            })
            .collect();

        if let Err(err) = persist::save_json(&self.paths.jobs_file, &sanitized) {
            push_warning(&self.warnings, format!("保存下载任务失败: {err}"));
        }
    }
}

pub(super) fn is_active(job: &Job) -> bool {
    job.status == "queued" || job.status == "running"
}

/// Restart policy: queued/running jobs are never resumed silently; they become
/// interrupted and keep their parameters so the user can retry.
pub(super) fn mark_interrupted(job: &mut Job, stamp: &str) -> bool {
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
