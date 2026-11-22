//! 下载任务重试：内存中凭据缺失时明确要求重填。

use crate::models::{Job, JobProgress, JobRetryRequest};

use super::args::validate_extra_args;
use super::credentials::{
    has_credential_fields, provides_credentials, CREDENTIALS_MISSING,
};
use super::state::{begin_batch_if_idle, is_active};
use super::{Downloader, Inner};

impl Downloader {
    /// 重新入队。任务需要凭据但内存中已无（重启后）且请求未补填时，
    /// 返回 `[CREDENTIALS]` 中文错误，由前端弹出重填对话框。
    pub fn retry(&self, id: &str, request: Option<JobRetryRequest>) -> Result<Job, String> {
        let request = request.unwrap_or_default();
        let config = self.config.read().clone();
        if let Some(extra_args) = request.extra_args.as_deref() {
            validate_extra_args(extra_args, config.security.allow_commands)?;
        }

        let job = {
            let mut inner = self.inner.lock();
            // 空闲时重试等同开启新一轮下载批次，汇总计数从零开始。
            begin_batch_if_idle(&mut inner);
            let Inner {
                jobs, credentials, ..
            } = &mut *inner;
            let Some(job) = jobs.iter_mut().find(|item| item.id == id) else {
                return Err("[CONFLICT] 任务不存在".into());
            };
            if is_active(job) {
                return Err("[CONFLICT] 任务正在排队或运行，无需重试".into());
            }

            let stored = credentials.get(id).cloned();
            if job.has_credentials && stored.is_none() && !provides_credentials(&request) {
                return Err(CREDENTIALS_MISSING.into());
            }

            if has_credential_fields(&request) || stored.is_some() {
                let mut merged = stored.unwrap_or_default();
                merged.merge(&request);
                let empty = merged.is_empty();
                if empty {
                    credentials.remove(id);
                } else {
                    credentials.insert(id.to_string(), merged);
                }
                // 用户显式改写凭据字段时，以合并结果为准更新“是否需要凭据”。
                if has_credential_fields(&request) || empty {
                    job.has_credentials = !empty;
                }
            }

            if let Some(quality) = request.quality.as_deref() {
                let quality = quality.trim();
                if !quality.is_empty() {
                    job.quality = quality.to_string();
                }
            }
            if let Some(threads) = request.threads {
                if threads > 0 {
                    job.threads = threads.clamp(1, 16);
                }
            }

            job.status = "queued".into();
            job.error.clear();
            // 保留成品路径：续传与清理临时文件都按它推导前缀。
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

    /// 重新下载：先清空该任务的临时分片，再从零开始。
    pub fn redownload(
        &self,
        id: &str,
        request: Option<JobRetryRequest>,
    ) -> Result<Job, String> {
        let job = self
            .job_snapshot(id)
            .ok_or_else(|| "[CONFLICT] 任务不存在".to_string())?;
        if is_active(&job) {
            return Err("[CONFLICT] 任务正在排队或运行，无需重新下载".into());
        }
        self.cleanup_temps_for(&job);
        self.retry(id, request)
    }
}
