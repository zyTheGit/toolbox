//! 集成测试：工具安装的 SHA-256 校验链路（获取校验文件 → 解析 → 比对）。
//! 使用本地 HTTP 服务器模拟官方校验源，不访问真实网络。

use std::collections::HashMap;
use std::io::{Read as _, Write as _};
use std::net::TcpListener;
use std::path::PathBuf;

use toolbox_lib::core::tools::{verify_ffmpeg_checksum, verify_ytdlp_checksum};

const SHA256_ABC: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

/// Start a local HTTP server that answers each request from `routes`
/// (path → status/body) and closes the connection afterwards. Returns
/// the bound port; unknown paths get a 404.
fn spawn_server(routes: HashMap<String, (u16, String)>) -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let mut buf = [0u8; 2048];
            let _ = stream.read(&mut buf);
            let request = String::from_utf8_lossy(&buf);
            let path = request
                .split_whitespace()
                .nth(1)
                .unwrap_or_default()
                .to_string();

            let (status, body) = routes
                .get(&path)
                .cloned()
                .unwrap_or((404, "not found".to_string()));
            let reason = if status == 200 { "OK" } else { "Not Found" };
            let response = format!(
                "HTTP/1.1 {status} {reason}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.flush();
        }
    });
    port
}

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn temp_file(name: &str, contents: &[u8]) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("toolbox-checksum-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(name);
    std::fs::write(&path, contents).unwrap();
    path
}

fn sums_body(filename: &str, hash: &str) -> String {
    format!("{hash}  {filename}\n")
}

#[tokio::test]
async fn ytdlp_checksum_passes_when_sums_match() {
    let port = spawn_server(HashMap::from([(
        "/releases/download/SHA2-256SUMS".to_string(),
        (200, sums_body("yt-dlp.exe", SHA256_ABC)),
    )]));
    let part = temp_file("yt-dlp.exe.part", b"abc");
    let url = format!("http://127.0.0.1:{port}/releases/download/yt-dlp.exe");

    verify_ytdlp_checksum(&client(), &part, &url)
        .await
        .unwrap();
}

#[tokio::test]
async fn ytdlp_checksum_rejects_mismatched_hash() {
    let wrong = "0".repeat(64);
    let port = spawn_server(HashMap::from([(
        "/releases/download/SHA2-256SUMS".to_string(),
        (200, sums_body("yt-dlp.exe", &wrong)),
    )]));
    let part = temp_file("yt-dlp.exe.part", b"abc");
    let url = format!("http://127.0.0.1:{port}/releases/download/yt-dlp.exe");

    let err = verify_ytdlp_checksum(&client(), &part, &url)
        .await
        .unwrap_err();
    assert!(err.starts_with("[TOOL]"), "got: {err}");
    assert!(err.contains("哈希校验失败"), "got: {err}");
    // 校验失败由调用方清理下载文件，这里只断言原文件未被改动。
    assert_eq!(std::fs::read(&part).unwrap(), b"abc");
}

#[tokio::test]
async fn ytdlp_checksum_fails_when_asset_missing_from_sums() {
    let port = spawn_server(HashMap::from([(
        "/releases/download/SHA2-256SUMS".to_string(),
        (200, sums_body("yt-dlp_linux", SHA256_ABC)),
    )]));
    let part = temp_file("yt-dlp.exe.part", b"abc");
    let url = format!("http://127.0.0.1:{port}/releases/download/yt-dlp.exe");

    let err = verify_ytdlp_checksum(&client(), &part, &url)
        .await
        .unwrap_err();
    assert!(err.contains("未找到"), "got: {err}");
}

#[tokio::test]
async fn ytdlp_checksum_fails_when_sums_unreachable() {
    // 空路由表 → 一切请求 404，模拟镜像缺少校验文件。
    let port = spawn_server(HashMap::new());
    let part = temp_file("yt-dlp.exe.part", b"abc");
    let url = format!("http://127.0.0.1:{port}/releases/download/yt-dlp.exe");

    let err = verify_ytdlp_checksum(&client(), &part, &url)
        .await
        .unwrap_err();
    assert!(err.starts_with("[NETWORK]"), "got: {err}");
    assert!(err.contains("获取官方校验文件失败"), "got: {err}");
}

#[tokio::test]
async fn ytdlp_checksum_derives_asset_from_url_path() {
    // 非 Windows 资产名（yt-dlp_linux）也应能从 URL 推导并匹配。
    let port = spawn_server(HashMap::from([(
        "/latest/download/SHA2-256SUMS".to_string(),
        (200, sums_body("yt-dlp_linux", SHA256_ABC)),
    )]));
    let part = temp_file("yt-dlp_linux.part", b"abc");
    let url = format!("http://127.0.0.1:{port}/latest/download/yt-dlp_linux");

    verify_ytdlp_checksum(&client(), &part, &url)
        .await
        .unwrap();
}

#[tokio::test]
async fn ffmpeg_checksum_passes_with_bare_sha256() {
    let port = spawn_server(HashMap::from([(
        "/builds/ffmpeg-release-essentials.zip.sha256".to_string(),
        (200, format!("{SHA256_ABC}\n")),
    )]));
    let zip = temp_file("ffmpeg.zip", b"abc");
    let url = format!("http://127.0.0.1:{port}/builds/ffmpeg-release-essentials.zip");

    verify_ffmpeg_checksum(&client(), &zip, &url)
        .await
        .unwrap();
}

#[tokio::test]
async fn ffmpeg_checksum_accepts_labeled_sha256_line() {
    let port = spawn_server(HashMap::from([(
        "/builds/packages/ffmpeg-9.0.2-essentials_build.zip.sha256".to_string(),
        (200, format!("{SHA256_ABC}  ffmpeg-9.0.2-essentials_build.zip\n")),
    )]));
    let zip = temp_file("ffmpeg.zip", b"abc");
    let url = format!(
        "http://127.0.0.1:{port}/builds/packages/ffmpeg-9.0.2-essentials_build.zip"
    );

    verify_ffmpeg_checksum(&client(), &zip, &url)
        .await
        .unwrap();
}

#[tokio::test]
async fn ffmpeg_checksum_rejects_mismatched_archive() {
    let wrong = "f".repeat(64);
    let port = spawn_server(HashMap::from([(
        "/builds/ffmpeg-release-essentials.zip.sha256".to_string(),
        (200, format!("{wrong}\n")),
    )]));
    let zip = temp_file("ffmpeg.zip", b"abc");
    let url = format!("http://127.0.0.1:{port}/builds/ffmpeg-release-essentials.zip");

    let err = verify_ffmpeg_checksum(&client(), &zip, &url)
        .await
        .unwrap_err();
    assert!(err.starts_with("[TOOL]"), "got: {err}");
    assert!(err.contains("哈希校验失败"), "got: {err}");
}

#[tokio::test]
async fn ffmpeg_checksum_fails_on_malformed_sha256_document() {
    let port = spawn_server(HashMap::from([(
        "/builds/ffmpeg-release-essentials.zip.sha256".to_string(),
        (200, "this-is-not-a-hash".to_string()),
    )]));
    let zip = temp_file("ffmpeg.zip", b"abc");
    let url = format!("http://127.0.0.1:{port}/builds/ffmpeg-release-essentials.zip");

    let err = verify_ffmpeg_checksum(&client(), &zip, &url)
        .await
        .unwrap_err();
    assert!(err.contains("格式异常"), "got: {err}");
}
