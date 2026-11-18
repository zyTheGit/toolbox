//! 备份脱敏 canary 测试。

use super::*;
use crate::models::{HistoryEntry, Job, Task, TaskPayload};

pub(crate) const CANARY_COMMAND: &str = "CANARY_COMMAND_7f1c";
pub(crate) const CANARY_ENV: &str = "CANARY_ENV_SECRET";
pub(crate) const CANARY_OUTPUT: &str = "CANARY_OUTPUT_TEXT";
pub(crate) const CANARY_QUERY: &str = "CANARY_QUERY_TOKEN";
pub(crate) const CANARY_API_KEY: &str = "CANARY_API_KEY";
pub(crate) const CANARY_COOKIE: &str = "CANARY_COOKIE_VALUE";
pub(crate) const CANARY_PROXY_PASS: &str = "CANARY_PROXY_PASSWORD";
pub(crate) const CANARY_HEADER: &str = "CANARY_HEADER_TOKEN";
pub(crate) const HISTORY_BODY: &str = "需要保留的翻译正文";

fn canary_config() -> AppConfig {
    let mut config = AppConfig::default();
    config.security.allow_commands = true;
    config.proxy = format!("http://user:{CANARY_PROXY_PASS}@127.0.0.1:7890");
    config.translate.openai.api_key = CANARY_API_KEY.into();
    config
}

fn canary_task() -> Task {
    Task {
        id: "task-1".into(),
        name: "命令任务".into(),
        task_type: "command".into(),
        enabled: true,
        payload: TaskPayload {
            url: format!("https://example.com/a?token={CANARY_QUERY}&page=2"),
            command: CANARY_COMMAND.into(),
            env: format!("TOKEN={CANARY_ENV}"),
            ..TaskPayload::default()
        },
        last_output: CANARY_OUTPUT.into(),
        last_error: format!("https://example.com/a?token={CANARY_QUERY}"),
        last_job_id: "job-1".into(),
        running: true,
        ..Task::default()
    }
}

fn canary_job() -> Job {
    Job {
        id: "job-1".into(),
        url: format!("https://example.com/v.m3u8?token={CANARY_QUERY}&page=2"),
        referer: format!("https://example.com/watch?token={CANARY_QUERY}"),
        cookies_browser: CANARY_COOKIE.into(),
        proxy: format!("http://user:{CANARY_PROXY_PASS}@127.0.0.1:7890"),
        extra_args: format!("--add-header Authorization: Bearer {CANARY_HEADER}"),
        log: vec![format!("header Bearer {CANARY_HEADER}")],
        error: format!("failed https://example.com/x?token={CANARY_QUERY}"),
        status: "queued".into(),
        has_credentials: true,
        ..Job::default()
    }
}

fn canary_history() -> HistoryEntry {
    HistoryEntry {
        id: "h1".into(),
        text: HISTORY_BODY.into(),
        result: "keep me".into(),
        ..Default::default()
    }
}

/// 未经任何处理的“恶意备份”：命令执行开启、命令任务启用、凭据齐全。
pub(crate) fn canary_bundle() -> BackupBundle {
    BackupBundle {
        schema_version: BUNDLE_SCHEMA_VERSION,
        app_version: "0.1.0".into(),
        exported_at: String::new(),
        config: canary_config(),
        tasks: vec![canary_task()],
        history: vec![canary_history()],
        jobs: vec![canary_job()],
    }
}

#[test]
fn default_export_contains_no_command_env_output_or_token() {
    let bundle = build_bundle(
        canary_config(),
        vec![canary_task()],
        vec![canary_history()],
        vec![canary_job()],
        false,
    );
    let json = serde_json::to_string(&bundle).unwrap();

    for canary in [
        CANARY_COMMAND,
        CANARY_ENV,
        CANARY_OUTPUT,
        CANARY_QUERY,
        CANARY_API_KEY,
        CANARY_COOKIE,
        CANARY_PROXY_PASS,
        CANARY_HEADER,
    ] {
        assert!(!json.contains(canary), "默认导出泄漏了 {canary}");
    }
    assert!(json.contains(HISTORY_BODY), "翻译历史正文必须保留");
    assert_eq!(bundle.tasks[0].payload.command, REDACTED_COMMAND);
}

#[test]
fn include_commands_reveals_command_but_never_secrets() {
    let bundle = build_bundle(
        canary_config(),
        vec![canary_task()],
        vec![canary_history()],
        vec![canary_job()],
        true,
    );
    let json = serde_json::to_string(&bundle).unwrap();

    assert!(json.contains(CANARY_COMMAND), "显式导出必须能看到命令");
    for canary in [
        CANARY_ENV,
        CANARY_QUERY,
        CANARY_API_KEY,
        CANARY_COOKIE,
        CANARY_PROXY_PASS,
        CANARY_HEADER,
    ] {
        assert!(!json.contains(canary), "显式导出仍泄漏了 {canary}");
    }
    assert_eq!(bundle.config.translate.openai.api_key, "");
    assert!(bundle.config.proxy.is_empty());
}

#[test]
fn diagnostics_reuse_sanitizers_and_drop_history() {
    let value = diagnostics_value(
        canary_config(),
        vec!["warn".into()],
        vec![canary_task()],
        vec![canary_job()],
        false,
    );
    let json = value.to_string();
    assert!(!json.contains(HISTORY_BODY));
    assert!(!json.contains(CANARY_COMMAND));
    assert!(!json.contains(CANARY_QUERY));
    assert!(!json.contains(CANARY_API_KEY));
    assert!(!json.contains(CANARY_COOKIE));
}

#[test]
fn import_disables_command_tasks_and_wipes_sensitive_fields() {
    let paths = AppPaths::from_data_dir(std::env::temp_dir().join("toolbox-backup-test"));
    let (plan, disabled) = sanitize_import(canary_bundle(), &paths).unwrap();

    assert!(!plan.config.security.allow_commands, "导入必须关闭命令执行");
    assert!(plan.config.proxy.is_empty());
    assert_eq!(disabled, 1);
    assert_eq!(plan.tasks.len(), 1);

    let task = &plan.tasks[0];
    assert_eq!(task.payload.command, CANARY_COMMAND, "命令文本保留供核对");
    assert!(!task.enabled);
    assert!(!task.running);
    assert!(task.payload.env.is_empty());
    assert!(task.last_output.is_empty());
    assert!(task.last_error.contains("待确认"));
    assert!(task.last_job_id.is_empty());
    assert!(task.next_run.is_none());

    let job = &plan.jobs[0];
    assert!(job.referer.is_empty());
    assert!(job.cookies_browser.is_empty());
    assert!(job.proxy.is_empty());
    assert!(job.extra_args.is_empty());
    assert!(!job.has_credentials);
    assert!(job.log.is_empty());
    assert_eq!(job.status, "interrupted");

    let json = serde_json::to_string(&plan.tasks).unwrap();
    for canary in [CANARY_ENV, CANARY_QUERY, CANARY_OUTPUT] {
        assert!(!json.contains(canary), "导入后仍残留 {canary}");
    }
}

#[test]
fn sanitizes_url_credentials_and_sensitive_query() {
    let value = sanitize_url("https://user:pass@example.com/live.m3u8?token=abc&page=2");
    assert!(!value.contains("user"));
    assert!(!value.contains("pass"));
    assert!(!value.contains("abc"));
    assert!(value.contains("page=2"));
}
