//! 断点续传配套：清理 yt-dlp 留下的未完成临时文件。
//!
//! 临时产物与成品同目录，均以成品主干开头：
//! `<主干>.<扩展>.part`、`<主干>.<扩展>.part-FragN(.part)`、`<主干>.<扩展>.ytdl`。

use std::path::Path;

use crate::models::Job;

use super::naming::MEDIA_EXTS;
use super::parse::parse_file;
use super::state::is_active;
use super::Downloader;

/// 任务产物的文件名主干（不含媒体扩展名），用于匹配临时文件前缀。
///
/// 优先用运行中解析到的成品路径；重启后未记录成品时退回到任务名，
/// 最后从日志里的 `Destination` 行恢复。
pub(super) fn temp_stem(job: &Job) -> Option<String> {
    if let Some(stem) = file_stem(&job.file) {
        return Some(stem);
    }
    let name = job.name.trim();
    if !name.is_empty() {
        return Some(name.to_string());
    }
    job.log
        .iter()
        .rev()
        .find_map(|line| parse_file(line).and_then(|file| file_stem(&file)))
}

fn file_stem(value: &str) -> Option<String> {
    let name = Path::new(value.trim())
        .file_name()?
        .to_string_lossy()
        .to_string();
    if name.is_empty() {
        return None;
    }
    let lower = name.to_ascii_lowercase();
    for ext in MEDIA_EXTS {
        let suffix = format!(".{ext}");
        if lower.ends_with(&suffix) {
            return Some(name[..name.len() - suffix.len()].to_string());
        }
    }
    Some(name)
}

/// `.part`、`.part-FragN`、`.ytdl` 以及合并临时文件都视为未完成产物。
fn is_temp_artifact(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.ends_with(".part")
        || lower.ends_with(".ytdl")
        || lower.contains(".part-frag")
        || lower.contains(".temp.")
}

/// 删除属于该任务的临时文件，返回删除数量；成品文件不受影响。
pub(super) fn cleanup_temps(job: &Job) -> usize {
    match temp_stem(job) {
        Some(stem) => cleanup_temps_with_stem(job, &stem),
        None => 0,
    }
}

fn cleanup_temps_with_stem(job: &Job, stem: &str) -> usize {
    let dir = Path::new(job.out_dir.trim());
    let Ok(entries) = std::fs::read_dir(dir) else {
        return 0;
    };
    let prefix = format!("{}.", stem.to_ascii_lowercase());
    let mut removed = 0;
    for entry in entries.flatten() {
        let Some(name) = entry.file_name().to_str().map(str::to_string) else {
            continue;
        };
        if !name.to_ascii_lowercase().starts_with(&prefix) || !is_temp_artifact(&name) {
            continue;
        }
        let path = entry.path();
        if path.is_file() && std::fs::remove_file(&path).is_ok() {
            removed += 1;
        }
    }
    removed
}

impl Downloader {
    /// 清理该任务的临时文件。
    ///
    /// 同目录存在同主干的进行中任务时跳过，避免误删另一个任务的断点。
    pub(super) fn cleanup_temps_for(&self, job: &Job) -> usize {
        let Some(stem) = temp_stem(job) else {
            return 0;
        };
        let conflict = self.inner.lock().jobs.iter().any(|other| {
            other.id != job.id
                && is_active(other)
                && other.out_dir.eq_ignore_ascii_case(&job.out_dir)
                && temp_stem(other)
                    .map(|other_stem| other_stem.eq_ignore_ascii_case(&stem))
                    .unwrap_or(false)
        });
        if conflict {
            return 0;
        }
        cleanup_temps_with_stem(job, &stem)
    }
}
