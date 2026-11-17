use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::core::downloader::redact_text;
use crate::core::{config, persist};
use crate::models::{AppConfig, HistoryEntry, HistoryQuery, Job, Task};
use crate::state::AppState;

pub const BUNDLE_SCHEMA_VERSION: u32 = 1;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BackupBundle {
    pub schema_version: u32,
    pub app_version: String,
    pub exported_at: String,
    pub config: AppConfig,
    pub tasks: Vec<Task>,
    pub history: Vec<HistoryEntry>,
    pub jobs: Vec<Job>,
}

pub fn build(state: &AppState) -> BackupBundle {
    let mut config = state.view_config();
    config::clear_secrets(&mut config);
    config.proxy.clear();

    let history = state
        .history
        .query(&HistoryQuery {
            limit: u32::MAX,
            ..HistoryQuery::default()
        })
        .entries;

    let mut jobs = state.downloader.list();
    for job in jobs.iter_mut() {
        job.url = sanitize_url(&job.url);
        job.referer.clear();
        job.cookies_browser.clear();
        job.proxy.clear();
        job.extra_args.clear();
        for line in job.log.iter_mut() {
            *line = redact_text(line);
        }
    }

    let mut tasks = state.scheduler.list();
    for task in tasks.iter_mut() {
        task.payload.env.clear();
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

pub fn export_to(path: &Path, state: &AppState) -> Result<String, String> {
    let bundle = build(state);
    persist::save_json(path, &bundle)?;
    Ok(path.to_string_lossy().to_string())
}

pub fn import_from(path: &Path, state: &AppState) -> Result<String, String> {
    let raw = fs::read_to_string(path).map_err(|err| format!("[DATA] 读取备份失败：{err}"))?;
    let bundle: BackupBundle =
        serde_json::from_str(&raw).map_err(|err| format!("[DATA] 备份内容无法解析：{err}"))?;
    if bundle.schema_version != BUNDLE_SCHEMA_VERSION {
        return Err(format!(
            "[DATA] 备份版本不兼容（文件 {}，当前支持 {}）",
            bundle.schema_version, BUNDLE_SCHEMA_VERSION
        ));
    }

    let mut config = bundle.config;
    config::normalize(&mut config, &state.paths);
    config::clear_secrets(&mut config);
    config::validate(&config).map_err(|err| format!("[DATA] 备份配置无效：{err}"))?;

    if state.downloader.active_count() > 0 || state.scheduler.active_count() > 0 {
        return Err("[CONFLICT] 有任务正在运行，请先停止后再恢复备份".into());
    }

    let targets: [(PathBuf, Vec<u8>); 4] = [
        (
            state.paths.config_file.clone(),
            serde_json::to_vec_pretty(&config).map_err(|err| format!("[DATA] 配置序列化失败：{err}"))?,
        ),
        (
            state.paths.history_file.clone(),
            serde_json::to_vec_pretty(&bundle.history)
                .map_err(|err| format!("[DATA] 历史序列化失败：{err}"))?,
        ),
        (
            state.paths.jobs_file.clone(),
            serde_json::to_vec_pretty(&bundle.jobs)
                .map_err(|err| format!("[DATA] 下载记录序列化失败：{err}"))?,
        ),
        (
            state.paths.tasks_file.clone(),
            serde_json::to_vec_pretty(&bundle.tasks)
                .map_err(|err| format!("[DATA] 任务序列化失败：{err}"))?,
        ),
    ];

    let originals: Vec<Option<Vec<u8>>> = targets
        .iter()
        .map(|(path, _)| fs::read(path).ok())
        .collect();

    for (path, bytes) in targets.iter() {
        if let Err(err) = fs::write(path, bytes) {
            for (rollback_index, (rollback_path, _)) in targets.iter().enumerate() {
                if let Some(previous) = &originals[rollback_index] {
                    let _ = fs::write(rollback_path, previous);
                }
            }
            return Err(format!(
                "[DATA] 写入备份失败（已回滚）：{} {}",
                path.display(),
                err
            ));
        }
    }

    Ok(format!(
        "已恢复 {} 条历史、{} 个任务、{} 条下载记录",
        bundle.history.len(),
        bundle.tasks.len(),
        bundle.jobs.len()
    ))
}

fn sanitize_url(raw: &str) -> String {
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
            pairs
                .iter()
                .map(|(key, value)| (key.as_str(), if is_sensitive_key(key) { "***" } else { value.as_str() })),
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
mod tests {
    use super::*;

    #[test]
    fn sanitizes_url_credentials_and_sensitive_query() {
        let value = sanitize_url("https://user:pass@example.com/live.m3u8?token=abc&page=2");
        assert!(!value.contains("user"));
        assert!(!value.contains("pass"));
        assert!(!value.contains("abc"));
        assert!(value.contains("page=2"));
    }

    #[test]
    fn rejects_unknown_bundle_version() {
        let bundle = BackupBundle {
            schema_version: BUNDLE_SCHEMA_VERSION + 1,
            app_version: "0.1.0".into(),
            exported_at: String::new(),
            config: AppConfig::default(),
            tasks: Vec::new(),
            history: Vec::new(),
            jobs: Vec::new(),
        };
        let json = serde_json::to_string(&bundle).unwrap();
        let parsed: BackupBundle = serde_json::from_str(&json).unwrap();
        assert_ne!(parsed.schema_version, BUNDLE_SCHEMA_VERSION);
    }
}