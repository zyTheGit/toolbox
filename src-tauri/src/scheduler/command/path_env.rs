//! 刷新子进程 PATH：驻留进程继承的环境是启动时的快照，
//! 之后安装的命令行工具（或其目录）不会自动可见。
//!
//! - Windows：合并注册表中的系统级 / 用户级 PATH；
//! - macOS / Linux：补充常见但当前 PATH 缺失的命令目录（GUI 启动时环境往往不完整）。

use std::collections::HashSet;

#[cfg(windows)]
use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ};
#[cfg(windows)]
use winreg::{RegKey, HKEY};

#[cfg(windows)]
use super::vars::expand_vars;

#[cfg(windows)]
const MACHINE_ENV: &str = r"SYSTEM\CurrentControlSet\Control\Session Manager\Environment";
#[cfg(windows)]
const USER_ENV: &str = "Environment";

const SEP: char = if cfg!(windows) { ';' } else { ':' };

pub fn merged_path() -> Option<String> {
    let process_path = std::env::var_os("PATH")
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_default();
    let extras = extra_entries();
    let merged = merge(&process_path, &extras);
    if merged.is_empty() {
        None
    } else {
        Some(merged)
    }
}

/// 进程 PATH 在前，扩展目录按顺序补充，重复目录忽略。
fn merge(process_path: &str, extras: &[String]) -> String {
    let mut seen: HashSet<String> = HashSet::new();
    let mut entries: Vec<&str> = Vec::new();
    for entry in process_path
        .split(SEP)
        .chain(extras.iter().map(String::as_str))
    {
        let trimmed = entry.trim();
        if trimmed.is_empty() {
            continue;
        }
        if seen.insert(normalize(trimmed)) {
            entries.push(trimmed);
        }
    }
    entries.join(&SEP.to_string())
}

#[cfg(windows)]
fn normalize(entry: &str) -> String {
    let trimmed = entry.trim();
    let no_tail = trimmed.trim_end_matches(|ch| ch == '\\' || ch == '/');
    // 盘符根目录（如 C:\）不能简写成 C:，二者语义不同。
    let kept = if no_tail.len() == 2 && no_tail.ends_with(':') {
        trimmed
    } else {
        no_tail
    };
    kept.replace('/', "\\").to_ascii_lowercase()
}

#[cfg(unix)]
fn normalize(entry: &str) -> String {
    let trimmed = entry.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        "/".to_string()
    } else {
        trimmed.to_string()
    }
}

#[cfg(windows)]
fn extra_entries() -> Vec<String> {
    let machine = read_path(HKEY_LOCAL_MACHINE, MACHINE_ENV);
    let user = read_path(HKEY_CURRENT_USER, USER_ENV);
    [machine, user]
        .into_iter()
        .flatten()
        .flat_map(|value| value.split(';').map(expand_vars).collect::<Vec<String>>())
        .collect()
}

#[cfg(windows)]
fn read_path(hive: HKEY, subkey: &str) -> Option<String> {
    let key = RegKey::predef(hive)
        .open_subkey_with_flags(subkey, KEY_READ)
        .ok()?;
    key.get_value::<String, _>("Path").ok()
}

#[cfg(unix)]
fn extra_entries() -> Vec<String> {
    let mut dirs: Vec<String> = vec![
        "/usr/local/bin".into(),
        "/usr/bin".into(),
        "/bin".into(),
        "/usr/sbin".into(),
        "/sbin".into(),
    ];
    #[cfg(target_os = "macos")]
    dirs.extend(
        ["/opt/homebrew/bin", "/opt/homebrew/sbin", "/opt/local/bin"]
            .into_iter()
            .map(String::from),
    );
    #[cfg(target_os = "linux")]
    dirs.extend(
        ["/snap/bin", "/nix/var/nix/profiles/default/bin"]
            .into_iter()
            .map(String::from),
    );
    if let Some(home) = crate::core::config::home_dir() {
        let home = home.to_string_lossy();
        for rel in [
            ".local/bin",
            "bin",
            ".cargo/bin",
            ".bun/bin",
            ".deno/bin",
            ".nix-profile/bin",
        ] {
            dirs.push(format!("{home}/{rel}"));
        }
    }
    dirs.retain(|dir| std::path::Path::new(dir).is_dir());
    dirs
}

#[cfg(all(test, windows))]
mod windows_tests {
    use super::*;

    #[test]
    fn merges_registry_entries_without_duplicates() {
        let path = merge(
            r"C:\A;;c:\b\",
            &["c:/A".to_string(), r"C:\C".to_string(), r"c:\c\".to_string()],
        );
        assert_eq!(path, r"C:\A;c:\b\;C:\C");
    }

    #[test]
    fn keeps_registry_only_when_process_path_is_empty() {
        assert_eq!(merge("", &[r"C:\X".to_string()]), r"C:\X");
    }
}

#[cfg(all(test, unix))]
mod unix_tests {
    use super::*;

    #[test]
    fn merges_extra_directories_without_duplicates() {
        let path = merge(
            "/usr/bin:/bin",
            &["/usr/bin/".to_string(), "/opt/homebrew/bin".to_string()],
        );
        assert_eq!(path, "/usr/bin:/bin:/opt/homebrew/bin");
    }

    #[test]
    fn keeps_registry_only_when_process_path_is_empty() {
        assert_eq!(merge("", &["/usr/local/bin".to_string()]), "/usr/local/bin");
    }
}
