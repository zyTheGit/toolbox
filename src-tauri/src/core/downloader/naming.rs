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
