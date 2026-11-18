//! 备份导入 / 导出的脱敏规则。
//!
//! 导出与诊断导出共用这里的 `scrub_*` 函数；导入侧把备份视为不可信输入，
//! 关闭命令执行、禁用命令任务并清空一切敏感与运行态字段。

use crate::core::{config, downloader::redact_text};
use crate::models::{AppConfig, HistoryEntry, Job, Task};
use crate::state::AppPaths;

use super::restore::RestorePlan;
use super::{BackupBundle, BUNDLE_SCHEMA_VERSION};

/// 默认导出时命令文本的占位符。
pub(super) const REDACTED_COMMAND: &str = "（命令未导出）";
/// 环境变量始终视为敏感信息，即使勾选“包含命令与输出”也不导出。
pub(super) const REDACTED_ENV: &str = "（环境变量未导出）";
/// 默认导出时运行输出 / 错误的占位符。
pub(super) const REDACTED_OUTPUT: &str = "（输出未导出）";
/// 导入的命令任务被禁用后的说明（任务列表直接展示 lastError）。
pub(super) const PENDING_CONFIRM_MESSAGE: &str =
    "来自备份的命令任务，待确认：命令执行已关闭，请核对命令内容后再手动启用";
/// 导入后处于中断状态的下载记录说明。
pub(super) const JOB_RESTORED_MESSAGE: &str = "来自备份，需重新确认凭据后重试";

/// 组装导出用备份包（纯函数，便于 canary 测试）。
pub(super) fn build_bundle(
    mut config: AppConfig,
    mut tasks: Vec<Task>,
    history: Vec<HistoryEntry>,
    mut jobs: Vec<Job>,
    include_commands: bool,
) -> BackupBundle {
    // 密钥与全局代理永远不进入备份。
    config::clear_secrets(&mut config);
    config.proxy.clear();

    for task in tasks.iter_mut() {
        scrub_task_for_export(task, include_commands);
    }
    for job in jobs.iter_mut() {
        scrub_job_for_export(job, include_commands);
    }

    BackupBundle {
        schema_version: BUNDLE_SCHEMA_VERSION,
        app_version: env!("CARGO_PKG_VERSION").into(),
        exported_at: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        config,
        tasks,
        history,
        jobs,
    }
}

/// 诊断导出使用同一套脱敏逻辑，且不包含翻译历史正文。
pub fn diagnostics_value(
    mut config: AppConfig,
    warnings: Vec<String>,
    mut tasks: Vec<Task>,
    mut jobs: Vec<Job>,
    include_commands: bool,
) -> serde_json::Value {
    config::clear_secrets(&mut config);
    config.proxy.clear();
    for task in tasks.iter_mut() {
        scrub_task_for_export(task, include_commands);
    }
    for job in jobs.iter_mut() {
        scrub_job_for_export(job, include_commands);
    }
    serde_json::json!({
        "appVersion": env!("CARGO_PKG_VERSION"),
        "platform": std::env::consts::OS,
        "warnings": warnings,
        "config": config,
        "tasks": tasks,
        "jobs": jobs,
    })
}

fn scrub_task_for_export(task: &mut Task, include_commands: bool) {
    task.payload.url = sanitize_url(&task.payload.url);
    task.payload.command = if include_commands {
        task.payload.command.clone()
    } else {
        REDACTED_COMMAND.into()
    };
    task.payload.env = REDACTED_ENV.into();
    task.last_output = if include_commands {
        scrub_multiline(&task.last_output)
    } else {
        REDACTED_OUTPUT.into()
    };
    task.last_error = if include_commands {
        scrub_multiline(&task.last_error)
    } else {
        REDACTED_OUTPUT.into()
    };
    task.last_job_id.clear();
    task.running = false;
}

fn scrub_job_for_export(job: &mut Job, include_commands: bool) {
    job.url = sanitize_url(&job.url);
    // Cookie、Referer、代理与额外 Header 无论是否包含命令都不得导出。
    job.referer.clear();
    job.cookies_browser.clear();
    job.proxy.clear();
    job.extra_args.clear();
    job.has_credentials = false;
    job.error = scrub_multiline(&job.error);
    if include_commands {
        for line in job.log.iter_mut() {
            *line = scrub_multiline(line);
        }
    } else {
        job.log.clear();
    }
}

/// 文本脱敏：先按 URL 规则处理，再遮蔽 Bearer / Authorization / Cookie 等
/// Header Token，避免“包含输出”时把凭据带出去。按行处理以保留输出结构。
fn scrub_multiline(input: &str) -> String {
    input.lines().map(scrub_line).collect::<Vec<_>>().join("\n")
}

fn scrub_line(line: &str) -> String {
    let redacted = redact_text(line);
    let mut out: Vec<String> = Vec::new();
    let mut mask_next = false;
    for token in redacted.split_whitespace() {
        if mask_next {
            out.push("***".into());
            mask_next = false;
            continue;
        }
        let lower = token.to_ascii_lowercase();
        if lower == "bearer" || lower == "basic" {
            out.push(token.to_string());
            mask_next = true;
            continue;
        }
        if let Some(name) = token.split_once(':').map(|(name, _)| name) {
            let header = name.to_ascii_lowercase();
            if header == "authorization"
                || header == "cookie"
                || header == "set-cookie"
                || header == "x-api-key"
                || header.contains("token")
                || header.contains("secret")
            {
                out.push(format!("{name}: ***"));
                continue;
            }
        }
        out.push(token.to_string());
    }
    out.join(" ")
}

/// 校验并脱敏不可信的备份包，返回可落盘的恢复数据与被禁用的命令任务数。
pub(super) fn sanitize_import(
    bundle: BackupBundle,
    paths: &AppPaths,
) -> Result<(RestorePlan, usize), String> {
    super::check_limits(&bundle)?;

    let mut config = bundle.config;
    config::normalize(&mut config, paths);
    config::clear_secrets(&mut config);
    // 备份不能恢复命令执行能力，也不能恢复带凭据的代理。
    config.security.allow_commands = false;
    config.proxy.clear();
    config::validate(&config).map_err(|err| format!("[DATA] 备份配置无效：{err}"))?;

    let mut disabled = 0usize;
    let mut tasks = bundle.tasks;
    for task in tasks.iter_mut() {
        scrub_task_for_import(task, &mut disabled);
    }
    let mut jobs = bundle.jobs;
    for job in jobs.iter_mut() {
        scrub_job_for_import(job);
    }

    Ok((
        RestorePlan {
            config,
            history: bundle.history,
            tasks,
            jobs,
        },
        disabled,
    ))
}

fn scrub_task_for_import(task: &mut Task, disabled: &mut usize) {
    task.payload.url = sanitize_url(&task.payload.url);
    // 命令文本保留（用户需要核对），但其余敏感 / 运行态字段一律清空。
    task.payload.env.clear();
    task.last_output.clear();
    task.last_error.clear();
    task.last_job_id.clear();
    task.last_run_at.clear();
    task.last_status.clear();
    task.running = false;
    task.next_run = None;
    task.revision = 0;

    if task.task_type == "command" {
        task.enabled = false;
        task.last_status = "skipped".into();
        task.last_error = PENDING_CONFIRM_MESSAGE.into();
        *disabled += 1;
    }
}

fn scrub_job_for_import(job: &mut Job) {
    job.url = sanitize_url(&job.url);
    job.referer.clear();
    job.cookies_browser.clear();
    job.proxy.clear();
    job.extra_args.clear();
    job.has_credentials = false;
    job.log.clear();
    job.progress = Default::default();
    job.error.clear();

    // 排队 / 运行中的记录不得自动恢复执行。
    if job.status == "queued" || job.status == "running" {
        job.status = "interrupted".into();
        job.error = JOB_RESTORED_MESSAGE.into();
        job.finished_at = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    }
}

/// 把 URL 中的 userinfo 与敏感查询参数替换为占位符；无法解析时返回空串。
pub(super) fn sanitize_url(raw: &str) -> String {
    let Ok(mut url) = reqwest::Url::parse(raw) else {
        return String::new();
    };
    if !url.username().is_empty() {
        let _ = url.set_username("***");
    }
    if url.password().is_some() {
        let _ = url.set_password(Some("***"));
    }
    let pairs: Vec<(String, String)> = url
        .query_pairs()
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect();
    if pairs.iter().any(|(key, _)| is_sensitive_key(key)) {
        url.query_pairs_mut().clear().extend_pairs(
            pairs.iter().map(|(key, value)| {
                (
                    key.as_str(),
                    if is_sensitive_key(key) { "***" } else { value.as_str() },
                )
            }),
        );
    }
    url.to_string()
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
    .any(|hint| key.contains(hint))
}

#[cfg(test)]
pub(crate) mod tests;
