//! 下载凭据不落盘的 canary、旧文件迁移与重启重试测试。

use std::path::{Path, PathBuf};
use std::sync::Arc;

use parking_lot::{Mutex, RwLock};
use uuid::Uuid;

use crate::core::persist;
use crate::models::{AppConfig, DownloadRequest, Job, JobRetryRequest};
use crate::state::{AppPaths, SharedConfig, Warnings};

use super::credentials::{strip_job, CREDENTIALS_MISSING};
use super::Downloader;

const CANARY: &str = "canary-7f3a9c";

struct Fixture {
    dir: PathBuf,
    paths: AppPaths,
    config: SharedConfig,
    warnings: Warnings,
}

fn fixture() -> Fixture {
    let dir = std::env::temp_dir().join(format!("toolbox-dl-cred-{}", Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    Fixture {
        paths: AppPaths::from_data_dir(dir.clone()),
        dir,
        config: Arc::new(RwLock::new(AppConfig::default())),
        warnings: Arc::new(Mutex::new(Vec::new())),
    }
}

impl Fixture {
    fn downloader(&self) -> Downloader {
        Downloader::for_test(self.paths.clone(), self.config.clone(), self.warnings.clone())
    }

    fn config(&self) -> AppConfig {
        self.config.read().clone()
    }

    fn disk_text(&self) -> String {
        std::fs::read_to_string(&self.paths.jobs_file).unwrap()
    }

    fn write_jobs(&self, jobs: &[Job]) {
        persist::save_json(&self.paths.jobs_file, jobs).unwrap();
    }

    fn read_jobs(&self) -> Vec<Job> {
        persist::load_json::<Vec<Job>>(&self.paths.jobs_file)
            .unwrap()
            .unwrap_or_default()
    }
}

fn canary_request(out_dir: &Path) -> DownloadRequest {
    DownloadRequest {
        urls: vec!["https://example.com/live/index.m3u8".into()],
        out_dir: Some(out_dir.to_string_lossy().to_string()),
        referer: format!("https://ref.example/play?token={CANARY}"),
        cookies_browser: "edge".into(),
        proxy: format!("socks5://user:{CANARY}@127.0.0.1:7891"),
        extra_args: format!("--add-header \"Authorization: Bearer {CANARY}\""),
        ..Default::default()
    }
}

fn legacy_job(id: &str, status: &str) -> Job {
    Job {
        id: id.into(),
        url: "https://example.com/live/index.m3u8".into(),
        quality: "best".into(),
        out_dir: std::env::temp_dir().to_string_lossy().to_string(),
        status: status.into(),
        referer: format!("https://ref.example/play?token={CANARY}"),
        cookies_browser: "edge".into(),
        proxy: format!("socks5://user:{CANARY}@127.0.0.1:7891"),
        extra_args: format!("--add-header \"Authorization: Bearer {CANARY}\""),
        ..Job::default()
    }
}

#[test]
fn enqueue_persists_no_credentials() {
    let fixture = fixture();
    let downloader = fixture.downloader();
    let config = fixture.config();
    let created = downloader
        .enqueue(&canary_request(&fixture.dir), &config)
        .unwrap();

    assert_eq!(created.len(), 1);
    let job = &created[0];
    assert!(job.has_credentials);
    assert!(job.referer.is_empty());
    assert!(job.cookies_browser.is_empty());
    assert!(job.proxy.is_empty());
    assert!(job.extra_args.is_empty());

    // jobs.json 0 命中 canary。
    let text = fixture.disk_text();
    assert!(!text.contains(CANARY), "jobs.json 不应包含凭据");
    assert!(text.contains("\"hasCredentials\": true"));

    // 列表 / 快照 0 命中。
    let listed = serde_json::to_string(&downloader.list()).unwrap();
    assert!(!listed.contains(CANARY));

    // 凭据仍在内存中，本会话重试无需重填。
    let stored = downloader.job_credentials(&job.id).unwrap();
    assert!(stored.proxy.contains(CANARY));
    assert!(stored.extra_args.contains(CANARY));

    downloader.cancel(&job.id).unwrap();
    let retried = downloader.retry(&job.id, None).unwrap();
    assert_eq!(retried.status, "queued");
    assert!(retried.has_credentials);

    // 事件载荷与落盘共用同一剥离函数。
    let mut payload = retried.clone();
    payload.proxy = format!("http://{CANARY}:1");
    strip_job(&mut payload);
    assert!(!serde_json::to_string(&payload).unwrap().contains(CANARY));
}

#[test]
fn defensive_strip_protects_list_and_disk_for_other_job_entries() {
    let fixture = fixture();
    let downloader = fixture.downloader();
    {
        let mut inner = downloader.inner.lock();
        inner.jobs.push(Job {
            id: "direct-entry".into(),
            url: "https://example.com/a.m3u8".into(),
            status: "failed".into(),
            referer: format!("https://r.example/{CANARY}"),
            cookies_browser: format!("{CANARY}-browser"),
            proxy: format!("http://{CANARY}:1"),
            extra_args: format!("--add-header \"X-Token: {CANARY}\""),
            ..Job::default()
        });
    }

    let listed = serde_json::to_string(&downloader.list()).unwrap();
    assert!(!listed.contains(CANARY));
    downloader.save();
    assert!(!fixture.disk_text().contains(CANARY));
}

#[test]
fn legacy_plaintext_jobs_are_migrated_and_scrubbed() {
    let fixture = fixture();
    fixture.write_jobs(&[legacy_job("legacy-1", "interrupted")]);
    assert!(fixture.disk_text().contains(CANARY));

    let downloader = fixture.downloader();

    // 磁盘明文消失，且字段已清空、标记为需要凭据。
    let text = fixture.disk_text();
    assert!(!text.contains(CANARY), "迁移后 jobs.json 不应残留明文");
    let jobs = fixture.read_jobs();
    assert_eq!(jobs.len(), 1);
    assert!(jobs[0].referer.is_empty());
    assert!(jobs[0].proxy.is_empty());
    assert!(jobs[0].extra_args.is_empty());
    assert!(jobs[0].has_credentials);

    // 内存中仍有凭据。
    assert!(downloader.job_credentials("legacy-1").is_some());
    let warnings = fixture.warnings.lock().clone();
    assert!(
        warnings.iter().any(|item| item.contains("明文凭据")),
        "迁移应记录一次中文提示: {warnings:?}"
    );
}

#[test]
fn restart_requires_refill_then_retry_with_request_succeeds() {
    let fixture = fixture();
    // 模拟旧版本写下的明文 jobs.json（queued 会被恢复为 interrupted）。
    fixture.write_jobs(&[legacy_job("legacy-2", "queued")]);
    {
        let first = fixture.downloader();
        assert!(first.job_credentials("legacy-2").is_some());
    }

    // “重启”：重新构造 Downloader，磁盘上是新格式（hasCredentials=true 且字段为空）。
    let restarted = fixture.downloader();
    let err = restarted.retry("legacy-2", None).unwrap_err();
    assert_eq!(err, CREDENTIALS_MISSING);
    assert!(err.starts_with("[CREDENTIALS]"));
    assert!(err.contains("重新填写"));

    let request = JobRetryRequest {
        proxy: Some("http://127.0.0.1:1080".into()),
        referer: Some("https://ref.example/again".into()),
        quality: Some("720".into()),
        threads: Some(4),
        ..Default::default()
    };
    let job = restarted.retry("legacy-2", Some(request)).unwrap();
    assert_eq!(job.status, "queued");
    assert_eq!(job.quality, "720");
    assert_eq!(job.threads, 4);
    assert!(job.has_credentials);
    assert!(job.proxy.is_empty());
    assert!(job.referer.is_empty());

    let stored = restarted.job_credentials("legacy-2").unwrap();
    assert_eq!(stored.proxy, "http://127.0.0.1:1080");

    // 新凭据只在内存：磁盘不出现补填的代理。
    assert!(!fixture.disk_text().contains("127.0.0.1:1080"));

    // 本会话内再次重试直接成功，不再要求重填。
    restarted.cancel("legacy-2").unwrap();
    let again = restarted.retry("legacy-2", None).unwrap();
    assert_eq!(again.status, "queued");
}

#[test]
fn retry_without_credentials_is_unaffected() {
    let fixture = fixture();
    fixture.write_jobs(&[Job {
        id: "plain-1".into(),
        url: "https://example.com/a.m3u8".into(),
        status: "interrupted".into(),
        ..Job::default()
    }]);

    let restarted = fixture.downloader();
    let job = restarted.retry("plain-1", None).unwrap();
    assert_eq!(job.status, "queued");
    assert!(!job.has_credentials);
}
