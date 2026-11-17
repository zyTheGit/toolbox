use std::path::{Path, PathBuf};
use std::time::SystemTime;

use tokio::io::{AsyncBufReadExt, BufReader};

use crate::models::{Job, JobProgress};

use super::naming::{MEDIA_EXTS, TIME_FORMAT};
use super::Downloader;

pub(super) async fn read_lines<R>(reader: Option<R>, downloader: Downloader, id: String)
where
    R: tokio::io::AsyncRead + Unpin,
{
    let Some(reader) = reader else {
        return;
    };
    let mut reader = BufReader::new(reader);
    let mut buffer = Vec::new();
    loop {
        buffer.clear();
        match reader.read_until(b'\n', &mut buffer).await {
            Ok(0) => break,
            Ok(_) => {
                let text = decode_line(&buffer);
                let cleaned = strip_ansi(&text);
                let line = cleaned.trim_end_matches(['\r', '\n']);
                if !line.trim().is_empty() {
                    downloader.apply_line(&id, line);
                }
            }
            Err(_) => break,
        }
    }
}

fn decode_line(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(text) => text.to_string(),
        Err(_) => {
            let (decoded, _, _) = encoding_rs::GBK.decode(bytes);
            decoded.into_owned()
        }
    }
}

pub(super) fn strip_ansi(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' {
            if chars.peek() == Some(&'[') {
                chars.next();
                while let Some(&next) = chars.peek() {
                    chars.next();
                    if next.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
            continue;
        }
        output.push(ch);
    }
    output
}

pub(super) fn parse_progress(line: &str) -> Option<JobProgress> {
    if !line.starts_with("[download]") {
        return None;
    }
    let tokens: Vec<&str> = line.split_whitespace().collect();
    let mut percent: Option<f64> = None;
    let mut total = String::new();
    let mut speed = String::new();
    let mut eta = String::new();

    for (index, token) in tokens.iter().enumerate() {
        if percent.is_none() {
            if let Some(value) = token.strip_suffix('%') {
                if let Ok(parsed) = value.parse::<f64>() {
                    percent = Some(parsed);
                }
            }
        }
        match *token {
            "of" => {
                let mut cursor = index + 1;
                if tokens.get(cursor) == Some(&"~") {
                    cursor += 1;
                }
                if let Some(value) = tokens.get(cursor) {
                    if looks_like_size(value) {
                        total = (*value).to_string();
                    }
                }
            }
            "at" => {
                if let Some(value) = tokens.get(index + 1) {
                    speed = clean_metric(value);
                }
            }
            "ETA" => {
                if let Some(value) = tokens.get(index + 1) {
                    eta = clean_metric(value);
                }
            }
            _ => {}
        }
    }

    let percent = percent?;
    let downloaded = if percent >= 100.0 {
        total.clone()
    } else {
        String::new()
    };
    Some(JobProgress {
        percent,
        speed,
        eta,
        downloaded,
        total,
    })
}

fn looks_like_size(token: &str) -> bool {
    match token.chars().next() {
        Some(ch) if ch.is_ascii_digit() => {}
        _ => return false,
    }
    token.chars().any(|ch| ch.is_ascii_alphabetic())
}

fn clean_metric(value: &str) -> String {
    if value.eq_ignore_ascii_case("unknown") || value == "?" || value == "N/A" {
        String::new()
    } else {
        value.to_string()
    }
}

pub(super) fn parse_file(line: &str) -> Option<String> {
    if let Some(rest) = line.strip_prefix("[download] Destination: ") {
        return Some(unquote(rest));
    }
    if let Some(rest) = line.strip_prefix("[Merger] Merging formats into ") {
        return Some(unquote(rest));
    }
    if let Some(rest) = line.strip_prefix("[download] ") {
        if let Some(name) = rest.strip_suffix(" has already been downloaded") {
            return Some(unquote(name));
        }
    }
    None
}

fn unquote(value: &str) -> String {
    let trimmed = value.trim();
    let trimmed = trimmed.strip_prefix('"').unwrap_or(trimmed);
    let trimmed = trimmed.strip_suffix('"').unwrap_or(trimmed);
    trimmed.trim().to_string()
}

pub(super) fn split_args(input: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    let mut has_token = false;
    let mut chars = input.chars();

    while let Some(ch) = chars.next() {
        match quote {
            Some(active) => {
                if ch == '\\' {
                    if let Some(next) = chars.next() {
                        current.push(next);
                    }
                } else if ch == active {
                    quote = None;
                } else {
                    current.push(ch);
                }
            }
            None => {
                if ch == '"' || ch == '\'' {
                    quote = Some(ch);
                    has_token = true;
                } else if ch == '\\' {
                    if let Some(next) = chars.next() {
                        current.push(next);
                        has_token = true;
                    }
                } else if ch.is_whitespace() {
                    if has_token {
                        args.push(std::mem::take(&mut current));
                        has_token = false;
                    }
                } else {
                    current.push(ch);
                    has_token = true;
                }
            }
        }
    }
    if has_token {
        args.push(current);
    }
    args
}

pub(super) fn failure_message(job: &Job, code: Option<i32>) -> String {
    for line in job.log.iter().rev() {
        let text = line.trim();
        if text.starts_with("ERROR:") {
            return truncate_chars(text, 500);
        }
    }

    let mut tail: Vec<String> = job
        .log
        .iter()
        .rev()
        .filter(|line| {
            let text = line.trim();
            !text.is_empty() && !text.starts_with("[download]")
        })
        .take(3)
        .map(|line| line.trim().to_string())
        .collect();
    tail.reverse();

    if !tail.is_empty() {
        return truncate_chars(&tail.join(" / "), 500);
    }

    match code {
        Some(value) => format!("yt-dlp 退出码 {value}"),
        None => "yt-dlp 异常退出".into(),
    }
}

pub(super) fn tag_failure(message: String) -> String {
    if message.starts_with('[') {
        return message;
    }
    let lower = message.to_ascii_lowercase();
    let code = if lower.contains("timed out") || lower.contains("timeout") {
        "TIMEOUT"
    } else if lower.contains("permission denied") || lower.contains("access is denied") {
        "PERMISSION"
    } else if lower.contains("no space") || lower.contains("disk full") {
        "PATH"
    } else if lower.contains("unsupported url") || lower.contains("invalid url") {
        "PARAM"
    } else if lower.contains("unable to download")
        || lower.contains("connection")
        || lower.contains("http error")
        || lower.contains("network")
        || lower.contains("unable to connect")
    {
        "NETWORK"
    } else if lower.contains("ffmpeg") && lower.contains("not found") {
        "TOOL"
    } else {
        return message;
    };
    format!("[{code}] {message}")
}

fn truncate_chars(value: &str, max: usize) -> String {
    if value.chars().count() <= max {
        value.to_string()
    } else {
        value.chars().take(max).collect()
    }
}

pub(super) fn scan_output_file(job: &Job) -> String {
    let dir = Path::new(&job.out_dir);
    let since: SystemTime = match parse_time(&job.started_at) {
        Some(stamp) => (stamp - chrono::Duration::seconds(10)).into(),
        None => (chrono::Local::now() - chrono::Duration::hours(1)).into(),
    };

    let mut best: Option<(SystemTime, PathBuf)> = None;
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() || !is_media(&path) {
                continue;
            }
            let Ok(metadata) = entry.metadata() else {
                continue;
            };
            let Ok(modified) = metadata.modified() else {
                continue;
            };
            if modified < since {
                continue;
            }
            if best.as_ref().map(|(time, _)| modified > *time).unwrap_or(true) {
                best = Some((modified, path));
            }
        }
    }

    best.map(|(_, path)| path.to_string_lossy().to_string())
        .unwrap_or_default()
}

fn parse_time(value: &str) -> Option<chrono::DateTime<chrono::Local>> {
    use chrono::TimeZone;
    chrono::NaiveDateTime::parse_from_str(value, TIME_FORMAT)
        .ok()
        .and_then(|naive| chrono::Local.from_local_datetime(&naive).single())
}

fn is_media(path: &Path) -> bool {
    let ext = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    MEDIA_EXTS.contains(&ext.as_str())
}
