//! 哈希校验：流式 SHA-256 计算与官方校验文件解析。
//!
//! fail-closed：校验文件不可达、格式异常或哈希不匹配时，
//! 必须终止安装并保留旧版本。

use std::path::Path;

use reqwest::Client;
use sha2::{Digest, Sha256};

use super::UA;

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
pub(crate) fn parse_sha256_sums(text: &str, filename: &str) -> Option<String> {
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
pub(crate) fn parse_bare_sha256(text: &str) -> Option<String> {
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
pub(crate) fn verify_sha256(path: &Path, expected: &str, what: &str) -> Result<(), String> {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("toolbox-checksum-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
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
