//! 集成测试：工具安装的 SHA-256 校验契约（解析官方校验文件 → 比对本地文件）。
//!
//! URL 选择（二进制可镜像、校验文件必须官方 origin）与官方源不可达 / 超限
//! 等获取侧场景，由 `core/tools/checksum.rs` 与 `core/tools/net.rs` 的内部
//! 单测覆盖（那里可以访问私有获取路径）；本文件只走 crate 的公开接口。

use std::path::PathBuf;

use toolbox_lib::core::tools::{parse_bare_sha256, parse_sha256_sums, verify_sha256};

const SHA256_ABC: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

fn temp_file(name: &str, contents: &[u8]) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("toolbox-checksum-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(name);
    std::fs::write(&path, contents).unwrap();
    path
}

fn asset() -> &'static str {
    match std::env::consts::OS {
        "windows" => "yt-dlp.exe",
        "macos" => "yt-dlp_macos",
        _ => "yt-dlp_linux",
    }
}

#[test]
fn official_sums_document_selects_platform_asset() {
    let sums = format!(
        "{}  yt-dlp_linux\n{}  {}\n",
        "1".repeat(64),
        SHA256_ABC,
        asset()
    );
    let expected = parse_sha256_sums(&sums, asset()).expect("应命中当前平台资产");
    assert_eq!(expected, SHA256_ABC);
    assert!(parse_sha256_sums(&sums, "yt-dlp.exe.sig").is_none());
}

#[test]
fn matching_hash_succeeds_and_tampered_binary_fails() {
    let good = temp_file("good.part", b"abc");
    verify_sha256(&good, SHA256_ABC, "yt-dlp").unwrap();

    // 镜像返回篡改后的二进制 + 官方哈希：必须硬失败。
    let tampered = temp_file("tampered.part", b"abcd");
    let err = verify_sha256(&tampered, SHA256_ABC, "yt-dlp").unwrap_err();
    assert!(err.starts_with("[TOOL]"), "got: {err}");
    assert!(err.contains("哈希校验失败"), "got: {err}");
    assert!(err.contains("旧版本保持不变"), "got: {err}");
}

#[test]
fn malformed_checksum_document_is_rejected() {
    assert!(parse_bare_sha256("this-is-not-a-hash").is_none());
    assert!(parse_sha256_sums("garbage", asset()).is_none());
    assert_eq!(
        parse_bare_sha256(&format!("{SHA256_ABC}  ffmpeg.zip\n")).as_deref(),
        Some(SHA256_ABC)
    );
}
