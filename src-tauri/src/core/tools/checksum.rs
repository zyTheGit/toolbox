//! 哈希校验：流式 SHA-256 计算与官方校验文件解析。
//!
//! 信任锚固定为官方 origin：二进制可以走用户配置的镜像，但 SHA2-256SUMS /
//! `.sha256` 始终取自官方地址，镜像无法同时伪造二进制与哈希。
//! fail-closed：校验文件不可达、格式异常或哈希不匹配时，
//! 必须终止安装并保留旧版本。

use std::path::Path;

use futures_util::StreamExt;
use reqwest::Client;
use sha2::{Digest, Sha256};

use super::net::MAX_CHECKSUM_BYTES;
use super::UA;

/// yt-dlp 官方发布目录（只用于拼接官方地址，不经过镜像）。
const YTDLP_RELEASE_BASE: &str = "https://github.com/yt-dlp/yt-dlp/releases/latest/download";
/// ffmpeg Windows 构建官方地址。
const FFMPEG_ZIP_URL: &str = "https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip";

/// 当前平台对应的 yt-dlp 资产名。
pub(crate) fn ytdlp_asset() -> &'static str {
    if cfg!(windows) {
        "yt-dlp.exe"
    } else if cfg!(target_os = "macos") {
        "yt-dlp_macos"
    } else {
        "yt-dlp_linux"
    }
}

/// yt-dlp 官方下载地址（镜像只应作用于返回值，见 `net::binary_url`）。
pub(crate) fn ytdlp_url() -> String {
    format!("{YTDLP_RELEASE_BASE}/{}", ytdlp_asset())
}

/// yt-dlp 官方校验文件地址：固定官方 origin，不接受镜像。
pub(crate) fn ytdlp_checksum_url() -> String {
    format!("{YTDLP_RELEASE_BASE}/SHA2-256SUMS")
}

/// ffmpeg 官方压缩包地址（仅 Windows 安装流程使用）。
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn ffmpeg_url() -> &'static str {
    FFMPEG_ZIP_URL
}

/// ffmpeg 官方校验文件地址：固定官方 origin，不接受镜像。
pub(crate) fn ffmpeg_checksum_url() -> String {
    format!("{FFMPEG_ZIP_URL}.sha256")
}

/// Compute the SHA-256 hex digest of a file, streaming it in chunks so large
/// archives never load fully into memory.
pub(crate) fn sha256_hex(path: &Path) -> Result<String, String> {
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
pub fn parse_sha256_sums(text: &str, filename: &str) -> Option<String> {
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
pub fn parse_bare_sha256(text: &str) -> Option<String> {
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
pub fn verify_sha256(path: &Path, expected: &str, what: &str) -> Result<(), String> {
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

/// Download an official checksum document with a hard size limit. Fail-closed:
/// an unreachable or malformed checksum source aborts installation instead of
/// skipping checks, and oversized bodies are rejected.
pub(crate) async fn fetch_checksum(client: &Client, url: &str) -> Result<String, String> {
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
    if super::net::exceeds_limit(response.content_length().unwrap_or(0), MAX_CHECKSUM_BYTES) {
        return Err(checksum_limit_error());
    }

    let mut body: Vec<u8> = Vec::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("[NETWORK] 读取官方校验文件失败: {e}"))?;
        if super::net::exceeds_limit(body.len() as u64 + chunk.len() as u64, MAX_CHECKSUM_BYTES) {
            return Err(checksum_limit_error());
        }
        body.extend_from_slice(&chunk);
    }
    String::from_utf8(body).map_err(|_| "[NETWORK] 官方校验文件编码无效".to_string())
}

fn checksum_limit_error() -> String {
    format!(
        "[NETWORK] 官方校验文件超过 {} MB 上限，已放弃安装",
        MAX_CHECKSUM_BYTES / (1024 * 1024)
    )
}

/// Verify a downloaded yt-dlp binary against the official SHA2-256SUMS asset.
/// The checksum URL is always the official release directory, never a mirror.
pub async fn verify_ytdlp_checksum(client: &Client, part: &Path) -> Result<(), String> {
    let asset = ytdlp_asset();
    let sums = fetch_checksum(client, &ytdlp_checksum_url()).await?;
    let expected = parse_sha256_sums(&sums, asset)
        .ok_or_else(|| format!("[TOOL] 官方校验文件中未找到 {asset} 的哈希，已放弃安装"))?;
    verify_sha256(part, &expected, "yt-dlp")
}

/// Verify a downloaded ffmpeg zip against the official `.sha256` document.
pub async fn verify_ffmpeg_checksum(client: &Client, zip: &Path) -> Result<(), String> {
    let text = fetch_checksum(client, &ffmpeg_checksum_url()).await?;
    let expected = parse_bare_sha256(&text)
        .ok_or_else(|| "[TOOL] 官方校验文件格式异常，已放弃安装".to_string())?;
    verify_sha256(zip, &expected, "ffmpeg 压缩包")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("toolbox-checksum-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn runtime() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
    }

    fn test_client() -> Client {
        Client::builder().no_proxy().build().unwrap()
    }

    /// 极简本地 HTTP 服务器：固定状态码 + 正文，用于离线模拟官方校验源。
    fn spawn_server(status: u16, body: String) -> u16 {
        use std::io::{Read as _, Write as _};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                let mut buf = [0u8; 1024];
                let _ = stream.read(&mut buf);
                let response = format!(
                    "HTTP/1.1 {status} X\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = stream.write_all(response.as_bytes());
                let _ = stream.flush();
            }
        });
        port
    }

    /// 校验链路：获取校验文件 → 解析 → 与本地文件比对。
    fn verify_document(port: u16, part: &Path) -> Result<(), String> {
        let url = format!("http://127.0.0.1:{port}/SHA2-256SUMS");
        let body = runtime()
            .block_on(fetch_checksum(&test_client(), &url))
            .map_err(|err| err.to_string())?;
        let expected = parse_sha256_sums(&body, ytdlp_asset())
            .ok_or_else(|| "[TOOL] missing asset".to_string())?;
        verify_sha256(part, &expected, "yt-dlp")
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
    fn checksum_urls_are_official_origin() {
        // 校验地址只由官方常量决定，签名里没有镜像参数。
        assert_eq!(
            ytdlp_checksum_url(),
            format!("{YTDLP_RELEASE_BASE}/SHA2-256SUMS")
        );
        assert!(ytdlp_checksum_url().starts_with("https://github.com/yt-dlp/"));
        assert_eq!(ffmpeg_checksum_url(), format!("{FFMPEG_ZIP_URL}.sha256"));
        assert!(ffmpeg_checksum_url().starts_with("https://www.gyan.dev/"));
        assert_eq!(ytdlp_url(), format!("{YTDLP_RELEASE_BASE}/{}", ytdlp_asset()));
    }

    #[test]
    fn binary_may_use_mirror_but_checksum_does_not() {
        let official = ytdlp_url();
        let mirrored = super::super::net::binary_url(&official, "https://mirror.example/")
            .unwrap();
        assert!(mirrored.starts_with("https://mirror.example/"));
        assert!(!mirrored.contains("SHA2-256SUMS"));
        // 无论镜像如何配置，校验文件地址不受影响。
        assert_eq!(
            ytdlp_checksum_url(),
            format!("{YTDLP_RELEASE_BASE}/SHA2-256SUMS")
        );
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

    /// 官方校验源不可达时必须返回错误（fail-closed），不能跳过校验继续安装。
    #[test]
    fn unreachable_checksum_source_fails_closed() {
        let result = runtime().block_on(fetch_checksum(&test_client(), "http://127.0.0.1:1/SHA2-256SUMS"));
        let err = result.unwrap_err();
        assert!(err.starts_with("[NETWORK]"), "错误必须指向网络/官方源: {err}");
        assert!(err.contains("官方校验文件"));
    }

    #[test]
    fn checksum_chain_accepts_matching_document() {
        let body = format!("{}  {}\n", SHA256_ABC, ytdlp_asset());
        let port = spawn_server(200, body);
        let dir = temp_dir();
        let part = dir.join("part");
        std::fs::write(&part, b"abc").unwrap();

        verify_document(port, &part).unwrap();
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn checksum_chain_rejects_tampered_binary() {
        // 镜像提供篡改后的二进制时，官方校验文件里的哈希不匹配 → 硬失败。
        let body = format!("{}  {}\n", "0".repeat(64), ytdlp_asset());
        let port = spawn_server(200, body);
        let dir = temp_dir();
        let part = dir.join("part");
        std::fs::write(&part, b"abc").unwrap();

        let err = verify_document(port, &part).unwrap_err();
        assert!(err.starts_with("[TOOL]"), "got: {err}");
        assert!(err.contains("哈希校验失败"), "got: {err}");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn checksum_source_http_error_fails_closed() {
        let port = spawn_server(404, "not found".to_string());
        let dir = temp_dir();
        let part = dir.join("part");
        std::fs::write(&part, b"abc").unwrap();

        let err = verify_document(port, &part).unwrap_err();
        assert!(err.starts_with("[NETWORK]"), "got: {err}");
        assert!(err.contains("获取官方校验文件失败"), "got: {err}");
        assert_eq!(std::fs::read(&part).unwrap(), b"abc", "校验失败不得改动原文件");
        let _ = std::fs::remove_dir_all(dir);
    }
}
