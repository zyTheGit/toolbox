use std::path::Path;
use std::time::{Duration, Instant};

use tauri::Emitter;

use crate::core::persist;
use crate::models::Job;
use crate::state::push_warning;

use super::args::redact_text;
use super::credentials::{strip_job, JobCredentials};
use super::naming::{finalize_media_ext, now_string};
use super::parse::{
    has_subtitle_ext, looks_chinese, parse_file, parse_progress, scan_output_file,
    scan_subtitle_file,
};
use super::toast::{emit_toast, failure_hint, job_label};
use super::{Downloader, Inner};

/// 后台任务的终态；与取消标记分开，便于 finish_job 统一收口。
pub(super) enum Finish {
    Completed,
    Canceled,
    Failed(String),
}

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
        let (mut snapshot, summary) = {
            let mut inner = self.inner.lock();
            let snapshot = {
                let Inner {
                    jobs,
                    cancel_ids,
                    children,
                    last_emit,
                    batch_success,
                    batch_failed,
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
                        *batch_success += 1;
                    }
                    Finish::Canceled => {
                        job.status = "canceled".into();
                        job.error = "已取消".into();
                    }
                    Finish::Failed(message) => {
                        job.status = "failed".into();
                        job.error = message;
                        *batch_failed += 1;
                    }
                }
                job.finished_at = now_string();
                job.clone()
            };
            // 本批最后一个任务结束：取出汇总并复位计数，供“全部完成”提醒使用。
            (snapshot, batch_summary(&mut inner))
        };

        if snapshot.status == "completed" {
            if snapshot.file.trim().is_empty() {
                snapshot.file = scan_output_file(&snapshot);
                if snapshot.file.is_empty() {
                    snapshot.file = scan_subtitle_file(&snapshot);
                }
            }
            // 未知扩展名（直链常见 .unknown_video）按文件头补正，保证系统能打开。
            snapshot.file = finalize_media_ext(&snapshot.file);
            let mut inner = self.inner.lock();
            if let Some(job) = inner.jobs.iter_mut().find(|item| item.id == id) {
                job.file = snapshot.file.clone();
            }
        }

        // 站点把非中文内容标成中文字幕时给出提示，避免误以为拿到了中文。
        let zh_requested = matches!(snapshot.subtitles.as_str(), "zh" | "zh-mt");
        if snapshot.status == "completed" && zh_requested && has_subtitle_ext(&snapshot.file) {
            if let Ok(text) = std::fs::read_to_string(&snapshot.file) {
                if !looks_chinese(&text) {
                    snapshot.notice = if snapshot.subtitles == "zh" {
                        "站点标注的中文字幕实际不是中文，可尝试「中文（YouTube 机翻）」".into()
                    } else {
                        "该视频的语音语言被站点标错，YouTube 机翻不会产出中文，建议下载原字幕后另行翻译"
                            .into()
                    };
                    let mut inner = self.inner.lock();
                    if let Some(job) = inner.jobs.iter_mut().find(|item| item.id == id) {
                        job.notice = snapshot.notice.clone();
                    }
                }
            }
        }

        self.emit_job(&snapshot);
        self.save();
        self.notify_finished(&snapshot, summary);
    }

    /// 下载事件提醒：应用内提示始终发送，系统通知仅在主窗口隐藏时由 notify() 放行。
    fn notify_finished(&self, job: &Job, summary: Option<(usize, usize)>) {
        let notify = self.config.read().notify.clone();
        if !notify.download {
            return;
        }
        let Some(app) = &self.app else {
            return;
        };
        if job.status == "completed" && notify.events.success {
            let label = job_label(job);
            emit_toast(app, "completed", "下载完成", &label);
            crate::core::notify::notify(app, "download", None, "下载完成", &label);
        } else if job.status == "failed" && notify.events.failed {
            let label = job_label(job);
            let text = format!("{label}：{}", failure_hint(&job.error));
            emit_toast(app, "failed", "下载失败", &text);
            crate::core::notify::notify(app, "download", None, "下载失败", &text);
        }
        if let Some((success, failed)) = summary {
            if notify.events.all_done {
                self.notify_all_done((success, failed));
            }
        }
    }

    fn notify_all_done(&self, counts: (usize, usize)) {
        let notify = self.config.read().notify.clone();
        if !notify.download || !notify.events.all_done {
            return;
        }
        let Some(app) = &self.app else {
            return;
        };
        let (success, failed) = counts;
        let text = if failed == 0 {
            format!("本轮完成 {success} 个")
        } else {
            format!("本轮完成 {success} 个，失败 {failed} 个")
        };
        emit_toast(app, "all-done", "下载任务已完成", &text);
        crate::core::notify::notify(app, "download", None, "下载任务已完成", &text);
    }

    pub(super) fn fail_pending(&self, current_id: &str, message: &str) {
        let (updated, summary) = {
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
            inner.batch_failed += updated.len();
            (updated, batch_summary(&mut inner))
        };

        for job in &updated {
            self.emit_job(job);
        }
        self.save();
        if let Some(counts) = summary {
            self.notify_all_done(counts);
        }
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

/// 队列由空闲转入繁忙时开启新一轮批次；已有活跃任务则沿用当前批次。
pub(super) fn begin_batch_if_idle(inner: &mut Inner) {
    if inner.jobs.iter().any(is_active) {
        return;
    }
    inner.batch_open = true;
    inner.batch_success = 0;
    inner.batch_failed = 0;
}

/// 队列全部结束且本轮有结果时返回 `(成功数, 失败数)` 并复位计数。
///
/// 全被取消（计数为 0）的批次不产生“全部完成”提醒。
pub(super) fn batch_summary(inner: &mut Inner) -> Option<(usize, usize)> {
    if !inner.batch_open || inner.jobs.iter().any(is_active) {
        return None;
    }
    inner.batch_open = false;
    let counts = (inner.batch_success, inner.batch_failed);
    inner.batch_success = 0;
    inner.batch_failed = 0;
    if counts.0 + counts.1 == 0 {
        return None;
    }
    Some(counts)
}

/// 读取下载记录文件；`Ok(None)` 表示文件尚不存在。
///
/// 启动初始化与备份恢复后的重载共用这一入口，避免两套加载逻辑。
pub(super) fn read_jobs(path: &Path) -> Result<Option<Vec<Job>>, String> {
    persist::load_json::<Vec<Job>>(path)
}

/// Restart policy: queued/running jobs are never resumed silently; they become
/// interrupted and keep their parameters so the user can retry.
pub(super) fn mark_interrupted(job: &mut Job, stamp: &str) -> bool {
    if !is_active(job) {
        return false;
    }
    job.status = "interrupted".into();
    job.error = "应用退出，任务已中断，可继续".into();
    job.finished_at = stamp.to_string();
    job.progress.speed.clear();
    job.progress.eta.clear();
    true
}
