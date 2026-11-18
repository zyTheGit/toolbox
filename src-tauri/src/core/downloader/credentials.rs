//! 下载凭据的进程内保存与防御性剥离。
//!
//! 代理、Cookie 来源、Referer 与附加参数（可能含自定义 Header）只保存在进程内的
//! `HashMap<job_id, JobCredentials>` 中，绝不写入 `jobs.json`、事件或前端快照。
//! 应用重启后凭据随之丢失，任务在重试时会明确提示重新填写。

use std::collections::HashMap;

use crate::models::{DownloadRequest, Job, JobRetryRequest};

use super::args::redact_text;
use super::state::mark_interrupted;

/// 凭据缺失时的统一错误：前端据此弹出重填对话框，不能当作普通下载失败。
pub(super) const CREDENTIALS_MISSING: &str =
    "[CREDENTIALS] 下载凭据仅保存在内存中，应用重启后已丢失，请重新填写代理 / Cookie / Referer 后重试";

/// 单个任务的下载凭据（仅内存，禁止序列化落盘）。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct JobCredentials {
    pub referer: String,
    pub cookies_browser: String,
    pub proxy: String,
    pub extra_args: String,
}

impl JobCredentials {
    /// 从新建任务的请求中提取凭据（统一去首尾空白）。
    pub(super) fn from_request(request: &DownloadRequest) -> Self {
        Self {
            referer: request.referer.trim().to_string(),
            cookies_browser: request.cookies_browser.trim().to_string(),
            proxy: request.proxy.trim().to_string(),
            extra_args: request.extra_args.trim().to_string(),
        }
    }

    pub(super) fn is_empty(&self) -> bool {
        self.referer.is_empty()
            && self.cookies_browser.is_empty()
            && self.proxy.is_empty()
            && self.extra_args.is_empty()
    }

    /// 合并用户重填的凭据：`None` 保持原值，`Some("")` 表示显式清空该项。
    pub(super) fn merge(&mut self, request: &JobRetryRequest) {
        if let Some(value) = &request.referer {
            self.referer = value.trim().to_string();
        }
        if let Some(value) = &request.cookies_browser {
            self.cookies_browser = value.trim().to_string();
        }
        if let Some(value) = &request.proxy {
            self.proxy = value.trim().to_string();
        }
        if let Some(value) = &request.extra_args {
            self.extra_args = value.trim().to_string();
        }
    }

    /// 从旧版 `jobs.json` 的明文字段提取凭据并清空 Job 上的四个字段。
    ///
    /// 返回 `None` 表示该记录本身没有明文凭据（新格式或从未使用凭据）。
    pub(super) fn take_legacy(job: &mut Job) -> Option<Self> {
        let credentials = Self {
            referer: std::mem::take(&mut job.referer),
            cookies_browser: std::mem::take(&mut job.cookies_browser),
            proxy: std::mem::take(&mut job.proxy),
            extra_args: std::mem::take(&mut job.extra_args),
        };
        if credentials.is_empty() {
            return None;
        }
        job.has_credentials = true;
        Some(credentials)
    }
}

/// 写入磁盘或事件前的防御性剥离：即使将来有别的入口构造 Job，也不会外泄凭据。
pub(super) fn strip_job(job: &mut Job) {
    job.referer.clear();
    job.cookies_browser.clear();
    job.proxy.clear();
    job.extra_args.clear();
}

/// 请求是否携带了任一凭据字段（含显式清空的空串）。
pub(super) fn has_credential_fields(request: &JobRetryRequest) -> bool {
    request.referer.is_some()
        || request.cookies_browser.is_some()
        || request.proxy.is_some()
        || request.extra_args.is_some()
}

/// 请求是否提供了非空凭据，用于判断“重填”是否足以满足执行条件。
pub(super) fn provides_credentials(request: &JobRetryRequest) -> bool {
    [
        &request.referer,
        &request.cookies_browser,
        &request.proxy,
        &request.extra_args,
    ]
    .into_iter()
    .flatten()
    .any(|value| !value.trim().is_empty())
}

/// 加载磁盘记录时的统一处理：日志脱敏、旧明文凭据迁入内存、恢复中断状态。
///
/// 返回 `(内容是否变化, 迁移的凭据条数)`。
pub(super) fn adopt_loaded_jobs(
    jobs: &mut [Job],
    credentials: &mut HashMap<String, JobCredentials>,
    stamp: &str,
) -> (bool, usize) {
    let mut changed = false;
    let mut migrated = 0usize;
    for job in jobs.iter_mut() {
        for line in job.log.iter_mut() {
            *line = redact_text(line);
        }
        if let Some(creds) = JobCredentials::take_legacy(job) {
            credentials.insert(job.id.clone(), creds);
            migrated += 1;
            changed = true;
        }
        if mark_interrupted(job, stamp) {
            changed = true;
        }
    }
    (changed, migrated)
}
