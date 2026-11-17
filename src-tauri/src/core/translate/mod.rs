//! 翻译入口：服务发现、回退链调度与通用工具。

mod providers;

use std::sync::OnceLock;
use std::time::{Duration, Instant};

use reqwest::{Client, Response};
use serde_json::Value;

use crate::models::{AppConfig, ProviderInfo, TranslateRequest, TranslateResult};

pub(crate) const UA: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Safari/537.36 Toolbox/0.1";
const PROVIDER_ORDER: [&str; 6] = ["microsoft", "google", "openai", "deepl", "baidu", "youdao"];
pub(crate) const MAX_DETAIL: usize = 160;

static CLIENT: OnceLock<Client> = OnceLock::new();
static CLIENT_LONG: OnceLock<Client> = OnceLock::new();

pub fn providers(config: &AppConfig) -> Vec<ProviderInfo> {
    PROVIDER_ORDER
        .iter()
        .map(|id| ProviderInfo {
            id: (*id).to_string(),
            name: provider_name(id).to_string(),
            requires_key: requires_key(id),
            ready: is_ready(id, config),
        })
        .collect()
}

pub async fn translate(
    request: &TranslateRequest,
    config: &AppConfig,
) -> Result<TranslateResult, String> {
    let text = request.text.trim();
    if text.is_empty() {
        return Err("[PARAM] 请输入要翻译的内容".into());
    }

    let direction = match request.direction.trim() {
        "zh2en" => "zh2en",
        "en2zh" => "en2zh",
        _ => detect_direction(text),
    };
    let (source, target) = if direction == "zh2en" {
        ("zh", "en")
    } else {
        ("en", "zh")
    };

    let mut chain: Vec<String> = Vec::new();
    let override_provider = request
        .provider
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let primary = override_provider.unwrap_or_else(|| config.translate.provider.trim());
    if !primary.is_empty() {
        chain.push(primary.to_string());
    }
    for item in &config.translate.fallback {
        let id = item.trim();
        if !id.is_empty() && !chain.iter().any(|existing| existing == id) {
            chain.push(id.to_string());
        }
    }

    if chain.is_empty() {
        return Err("[PARAM] 没有可用的翻译服务，请先在设置中配置".into());
    }

    let mut errors: Vec<String> = Vec::new();
    for id in &chain {
        if !PROVIDER_ORDER.contains(&id.as_str()) {
            errors.push(format!("{id}: 未知翻译服务"));
            continue;
        }
        if !is_ready(id, config) {
            errors.push(format!("{}: 未配置密钥", provider_name(id)));
            continue;
        }

        let started = Instant::now();
        let outcome = match id.as_str() {
            "microsoft" => providers::microsoft(text, source, target).await,
            "google" => providers::google(text, target).await,
            "openai" => providers::openai(text, target, config).await,
            "deepl" => providers::deepl(text, source, target, config).await,
            "baidu" => providers::baidu(text, target,config).await,
            "youdao" => providers::youdao(text, target, config).await,
            _ => Err("未知翻译服务".to_string()),
        };

        match outcome {
            Ok(value) => {
                let value = value.trim();
                if value.is_empty() {
                    errors.push(format!("{}: 返回结果为空", provider_name(id)));
                    continue;
                }
                return Ok(TranslateResult {
                    text: value.to_string(),
                    provider: id.clone(),
                    provider_name: provider_name(id).to_string(),
                    source: source.to_string(),
                    target: target.to_string(),
                    latency_ms: started.elapsed().as_millis() as u64,
                });
            }
            Err(err) => errors.push(format!("{}: {err}", provider_name(id))),
        }
    }

    Err(format!("[NETWORK] 全部翻译服务失败：{}", errors.join("；")))
}

pub fn detect_direction(text: &str) -> &'static str {
    if text
        .chars()
        .any(|ch| ('\u{3400}'..='\u{9fff}').contains(&ch))
    {
        "zh2en"
    } else {
        "en2zh"
    }
}

pub(crate) fn provider_name(id: &str) -> &'static str {
    match id {
        "microsoft" => "微软翻译",
        "google" => "Google 翻译",
        "openai" => "OpenAI 兼容接口",
        "deepl" => "DeepL",
        "baidu" => "百度翻译",
        "youdao" => "有道翻译",
        _ => "未知服务",
    }
}

fn requires_key(id: &str) -> bool {
    !matches!(id, "microsoft" | "google")
}

fn is_ready(id: &str, config: &AppConfig) -> bool {
    let translate = &config.translate;
    match id {
        "microsoft" | "google" => true,
        "openai" => has_value(&translate.openai.api_key),
        "deepl" => has_value(&translate.deepl.api_key),
        "baidu" => has_value(&translate.baidu.app_id) && has_value(&translate.baidu.secret_key),
        "youdao" => has_value(&translate.youdao.app_key) && has_value(&translate.youdao.app_secret),
        _ => false,
    }
}

fn has_value(value: &str) -> bool {
    !value.trim().is_empty()
}

pub(crate) fn client(long_request: bool) -> Result<&'static Client, String> {
    let cell = if long_request { &CLIENT_LONG } else { &CLIENT };
    if let Some(existing) = cell.get() {
        return Ok(existing);
    }
    let timeout = if long_request { 60 } else { 30 };
    let built = Client::builder()
        .user_agent(UA)
        .no_proxy()
        .timeout(Duration::from_secs(timeout))
        .connect_timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| format!("初始化网络客户端失败: {e}"))?;
    let _ = cell.set(built);
    cell.get().ok_or_else(|| "初始化网络客户端失败".to_string())
}

pub(crate) fn network_error(err: reqwest::Error) -> String {
    if err.is_timeout() {
        "请求超时".to_string()
    } else if err.is_connect() {
        "连接失败".to_string()
    } else {
        format!("网络请求失败: {err}")
    }
}

pub(crate) async fn parse_json(res: Response) -> Result<Value, String> {
    if !res.status().is_success() {
        return Err(format!("HTTP {}", res.status()));
    }
    res.json::<Value>()
        .await
        .map_err(|e| format!("解析响应失败: {e}"))
}

pub(crate) fn extract_text(value: Option<&Value>) -> Result<String, String> {
    let text = value.and_then(Value::as_str).unwrap_or_default().trim();
    if text.is_empty() {
        return Err("返回结果为空".into());
    }
    Ok(text.to_string())
}

pub(crate) fn scalar(value: Option<&Value>) -> Option<String> {
    match value {
        Some(Value::String(text)) => Some(text.clone()),
        Some(Value::Number(number)) => Some(number.to_string()),
        _ => None,
    }
}

pub(crate) fn truncate(value: &str, max: usize) -> String {
    if value.chars().count() <= max {
        value.to_string()
    } else {
        value.chars().take(max).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::providers::{baidu_sign, youdao_sign, youdao_truncate};
    use super::detect_direction;

    #[test]
    fn detects_direction() {
        assert_eq!(detect_direction("你好，世界"), "zh2en");
        assert_eq!(detect_direction("Hello world"), "en2zh");
        assert_eq!(detect_direction("中文 mixed"), "zh2en");
        assert_eq!(detect_direction("12345"), "en2zh");
    }

    #[test]
    fn youdao_truncate_keeps_short_text() {
        assert_eq!(youdao_truncate("short"), "short");
        let text = "123456789012345678901";
        let truncated = youdao_truncate(text);
        assert_eq!(truncated, "1234567890212345678901");
    }

    #[test]
    fn youdao_sign_is_stable_hex() {
        let sign = youdao_sign("key", "hello", "salt", "1700000000", "secret");
        assert_eq!(sign.len(), 64);
        assert!(sign.chars().all(|ch| ch.is_ascii_hexdigit()));
        assert_eq!(sign, sign.to_lowercase());

        let long = "这是一段超过二十个字符的文本用于测试截断逻辑";
        let first = youdao_sign("key", long, "salt", "1700000000", "secret");
        let second = youdao_sign("key", long, "salt", "1700000000", "secret");
        assert_eq!(first, second);
    }

    #[test]
    fn baidu_sign_is_md5() {
        let sign = baidu_sign("app", "text", "salt", "secret");
        assert_eq!(sign.len(), 32);
        assert!(sign.chars().all(|ch| ch.is_ascii_hexdigit()));
    }
}
