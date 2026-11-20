//! 版本检查：只回答“有没有新版本”，下载安装仍由用户在浏览器里完成。
//!
//! 端点固定在本模块（HTTPS + 本仓库），前端不能指定检查地址，因此这个 command
//! 不会被当成通用 HTTP 代理。返回的 Release 页面地址在返回前与打开前都要按前缀
//! 校验，避免远端把“打开链接”指向站外。

use std::time::Duration;

use semver::Version;
use serde::{Deserialize, Serialize};

use crate::models::AppConfig;

const LATEST_RELEASE_URL: &str = "https://api.github.com/repos/zyTheGit/toolbox/releases/latest";
const RELEASE_PAGE_PREFIX: &str = "https://github.com/zyTheGit/toolbox/releases/";
/// GitHub API 拒绝无 User-Agent 的请求，这里必须随请求发出。
const USER_AGENT: &str = "Toolbox-desktop";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCheckDto {
    pub current_version: String,
    pub latest_version: String,
    pub available: bool,
    /// 当前版本高于线上最新版。正常发版不会出现，出现即说明安装包的版本号被误标，
    /// 界面不能只说“已是最新”。
    pub current_newer: bool,
    /// Release 页面（不是安装包直链）：由系统浏览器打开，用户自己选择下载。
    pub page_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub published_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ReleasePayload {
    tag_name: String,
    html_url: String,
    #[serde(default)]
    published_at: Option<String>,
}

/// 版本检查用客户端：超时短、跟随全局代理设置（与下载器一致）。
fn client(config: &AppConfig) -> Result<reqwest::Client, String> {
    let mut builder = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .connect_timeout(Duration::from_secs(8))
        .timeout(Duration::from_secs(10));

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

pub async fn check(config: &AppConfig, current_version: &str) -> Result<UpdateCheckDto, String> {
    let client = client(config)?;
    let response = client
        .get(LATEST_RELEASE_URL)
        .header(reqwest::header::ACCEPT, "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| format!("[NETWORK] 无法获取最新版本信息: {e}"))?;
    if !response.status().is_success() {
        return Err(format!(
            "[NETWORK] 版本信息接口返回状态 {}",
            response.status()
        ));
    }
    let body = response
        .text()
        .await
        .map_err(|e| format!("[NETWORK] 读取版本信息失败: {e}"))?;
    parse_release(&body, current_version)
}

/// 纯解析：与网络无关，便于用固定 JSON 覆盖各版本比较分支。
pub(crate) fn parse_release(body: &str, current_version: &str) -> Result<UpdateCheckDto, String> {
    let release: ReleasePayload = serde_json::from_str(body)
        .map_err(|e| format!("[DATA] 版本信息格式异常: {e}"))?;
    if !is_pinned_release_page(&release.html_url) {
        return Err(off_repo_error());
    }
    let latest = parse_version(&release.tag_name)?;
    let current = parse_version(current_version)?;
    Ok(UpdateCheckDto {
        current_version: current.to_string(),
        latest_version: latest.to_string(),
        available: latest > current,
        current_newer: latest < current,
        page_url: release.html_url,
        published_at: normalize_published_at(release.published_at),
    })
}

/// 标签允许 `v0.2.0` 与 `0.2.0` 两种写法；其余一律拒绝，不做模糊比较。
fn parse_version(value: &str) -> Result<Version, String> {
    Version::parse(value.trim().trim_start_matches('v'))
        .map_err(|e| format!("[DATA] 版本号无法解析: {e}"))
}

/// API 返回 `2026-11-02T06:40:11Z`，界面只需要日期部分。
fn normalize_published_at(value: Option<String>) -> Option<String> {
    value.and_then(|raw| {
        let date = raw.split('T').next().unwrap_or_default();
        (!date.is_empty()).then(|| date.to_owned())
    })
}

/// Release 页面必须落在本仓库的 releases 路径内。前缀已含协议与主机名，
/// 所以 `releases/../..` 这类路径技巧也跳不出 github.com 上的本页范围。
fn is_pinned_release_page(url: &str) -> bool {
    url.starts_with(RELEASE_PAGE_PREFIX)
}

fn off_repo_error() -> String {
    "[DATA] 下载地址不属于本仓库的发布页".into()
}

/// command 层复用：界面回传 URL 时同样在 Rust 侧校验，不依赖前端判断。
pub fn ensure_release_page(url: &str) -> Result<(), String> {
    if is_pinned_release_page(url) {
        Ok(())
    } else {
        Err(off_repo_error())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn payload(tag: &str, url: &str) -> String {
        format!(
            r#"{{"tag_name":"{tag}","html_url":"{url}","published_at":"2026-11-02T06:40:11Z"}}"#
        )
    }

    const PAGE: &str = "https://github.com/zyTheGit/toolbox/releases/tag/v0.3.1";

    #[test]
    fn release_endpoint_is_pinned_to_this_repository() {
        // 仓库改名会静默让检查失效，端点漂移必须在这里先失败。
        assert_eq!(
            LATEST_RELEASE_URL,
            "https://api.github.com/repos/zyTheGit/toolbox/releases/latest"
        );
    }

    #[test]
    fn newer_tag_strips_prefix_and_reports_update_available() {
        let dto = parse_release(&payload("v0.4.0", PAGE), "0.3.1").expect("解析失败");
        assert!(dto.available);
        assert!(!dto.current_newer);
        assert_eq!(dto.latest_version, "0.4.0");
        assert_eq!(dto.current_version, "0.3.1");
        assert_eq!(dto.page_url, PAGE);
        assert_eq!(dto.published_at.as_deref(), Some("2026-11-02"));
    }

    #[test]
    fn same_version_reports_no_update() {
        let dto = parse_release(&payload("0.3.1", PAGE), "0.3.1").expect("解析失败");
        assert!(!dto.available);
        assert!(!dto.current_newer);
    }

    #[test]
    fn older_tag_never_prompts_downgrade() {
        let dto = parse_release(&payload("v0.2.0", PAGE), "0.3.1").expect("解析失败");
        assert!(!dto.available);
        assert!(dto.current_newer);
    }

    /// 误 bump 退回的安装包必须单独标出，界面才能提示“版本号可能标注有误”，
    /// 而不是显示“已是最新版本”。
    #[test]
    fn mislabeled_installed_version_is_flagged_as_newer_than_latest() {
        let dto = parse_release(&payload("v0.3.1", PAGE), "3.0.1-rc-2").expect("解析失败");
        assert!(!dto.available);
        assert!(dto.current_newer);
        assert_eq!(dto.latest_version, "0.3.1");
    }

    #[test]
    fn prerelease_tag_is_not_offered_as_newer() {
        let dto = parse_release(&payload("v0.3.1-rc.1", PAGE), "0.3.1").expect("解析失败");
        assert!(!dto.available);
    }

    #[test]
    fn two_digit_component_outperforms_lexical_compare() {
        let dto = parse_release(&payload("v0.10.0", PAGE), "0.9.0").expect("解析失败");
        assert!(dto.available);
    }

    #[test]
    fn malformed_tag_is_rejected() {
        assert!(parse_release(&payload("nightly-2026", PAGE), "0.3.1").is_err());
    }

    #[test]
    fn non_json_body_is_rejected() {
        assert!(parse_release("<html>rate limited</html>", "0.3.1").is_err());
    }

    #[test]
    fn release_url_outside_repository_is_rejected() {
        let body = payload("v0.4.0", "https://example.test/evil.exe");
        assert!(parse_release(&body, "0.3.1").is_err());
    }

    #[test]
    fn opened_page_is_validated_on_the_rust_side() {
        assert!(ensure_release_page(PAGE).is_ok());
        // 同一仓库的资产下载页同样允许。
        assert!(ensure_release_page(
            "https://github.com/zyTheGit/toolbox/releases/download/v0.3.1/Toolbox-0.3.1.zip"
        )
        .is_ok());
        for rejected in [
            // 其他仓库的发布页。
            "https://github.com/zyTheGit/gitdock/releases/tag/v0.2.0",
            // 主机名后缀伪装与协议降级。
            "https://github.com.evil.test/zyTheGit/toolbox/releases/tag/v0.3.1",
            "http://github.com/zyTheGit/toolbox/releases/tag/v0.3.1",
            "file:///C:/Windows/System32/calc.exe",
        ] {
            assert!(ensure_release_page(rejected).is_err());
        }
    }
}
