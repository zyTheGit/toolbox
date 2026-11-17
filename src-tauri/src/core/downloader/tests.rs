use super::*;
use super::args::*;
use super::naming::*;
use super::parse::*;

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
