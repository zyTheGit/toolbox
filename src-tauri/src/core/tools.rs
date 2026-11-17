use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use futures_util::StreamExt;
use reqwest::Client;
use tauri::{AppHandle, Emitter};
use tokio::io::AsyncWriteExt;

use crate::core::config::expand_home;
use crate::models::{AppConfig, ToolInstallResult, ToolProgress, ToolStatus, ToolsStatus};
use crate::state::AppPaths;

const UA: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Safari/537.36 Toolbox/0.1";
const CACHE_TTL: Duration = Duration::from_secs(30);
const EMIT_INTERVAL: Duration = Duration::from_millis(300);
const EMIT_BYTES: u64 = 1024 * 1024;
const FFMPEG_ZIP: &str = "https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip";

struct CacheEntry {
    at: Instant,
    status: ToolStatus,
}

static CACHE: OnceLock<Mutex<HashMap<String, CacheEntry>>> = OnceLock::new();

fn cache() -> std::sync::MutexGuard<'static, HashMap<String, CacheEntry>> {
    CACHE
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

pub fn get_status(config: &AppConfig, paths: &AppPaths, force: bool) -> ToolsStatus {
    ToolsStatus {
        ytdlp: status_for("ytdlp", &config.tool_paths.ytdlp, paths, force),
        ffmpeg: status_for("ffmpeg", &config.tool_paths.ffmpeg, paths, force),
    }
}

fn status_for(tool: &str, configured: &str, paths: &AppPaths, force: bool) -> ToolStatus {
    let Some(path) = locate(tool, configured, paths) else {
        return ToolStatus::default();
    };
    let path_text = path.to_string_lossy().to_string();

    if !force {
        if let Some(entry) = cache().get(tool) {
            if entry.status.path == path_text && entry.at.elapsed() < CACHE_TTL {
                return entry.status.clone();
            }
        }
    }

    let version = read_version(tool, &path);
    let status = ToolStatus {
        ok: !version.is_empty(),
        path: path_text,
        version,
    };
    cache().insert(
        tool.to_string(),
        CacheEntry {
            at: Instant::now(),
            status: status.clone(),
        },
    );
    status
}

fn locate(tool: &str, configured: &str, paths: &AppPaths) -> Option<PathBuf> {
    let configured = expand_home(configured.trim());
    if !configured.trim().is_empty() {
        let path = PathBuf::from(configured);
        if path.is_file() {
            return Some(path);
        }
    }

    let bundled = paths.bin_dir.join(exe_file(tool));
    if bundled.is_file() {
        return Some(bundled);
    }

    let binary = if tool == "ytdlp" { "yt-dlp" } else { "ffmpeg" };
    if let Ok(path) = which::which(binary) {
        if path.is_file() {
            return Some(path);
        }
    }
    #[cfg(windows)]
    if let Ok(path) = which::which(format!("{binary}.exe")) {
        if path.is_file() {
            return Some(path);
        }
    }
    None
}

fn exe_file(tool: &str) -> String {
    let base = if tool == "ytdlp" { "yt-dlp" } else { "ffmpeg" };
    if cfg!(windows) {
        format!("{base}.exe")
    } else {
        base.to_string()
    }
}

fn read_version(tool: &str, path: &Path) -> String {
    read_version_with_timeout(tool, path, Duration::from_secs(10))
}

/// Run `<tool> --version` (or `-version`) with a hard timeout so a broken
/// binary can never block tool detection or installation.
fn read_version_with_timeout(tool: &str, path: &Path, timeout: Duration) -> String {
    let mut command = std::process::Command::new(path);
    if tool == "ffmpeg" {
        command.arg("-version");
    } else {
        command.arg("--version");
    }
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }

    let Ok(mut child) = command.spawn() else {
        return String::new();
    };
    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    return String::new();
                }
                use std::io::Read;
                let mut stdout = String::new();
                if let Some(mut pipe) = child.stdout.take() {
                    let _ = pipe.read_to_string(&mut stdout);
                }
                let mut stderr = String::new();
                if let Some(mut pipe) = child.stderr.take() {
                    let _ = pipe.read_to_string(&mut stderr);
                }
                let text = if stdout.trim().is_empty() {
                    stderr
                } else {
                    stdout
                };
                return text
                    .lines()
                    .map(str::trim)
                    .find(|line| !line.is_empty())
                    .map(|line| line.chars().take(160).collect())
                    .unwrap_or_default();
            }
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return String::new();
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(_) => return String::new(),
        }
    }
}

/// Validate the downloaded binary, then swap it in while keeping the previous
/// version as a rollback target. A failed probe restores the old file.
fn install_binary(src: &Path, dest: &Path, tool: &str) -> Result<(), String> {
    if read_version_with_timeout(tool, src, Duration::from_secs(10)).is_empty() {
        let _ = std::fs::remove_file(src);
        return Err(format!(
            "[TOOL] {} 下载内容无法运行，已放弃安装（旧版本保持不变）",
            tool_display(tool)
        ));
    }

    let backup = swap_in(src, dest).map_err(|e| format!("[PATH] 保存 {} 失败: {e}", tool_display(tool)))?;

    if read_version_with_timeout(tool, dest, Duration::from_secs(10)).is_empty() {
        let _ = std::fs::remove_file(dest);
        if let Some(backup) = &backup {
            let _ = std::fs::rename(backup, dest);
        }
        return Err(format!(
            "[TOOL] {} 安装后无法运行，已回滚到旧版本",
            tool_display(tool)
        ));
    }

    if let Some(backup) = backup {
        let _ = std::fs::remove_file(backup);
    }
    Ok(())
}

/// Move `src` over `dest`, renaming the previous file to a backup first. The
/// caller decides whether to delete or restore the returned backup.
fn swap_in(src: &Path, dest: &Path) -> Result<Option<PathBuf>, String> {
    let backup = if dest.exists() {
        let name = dest
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("tool");
        let backup = dest.with_file_name(format!(".{name}.bak-{}", uuid::Uuid::new_v4()));
        std::fs::rename(dest, &backup).map_err(|e| format!("[PATH] 备份旧版本失败: {e}"))?;
        Some(backup)
    } else {
        None
    };

    if let Err(err) = std::fs::rename(src, dest) {
        if let Some(backup) = &backup {
            let _ = std::fs::rename(backup, dest);
        }
        return Err(format!("[PATH] 替换文件失败: {err}"));
    }
    Ok(backup)
}

fn tool_display(tool: &str) -> &'static str {
    if tool == "ffmpeg" {
        "ffmpeg"
    } else {
        "yt-dlp"
    }
}

/// Compute the SHA-256 hex digest of a file, streaming it in chunks so large
/// archives never load fully into memory.
fn sha256_hex(path: &Path) -> Result<String, String> {
    use sha2::{Digest, Sha256};
    use std::io::Read;

    let mut file =
        std::fs::File::open(path).map_err(|e| format!("[PATH] 打开文件失败: {e}"))?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let n = file
            .read(&mut buf)
            .map_err(|e| format!("[PATH] 读取文件失败: {e}"))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64 && value.chars().all(|c| c.is_ascii_hexdigit())
}

/// Extract the expected hash for `filename` from an official `SHA2-256SUMS`
/// document (`<hash>  <name>` lines, optionally with the `*` binary marker).
/// Malformed lines are skipped rather than aborting the scan.
fn parse_sha256_sums(text: &str, filename: &str) -> Option<String> {
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let mut parts = line.splitn(2, char::is_whitespace);
        let (Some(hash), Some(name)) = (parts.next(), parts.next()) else {
            continue;
        };
        let hash = hash.trim();
        let name = name.trim().trim_start_matches('*');
        if is_sha256_hex(hash) && name.eq_ignore_ascii_case(filename) {
            return Some(hash.to_ascii_lowercase());
        }
    }
    None
}

/// Extract a bare SHA-256 hex digest from a `.sha256` document. Accepts both a
/// lone hash line and a `hash  name` line, taking the first 64-hex token.
fn parse_bare_sha256(text: &str) -> Option<String> {
    for line in text.lines() {
        let token = line.split_whitespace().next().unwrap_or("");
        if is_sha256_hex(token) {
            return Some(token.to_ascii_lowercase());
        }
    }
    None
}

/// Compare a downloaded file's SHA-256 against the official hash. A mismatch is
/// a hard failure: the caller must discard the download and keep the old tool.
fn verify_sha256(path: &Path, expected: &str, what: &str) -> Result<(), String> {
    let actual = sha256_hex(path)?;
    if !actual.eq_ignore_ascii_case(expected.trim()) {
        return Err(format!(
            "[TOOL] {what} 哈希校验失败（官方 {}…，实际 {}…），已放弃安装，旧版本保持不变",
            &expected[..12.min(expected.len())],
            &actual[..12.min(actual.len())],
        ));
    }
    Ok(())
}

/// Download an official checksum document. Fail-closed: an unreachable or
/// malformed checksum source aborts installation instead of skipping checks.
async fn fetch_checksum(client: &Client, url: &str) -> Result<String, String> {
    let response = client
        .get(url)
        .header(reqwest::header::USER_AGENT, UA)
        .send()
        .await
        .map_err(|e| format!("[NETWORK] 获取官方校验文件失败: {e}"))?;
    if !response.status().is_success() {
        return Err(format!(
            "[NETWORK] 获取官方校验文件失败: HTTP {}",
            response.status()
        ));
    }
    response
        .text()
        .await
        .map_err(|e| format!("[NETWORK] 读取官方校验文件失败: {e}"))
}

/// Verify a downloaded yt-dlp binary against the official SHA2-256SUMS asset
/// published in the same release directory as the binary.
pub async fn verify_ytdlp_checksum(client: &Client, part: &Path, url: &str) -> Result<(), String> {
    let asset = url
        .rsplit('/')
        .next()
        .filter(|name| !name.is_empty())
        .ok_or_else(|| "[TOOL] 无法确定 yt-dlp 安装文件名".to_string())?;
    let download_dir = url
        .rfind('/')
        .map(|idx| &url[..idx])
        .ok_or_else(|| "[TOOL] 无法确定 yt-dlp 校验文件地址".to_string())?;
    let sums = fetch_checksum(client, &format!("{download_dir}/SHA2-256SUMS")).await?;
    let expected = parse_sha256_sums(&sums, asset)
        .ok_or_else(|| format!("[TOOL] 官方校验文件中未找到 {asset} 的哈希，已放弃安装"))?;
    verify_sha256(part, &expected, "yt-dlp")
}

/// Verify a downloaded ffmpeg zip against the `.sha256` document published
/// alongside the archive.
pub async fn verify_ffmpeg_checksum(client: &Client, zip: &Path, url: &str) -> Result<(), String> {
    let text = fetch_checksum(client, &format!("{url}.sha256")).await?;
    let expected = parse_bare_sha256(&text)
        .ok_or_else(|| "[TOOL] 官方校验文件格式异常，已放弃安装".to_string())?;
    verify_sha256(zip, &expected, "ffmpeg 压缩包")
}

pub async fn install_tool(
    app: &AppHandle,
    tool: &str,
    config: &AppConfig,
    paths: &AppPaths,
) -> Result<ToolInstallResult, String> {
    let key = match tool {
        "ytdlp" => "ytdlp",
        "ffmpeg" => "ffmpeg",
        _ => return Err(format!("[PARAM] 未知工具: {tool}")),
    };

    std::fs::create_dir_all(&paths.bin_dir)
        .map_err(|e| format!("[PATH] 创建工具目录失败: {e}"))?;
    let client = build_client(config)?;

    if key == "ytdlp" {
        install_ytdlp(&client, app, config, paths).await?;
    } else {
        install_ffmpeg(&client, app, config, paths).await?;
    }

    let tools = get_status(config, paths, true);
    let status = if key == "ytdlp" {
        tools.ytdlp.clone()
    } else {
        tools.ffmpeg.clone()
    };
    if !status.ok {
        return Err(format!(
            "[TOOL] {} 安装后无法运行，请检查杀毒软件拦截或网络后重试",
            tool_display(key)
        ));
    }
    emit_progress(app, key, "done", Some(100.0), 0, 0);

    Ok(ToolInstallResult {
        tool: key.to_string(),
        path: status.path,
        version: status.version,
        tools,
    })
}

async fn install_ytdlp(
    client: &Client,
    app: &AppHandle,
    config: &AppConfig,
    paths: &AppPaths,
) -> Result<(), String> {
    let url = ytdlp_url();
    let url = with_mirror(url, &config.tool_download_mirror);
    let dest = paths.bin_dir.join(exe_file("ytdlp"));
    let part = paths.bin_dir.join(format!("{}.part", exe_file("ytdlp")));
    let _ = std::fs::remove_file(&part);

    if let Err(err) = download_file(client, &url, &part, app, "ytdlp").await {
        let _ = std::fs::remove_file(&part);
        return Err(err);
    }

    if let Err(err) = verify_ytdlp_checksum(client, &part, &url).await {
        let _ = std::fs::remove_file(&part);
        return Err(err);
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&part, std::fs::Permissions::from_mode(0o755))
            .map_err(|e| format!("设置可执行权限失败: {e}"))?;
    }

    install_binary(&part, &dest, "ytdlp")
}
async fn install_ffmpeg(
    client: &Client,
    app: &AppHandle,
    config: &AppConfig,
    paths: &AppPaths,
) -> Result<(), String> {
    if !cfg!(windows) {
        return Err("[TOOL] 当前平台暂不支持自动安装 ffmpeg，请自行安装".into());
    }

    let url = with_mirror(FFMPEG_ZIP, &config.tool_download_mirror);
    let temp_dir = paths
        .bin_dir
        .join(format!(".tmp-ffmpeg-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&temp_dir).map_err(|e| format!("创建临时目录失败: {e}"))?;
    let zip = temp_dir.join("ffmpeg.zip");

    let outcome = async {
        download_file(client, &url, &zip, app, "ffmpeg").await?;
        verify_ffmpeg_checksum(client, &zip, &url).await?;
        emit_progress(app, "ffmpeg", "extract", Some(0.0), 0, 0);
        expand_archive(&zip, &temp_dir).await?;

        let ffmpeg_src = find_file(&temp_dir, "ffmpeg.exe", 6)
            .ok_or_else(|| "压缩包中未找到 ffmpeg.exe".to_string())?;
        if read_version_with_timeout("ffmpeg", &ffmpeg_src, Duration::from_secs(10)).is_empty() {
            return Err("[TOOL] ffmpeg.exe 无法运行，已放弃安装（旧版本保持不变）".into());
        }

        if let Some(ffprobe_src) = find_file(&temp_dir, "ffprobe.exe", 6) {
            let probe_dest = paths.bin_dir.join("ffprobe.exe");
            match swap_in(&ffprobe_src, &probe_dest) {
                Ok(backup) => {
                    if let Some(backup) = backup {
                        let _ = std::fs::remove_file(backup);
                    }
                }
                Err(err) => return Err(format!("[PATH] 保存 ffprobe.exe 失败: {err}")),
            }
        }

        let dest = paths.bin_dir.join("ffmpeg.exe");
        install_binary(&ffmpeg_src, &dest, "ffmpeg")?;

        emit_progress(app, "ffmpeg", "extract", Some(100.0), 0, 0);
        Ok(())
    }
    .await;

    let _ = std::fs::remove_dir_all(&temp_dir);
    outcome
}

fn ytdlp_url() -> &'static str {
    if cfg!(windows) {
        "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe"
    } else if cfg!(target_os = "macos") {
        "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_macos"
    } else {
        "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_linux"
    }
}

fn with_mirror(url: &str, mirror: &str) -> String {
    let prefix = mirror.trim();
    if prefix.is_empty() {
        return url.to_string();
    }
    if prefix.ends_with('/') {
        format!("{prefix}{url}")
    } else {
        format!("{prefix}/{url}")
    }
}

fn build_client(config: &AppConfig) -> Result<Client, String> {
    let mut builder = Client::builder()
        .user_agent(UA)
        .connect_timeout(Duration::from_secs(30))
        .timeout(Duration::from_secs(15 * 60));

    let proxy = config.proxy.trim();
    if proxy.is_empty() {
        builder = builder.no_proxy();
    } else {
        builder = builder.proxy(
            reqwest::Proxy::all(proxy).map_err(|e| format!("[PARAM] 代理地址无效: {e}"))?,
        );
    }

    builder
        .build()
        .map_err(|e| format!("[NETWORK] 初始化网络客户端失败: {e}"))
}

async fn download_file(
    client: &Client,
    url: &str,
    dest: &Path,
    app: &AppHandle,
    tool: &str,
) -> Result<u64, String> {
    let response = client
        .get(url)
        .header(reqwest::header::USER_AGENT, UA)
        .send()
        .await
        .map_err(|e| format!("[NETWORK] 下载失败: {e}"))?;
    if !response.status().is_success() {
        return Err(format!("[NETWORK] 下载失败: HTTP {}", response.status()));
    }

    let total = response.content_length().unwrap_or(0);
    let mut file = tokio::fs::File::create(dest)
        .await
        .map_err(|e| format!("[PATH] 创建临时文件失败: {e}"))?;
    let mut stream = response.bytes_stream();
    let mut received: u64 = 0;
    let mut last_at = Instant::now();
    let mut last_bytes: u64 = 0;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("[NETWORK] 下载中断: {e}"))?;
        file.write_all(&chunk)
            .await
            .map_err(|e| format!("[PATH] 写入临时文件失败: {e}"))?;
        received += chunk.len() as u64;

        if last_at.elapsed() >= EMIT_INTERVAL
            || received.saturating_sub(last_bytes) >= EMIT_BYTES
        {
            last_at = Instant::now();
            last_bytes = received;
            emit_progress(app, tool, "download", percent(received, total), received, total);
        }
    }

    file.flush()
        .await
        .map_err(|e| format!("写入临时文件失败: {e}"))?;
    let _ = file.sync_all().await;
    emit_progress(app, tool, "download", percent(received, total), received, total);
    Ok(received)
}

fn percent(received: u64, total: u64) -> Option<f64> {
    if total == 0 {
        None
    } else {
        Some((received as f64 / total as f64 * 100.0).min(100.0))
    }
}

fn emit_progress(
    app: &AppHandle,
    tool: &str,
    phase: &str,
    pct: Option<f64>,
    received: u64,
    total: u64,
) {
    let _ = app.emit(
        "tool-progress",
        ToolProgress {
            tool: tool.to_string(),
            phase: phase.to_string(),
            pct,
            received,
            total,
        },
    );
}

async fn expand_archive(zip: &Path, dest: &Path) -> Result<(), String> {
    let script = format!(
        "Expand-Archive -LiteralPath '{}' -DestinationPath '{}' -Force",
        zip.to_string_lossy().replace('\'', "''"),
        dest.to_string_lossy().replace('\'', "''")
    );

    let mut command = tokio::process::Command::new("powershell.exe");
    command
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        command.creation_flags(0x0800_0000);
    }

    let output = command
        .output()
        .await
        .map_err(|e| format!("解压失败: {e}"))?;
    if !output.status.success() {
        let detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if detail.is_empty() {
            format!("解压失败: 退出码 {:?}", output.status.code())
        } else {
            format!("解压失败: {detail}")
        });
    }
    Ok(())
}

fn find_file(dir: &Path, name: &str, depth: usize) -> Option<PathBuf> {
    if depth == 0 {
        return None;
    }
    let entries = std::fs::read_dir(dir).ok()?;
    let mut dirs = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            if path
                .file_name()
                .and_then(|value| value.to_str())
                .is_some_and(|file| file.eq_ignore_ascii_case(name))
            {
                return Some(path);
            }
        } else if path.is_dir() {
            dirs.push(path);
        }
    }
    dirs.into_iter()
        .find_map(|sub| find_file(&sub, name, depth - 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("toolbox-tools-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn missing_binary_probe_returns_empty() {
        let dir = temp_dir();
        let missing = dir.join("nope.exe");
        assert!(read_version_with_timeout("ytdlp", &missing, Duration::from_secs(1)).is_empty());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn swap_in_keeps_previous_file_as_backup() {
        let dir = temp_dir();
        let dest = dir.join("yt-dlp.exe");
        let src = dir.join("yt-dlp.part");
        std::fs::write(&dest, b"old").unwrap();
        std::fs::write(&src, b"new").unwrap();

        let backup = swap_in(&src, &dest).unwrap().expect("backup");
        assert_eq!(std::fs::read(&dest).unwrap(), b"new");
        assert_eq!(std::fs::read(&backup).unwrap(), b"old");
        assert!(!src.exists());

        std::fs::remove_file(backup).unwrap();
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn invalid_binary_is_rejected_and_old_binary_survives() {
        let dir = temp_dir();
        let dest = dir.join("yt-dlp.exe");
        let src = dir.join("yt-dlp.part");
        std::fs::write(&dest, b"old").unwrap();
        std::fs::write(&src, b"not a program").unwrap();

        assert!(install_binary(&src, &dest, "ytdlp").is_err());
        assert_eq!(std::fs::read(&dest).unwrap(), b"old");
        assert!(!src.exists());
        let _ = std::fs::remove_dir_all(dir);
    }

    const SHA256_ABC: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

    #[test]
    fn sha256_hex_matches_known_vector() {
        let dir = temp_dir();
        let file = dir.join("sample");
        std::fs::write(&file, b"abc").unwrap();
        assert_eq!(sha256_hex(&file).unwrap(), SHA256_ABC);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn parse_sha256_sums_finds_expected_asset() {
        let sums = format!(
            "{}  other.exe\n{}  *yt-dlp.exe\nshort  yt-dlp_linux\n",
            "1".repeat(64),
            "2".repeat(64)
        );
        assert_eq!(parse_sha256_sums(&sums, "yt-dlp.exe").unwrap(), "2".repeat(64));
        assert!(parse_sha256_sums(&sums, "yt-dlp_macos").is_none());
        // Malformed lines must not abort the scan.
        let mixed = format!("garbage-line\n{}  yt-dlp.exe\n", "3".repeat(64));
        assert_eq!(
            parse_sha256_sums(&mixed, "yt-dlp.exe").unwrap(),
            "3".repeat(64)
        );
    }

    #[test]
    fn parse_bare_sha256_accepts_bare_and_labeled_lines() {
        assert_eq!(parse_bare_sha256(&format!("{SHA256_ABC}\n")).unwrap(), SHA256_ABC);
        assert_eq!(
            parse_bare_sha256(&format!("{SHA256_ABC}  ffmpeg-9.0.2.zip\n")).unwrap(),
            SHA256_ABC
        );
        assert!(parse_bare_sha256("not-a-hash\n").is_none());
    }

    #[test]
    fn verify_sha256_accepts_match_and_rejects_mismatch() {
        let dir = temp_dir();
        let file = dir.join("sample");
        std::fs::write(&file, b"abc").unwrap();

        verify_sha256(&file, SHA256_ABC, "样例").unwrap();
        verify_sha256(&file, &SHA256_ABC.to_ascii_uppercase(), "样例").unwrap();

        let err = verify_sha256(&file, &"0".repeat(64), "样例").unwrap_err();
        assert!(err.starts_with("[TOOL]"));
        assert!(err.contains("哈希校验失败"));
        let _ = std::fs::remove_dir_all(dir);
    }
}
