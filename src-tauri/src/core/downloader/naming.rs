use std::path::Path;

use crate::models::{AppConfig, Job};

pub(super) const TIME_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

pub(super) const MEDIA_EXTS: [&str; 14] = [
    "mp4", "mkv", "webm", "ts", "m4v", "mov", "flv", "avi", "mp3", "m4a", "aac", "opus", "ogg",
    "wav",
];

pub(super) fn now_string() -> String {
    chrono::Local::now().format(TIME_FORMAT).to_string()
}

/// 画质档位对应的 yt-dlp 格式选择器（-f 值）。
/// 视频档位优先命中 MP4（视频流 ext=mp4 + 音频流 ext=m4a），
/// 站点不提供 MP4 时回退到任意格式的最佳组合。
pub(super) fn format_for(quality: &str) -> &'static str {
    match quality {
        "1080" => "bv*[height<=1080][ext=mp4]+ba[ext=m4a]/b[height<=1080][ext=mp4]/bv*[height<=1080]+ba/b[height<=1080]",
        "720" => "bv*[height<=720][ext=mp4]+ba[ext=m4a]/b[height<=720][ext=mp4]/bv*[height<=720]+ba/b[height<=720]",
        "480" => "bv*[height<=480][ext=mp4]+ba[ext=m4a]/b[height<=480][ext=mp4]/bv*[height<=480]+ba/b[height<=480]",
        "audio" => "bestaudio/best",
        _ => "bv*[ext=mp4]+ba[ext=m4a]/b[ext=mp4]/bv*+ba/b",
    }
}

/// 档位需要附加在 -f 之外的 yt-dlp 参数。
/// 视频档位强制合并容器为 MP4；音频档位提取并转码为 MP3（依赖 ffmpeg）。
pub(super) fn extra_flags_for(quality: &str) -> &'static [&'static str] {
    match quality {
        "audio" => &["-x", "--audio-format", "mp3"],
        _ => &["--merge-output-format", "mp4"],
    }
}

/// 校验字幕档位，未知值按“不下载”处理。
pub(super) fn normalize_subtitles(value: &str) -> String {
    match value.trim() {
        "zh" | "zh-mt" | "en" | "both" => value.trim().to_string(),
        _ => String::new(),
    }
}

/// 字幕档位对应的 yt-dlp 参数；站点没有字幕时会正常结束。
/// 手动与自动生成字幕一并尝试，能在 ffmpeg 可用时统一转成 SRT。
/// `skip_download` 为真时只抓字幕，不下载视频/音频。
///
/// 使用精确语言码而不是 `zh.*` 正则：正则会把 YouTube 的机翻变体
/// （如 zh-Hant-zh-CN）一并请求，请求数翻倍且容易触发 429。
/// `--sleep-subtitles` 让多条字幕串行下载，降低被限流的概率。
pub(super) fn subtitle_flags(
    subtitles: &str,
    convert_srt: bool,
    skip_download: bool,
) -> Vec<String> {
    // 机翻轨必须等 60 秒再下载：YouTube 对“自动翻译字幕”接口限流，
    // 官方给出的规避方式是 --sleep-subtitles 60（原字幕不受影响）。
    let (langs, sleep) = match subtitles.trim() {
        "zh" => ("zh,zh-Hans,zh-Hant,zh-CN,zh-TW", "1"),
        "zh-mt" => ("zh-Hans-.*", "60"),
        "en" => ("en,en-US,en-GB", "1"),
        "both" => ("zh,zh-Hans,zh-Hant,zh-CN,zh-TW,en,en-US,en-GB", "1"),
        _ => return Vec::new(),
    };
    let mut flags: Vec<String> = [
        "--write-subs",
        "--write-auto-subs",
        "--sub-langs",
        langs,
        "--sleep-subtitles",
        sleep,
    ]
    .iter()
    .map(|item| (*item).to_string())
    .collect();
    if skip_download {
        flags.push("--skip-download".into());
    }
    if convert_srt {
        flags.push("--convert-subs".into());
        flags.push("srt".into());
    }
    flags
}

pub(super) fn job_name(base: Option<&str>, index: usize, total: usize) -> String {
    let Some(base) = base else {
        return String::new();
    };
    if total > 1 {
        format!("{base} {:02}", index + 1)
    } else {
        base.to_string()
    }
}

pub(super) fn output_template(job: &Job) -> String {
    let template = if job.name.trim().is_empty() {
        "%(title)s [%(id)s].%(ext)s".to_string()
    } else {
        format!("{}.%(ext)s", escape_template(&job.name))
    };
    Path::new(&job.out_dir)
        .join(template)
        .to_string_lossy()
        .to_string()
}

pub(super) fn effective_threads(job: &Job, config: &AppConfig) -> u32 {
    let threads = if job.threads == 0 {
        config.fragment_threads
    } else {
        job.threads
    };
    threads.clamp(1, 16)
}

pub(super) fn sanitize_file_name(input: &str) -> String {
    let cleaned: String = input
        .chars()
        .map(|ch| {
            if ch.is_control() || matches!(ch, '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|')
            {
                '_'
            } else {
                ch
            }
        })
        .collect();
    let without_ext = strip_media_ext(cleaned.trim());
    let trimmed = without_ext.trim().trim_end_matches(['.', ' ']).trim();
    let mut name: String = trimmed.chars().take(120).collect();
    if is_reserved_name(&name) {
        name.insert(0, '_');
    }
    name
}

fn strip_media_ext(name: &str) -> &str {
    let lower = name.to_ascii_lowercase();
    for ext in MEDIA_EXTS {
        let suffix = format!(".{ext}");
        if lower.ends_with(&suffix) {
            return &name[..name.len() - suffix.len()];
        }
    }
    name
}

fn is_reserved_name(name: &str) -> bool {
    let upper = name.to_ascii_uppercase();
    let stem = upper.split('.').next().unwrap_or_default();
    matches!(stem, "CON" | "PRN" | "AUX" | "NUL")
        || (stem.len() == 4
            && (stem.starts_with("COM") || stem.starts_with("LPT"))
            && stem.as_bytes()[3].is_ascii_digit())
}

pub(super) fn escape_template(name: &str) -> String {
    name.replace('%', "%%")
}
