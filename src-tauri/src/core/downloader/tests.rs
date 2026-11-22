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
    // 视频档位优先命中 MP4，站点不提供时回退任意最佳组合。
    assert_eq!(format_for("best"), "bv*[ext=mp4]+ba[ext=m4a]/b[ext=mp4]/bv*+ba/b");
    assert_eq!(
        format_for("1080"),
        "bv*[height<=1080][ext=mp4]+ba[ext=m4a]/b[height<=1080][ext=mp4]/bv*[height<=1080]+ba/b[height<=1080]"
    );
    assert_eq!(format_for("unknown"), "bv*[ext=mp4]+ba[ext=m4a]/b[ext=mp4]/bv*+ba/b");
    assert_eq!(format_for("audio"), "bestaudio/best");
}

#[test]
fn extra_flags_match_quality() {
    // 视频档位强制合并容器为 MP4。
    assert_eq!(extra_flags_for("best"), &["--merge-output-format", "mp4"]);
    assert_eq!(extra_flags_for("720"), &["--merge-output-format", "mp4"]);
    // 音频档位提取并转码为 MP3。
    assert_eq!(extra_flags_for("audio"), &["-x", "--audio-format", "mp3"]);
}

#[test]
fn detects_chinese_subtitle_content() {
    assert!(looks_chinese("你好世界，这是中文字幕。"));
    assert!(looks_chinese("中文 with 少量 English"));
    assert!(!looks_chinese(
        "President, Nvidia's investigating Jensen Huang's relationship."
    ));
    assert!(!looks_chinese(""));
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
    // 凭据不再随 Job 落盘，这里的运行参数只保留文件名、线程与画质。
    let mut job = Job {
        status: "running".into(),
        name: "视频".into(),
        quality: "720".into(),
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
    assert_eq!(job.quality, "720");
    assert_eq!(job.threads, 8);
    assert!(job.referer.is_empty());
    assert!(job.proxy.is_empty());

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
fn subtitle_flags_follow_choice() {
    assert!(subtitle_flags("", true, false).is_empty());
    assert!(subtitle_flags("unknown", true, false).is_empty());
    // 精确语言码：避免正则把机翻变体（zh-Hant-zh-CN 等）也请求下来。
    assert_eq!(
        subtitle_flags("zh", false, false).join(" "),
        "--write-subs --write-auto-subs --sub-langs zh,zh-Hans,zh-Hant,zh-CN,zh-TW --sleep-subtitles 1"
    );
    let both = subtitle_flags("both", true, false).join(" ");
    assert!(both.contains("--convert-subs srt"));
    assert!(both.contains("--sub-langs zh,zh-Hans,zh-Hant,zh-CN,zh-TW,en,en-US,en-GB"));
    assert!(!both.contains("zh.*"));
    assert!(!both.contains("--skip-download"));

    // 仅字幕：跳过媒体下载，字幕参数保持不变。
    let only = subtitle_flags("en", false, true).join(" ");
    assert!(only.contains("--skip-download"));
    assert!(only.contains("--sub-langs en,en-US,en-GB"));

    // 机翻中文：只匹配简体翻译轨，并等 60 秒规避 YouTube 的 429 限流。
    let mt = subtitle_flags("zh-mt", false, false).join(" ");
    assert!(mt.contains("--sub-langs zh-Hans-.*"));
    assert!(mt.contains("--sleep-subtitles 60"));
    assert!(!mt.contains("zh-CN,"));
}

#[test]
fn normalizes_subtitle_choice() {
    assert_eq!(normalize_subtitles(" zh "), "zh");
    assert_eq!(normalize_subtitles("zh-mt"), "zh-mt");
    assert_eq!(normalize_subtitles("both"), "both");
    assert_eq!(normalize_subtitles("ALL"), "");
}

#[test]
fn parses_download_request_options_over_ipc() {
    let request: DownloadRequest = serde_json::from_str(
        r#"{"urls":["https://example.com/a.m3u8"],"quality":"best","fileName":"我的视频","threads":8,"subtitles":"both","subtitleOnly":true}"#,
    )
    .unwrap();
    assert_eq!(request.file_name.as_deref(), Some("我的视频"));
    assert_eq!(request.threads, Some(8));
    assert_eq!(request.subtitles, "both");
    assert!(request.subtitle_only);
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
    assert!(legacy.subtitles.is_empty());
    assert!(!legacy.subtitle_only);
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

#[test]
fn sniffs_container_from_file_header() {
    let mut mp4 = vec![0u8; 32];
    mp4[4..8].copy_from_slice(b"ftyp");
    mp4[8..12].copy_from_slice(b"isom");
    assert_eq!(sniff_magic(&mp4), Some("mp4"));

    let mut m4a = vec![0u8; 32];
    m4a[4..8].copy_from_slice(b"ftyp");
    m4a[8..12].copy_from_slice(b"M4A ");
    assert_eq!(sniff_magic(&m4a), Some("m4a"));

    let webm = b"\x1A\x45\xDF\xA3\x9F\x42\x86\x81\x01\x42\x82\x84webm";
    assert_eq!(sniff_magic(webm), Some("webm"));
    let mkv = b"\x1A\x45\xDF\xA3\x9F\x42\x86\x81\x01\x42\x82\x88matroska";
    assert_eq!(sniff_magic(mkv), Some("mkv"));

    assert_eq!(
        sniff_magic(b"FLV\x01\x05\x00\x00\x00\x00\x09\x00\x00\x00\x00"),
        Some("flv")
    );
    assert_eq!(
        sniff_magic(b"RIFF\x00\x00\x00\x00WAVEfmt "),
        Some("wav")
    );
    assert_eq!(sniff_magic(&[0xFF, 0xFB, 0x90, 0x00, 0, 0, 0, 0, 0, 0, 0, 0]), Some("mp3"));

    let mut ts = vec![0u8; 377];
    ts[0] = 0x47;
    ts[188] = 0x47;
    ts[376] = 0x47;
    assert_eq!(sniff_magic(&ts), Some("ts"));

    // 字幕等文本文件不做重命名。
    assert_eq!(sniff_magic(b"1\r\n00:00:01,000 --> 00:00:02,000"), None);
    assert_eq!(sniff_magic(b"short"), None);
}

#[test]
fn finalizes_unknown_video_extension() {
    let dir = std::env::temp_dir().join(format!("toolbox-finalize-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let src = dir.join("clip [id].unknown_video");
    let mut head = vec![0u8; 32];
    head[4..8].copy_from_slice(b"ftyp");
    head[8..12].copy_from_slice(b"isom");
    std::fs::write(&src, &head).unwrap();

    let out = finalize_media_ext(&src.to_string_lossy());
    assert!(out.ends_with("clip [id].mp4"), "{out}");
    assert!(std::path::Path::new(&out).exists());
    assert!(!src.exists());

    // 已知扩展名原样返回，不触碰文件。
    assert_eq!(finalize_media_ext("C:\\out\\a.mp4"), "C:\\out\\a.mp4");
    assert_eq!(finalize_media_ext(""), "");
    std::fs::remove_dir_all(&dir).ok();
}

/// 备份恢复后的重载入口：磁盘上的记录必须可读回，且不得自动恢复下载。
#[test]
fn reload_reads_restored_jobs_from_disk() {
    let dir = std::env::temp_dir().join(format!("toolbox-jobs-reload-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("jobs.json");

    let restored = Job {
        id: "restored-job".into(),
        url: "https://example.com/video.m3u8".into(),
        status: "interrupted".into(),
        error: "来自备份，需重新确认后重试".into(),
        has_credentials: false,
        ..Job::default()
    };
    crate::core::persist::save_json(&path, &[restored]).unwrap();

    let jobs = super::read_jobs(&path).unwrap().unwrap();
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].id, "restored-job");
    assert!(jobs[0].referer.is_empty());
    assert!(jobs[0].proxy.is_empty());
    assert_eq!(jobs[0].status, "interrupted");
    assert!(!is_active(&jobs[0]));
}
