use std::sync::OnceLock;
use std::time::{Duration, Instant};

use reqwest::{Client, Response};
use serde_json::{json, Value};

use crate::models::{AppConfig, ProviderInfo, TranslateRequest, TranslateResult};

const UA: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Safari/537.36 Toolbox/0.1";
const PROVIDER_ORDER: [&str; 6] = ["microsoft", "google", "openai", "deepl", "baidu", "youdao"];
const DEEPL_DEFAULT_ENDPOINT: &str = "https://api-free.deepl.com/v2/translate";
const MAX_DETAIL: usize = 160;

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
            "microsoft" => microsoft(text, source, target).await,
            "google" => google(text, target).await,
            "openai" => openai(text, target, config).await,
            "deepl" => deepl(text, source, target, config).await,
            "baidu" => baidu(text, target, config).await,
            "youdao" => youdao(text, target, config).await,
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

async fn microsoft(text: &str, source: &str, target: &str) -> Result<String, String> {
    let url = reqwest::Url::parse("https://edge.microsoft.com/translate/translatetext")
        .map_err(|e| format!("构造请求失败: {e}"))?;
    let mut url = url;
    url.query_pairs_mut()
        .append_pair("from", if source == "zh" { "zh-Hans" } else { "en" })
        .append_pair("to", if target == "zh" { "zh-Hans" } else { "en" })
        .append_pair("isEnterpriseClient", "false");

    let res = client(false)?
        .post(url)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .json(&json!([text]))
        .send()
        .await
        .map_err(network_error)?;
    let data = parse_json(res).await?;

    extract_text(
        data.get(0)
            .and_then(|item| item.get("translations"))
            .and_then(|item| item.get(0))
            .and_then(|item| item.get("text")),
    )
}

async fn google(text: &str, target: &str) -> Result<String, String> {
    let mut url = reqwest::Url::parse("https://translate.googleapis.com/translate_a/single")
        .map_err(|e| format!("构造请求失败: {e}"))?;
    url.query_pairs_mut()
        .append_pair("client", "gtx")
        .append_pair("sl", "auto")
        .append_pair("tl", if target == "zh" { "zh-CN" } else { "en" })
        .append_pair("dt", "t")
        .append_pair("q", text);

    let res = client(false)?.get(url).send().await.map_err(network_error)?;
    let data = parse_json(res).await?;

    let out: String = data
        .get(0)
        .and_then(Value::as_array)
        .map(|segments| {
            segments
                .iter()
                .filter_map(|segment| segment.get(0).and_then(Value::as_str))
                .collect()
        })
        .unwrap_or_default();
    if out.trim().is_empty() {
        return Err("返回结果为空".into());
    }
    Ok(out)
}

async fn openai(text: &str, target: &str, config: &AppConfig) -> Result<String, String> {
    let conf = &config.translate.openai;
    let mut base = conf.base_url.trim().trim_end_matches('/').to_string();
    if base.is_empty() {
        base = "https://api.openai.com/v1".into();
    }
    let endpoint = if base.ends_with("/chat/completions") {
        base
    } else {
        format!("{base}/chat/completions")
    };
    let model = if conf.model.trim().is_empty() {
        "gpt-4o-mini"
    } else {
        conf.model.trim()
    };
    let instruction = if target == "en" {
        "You are a professional translator. Translate the user text into natural English. Preserve meaning, tone and formatting. Output only the translation."
    } else {
        "你是一名专业翻译。请将用户文本翻译成自然流畅的简体中文，保持原意、语气与格式。只输出译文。"
    };

    let body = json!({
        "model": model,
        "messages": [
            { "role": "system", "content": instruction },
            { "role": "user", "content": text }
        ],
        "temperature": 0.2
    });

    let res = client(true)?
        .post(endpoint)
        .bearer_auth(conf.api_key.trim())
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .json(&body)
        .send()
        .await
        .map_err(network_error)?;

    if !res.status().is_success() {
        let status = res.status();
        let detail = res.text().await.unwrap_or_default();
        let detail = truncate(&detail.trim().replace(['\r', '\n'], " "), MAX_DETAIL);
        return Err(if detail.is_empty() {
            format!("HTTP {status}")
        } else {
            format!("HTTP {status} {detail}")
        });
    }

    let data = parse_json(res).await?;
    extract_text(
        data.get("choices")
            .and_then(|item| item.get(0))
            .and_then(|item| item.get("message"))
            .and_then(|item| item.get("content")),
    )
}

async fn deepl(text: &str, source: &str, target: &str, config: &AppConfig) -> Result<String, String> {
    let conf = &config.translate.deepl;
    let mut endpoint = conf.endpoint.trim().to_string();
    if endpoint.is_empty() {
        endpoint = DEEPL_DEFAULT_ENDPOINT.into();
    }

    let res = client(false)?
        .post(endpoint)
        .form(&[
            ("auth_key", conf.api_key.trim()),
            ("text", text),
            ("target_lang", if target == "zh" { "ZH" } else { "EN" }),
            ("source_lang", if source == "zh" { "ZH" } else { "EN" }),
        ])
        .send()
        .await
        .map_err(network_error)?;
    let data = parse_json(res).await?;

    extract_text(
        data.get("translations")
            .and_then(|item| item.get(0))
            .and_then(|item| item.get("text")),
    )
}

async fn baidu(text: &str, target: &str, config: &AppConfig) -> Result<String, String> {
    let conf = &config.translate.baidu;
    let salt = chrono::Local::now().timestamp_millis().to_string();
    let sign = baidu_sign(
        conf.app_id.trim(),
        text,
        &salt,
        conf.secret_key.trim(),
    );

    let mut url = reqwest::Url::parse("https://fanyi-api.baidu.com/api/trans/vip/translate")
        .map_err(|e| format!("构造请求失败: {e}"))?;
    url.query_pairs_mut()
        .append_pair("q", text)
        .append_pair("from", "auto")
        .append_pair("to", if target == "zh" { "zh" } else { "en" })
        .append_pair("appid", conf.app_id.trim())
        .append_pair("salt", &salt)
        .append_pair("sign", &sign);

    let res = client(false)?.get(url).send().await.map_err(network_error)?;
    let data = parse_json(res).await?;

    if let Some(code) = scalar(data.get("error_code")) {
        if code != "0" {
            let message = scalar(data.get("error_msg")).unwrap_or_default();
            return Err(format!("{code} {message}").trim().to_string());
        }
    }

    let out = data
        .get("trans_result")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.get("dst").and_then(Value::as_str))
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default();
    if out.trim().is_empty() {
        return Err("返回结果为空".into());
    }
    Ok(out)
}

async fn youdao(text: &str, target: &str, config: &AppConfig) -> Result<String, String> {
    let conf = &config.translate.youdao;
    let salt = uuid::Uuid::new_v4().to_string();
    let curtime = chrono::Local::now().timestamp().to_string();
    let sign = youdao_sign(
        conf.app_key.trim(),
        text,
        &salt,
        &curtime,
        conf.app_secret.trim(),
    );

    let res = client(false)?
        .post("https://openapi.youdao.com/api")
        .form(&[
            ("q", text),
            ("from", "auto"),
            ("to", if target == "zh" { "zh-CHS" } else { "en" }),
            ("appKey", conf.app_key.trim()),
            ("salt", &salt),
            ("sign", &sign),
            ("signType", "v3"),
            ("curtime", &curtime),
        ])
        .send()
        .await
        .map_err(network_error)?;
    let data = parse_json(res).await?;

    if let Some(code) = scalar(data.get("errorCode")) {
        if code != "0" {
            return Err(format!("错误码 {code}"));
        }
    }

    let out = data
        .get("translation")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default();
    if out.trim().is_empty() {
        return Err("返回结果为空".into());
    }
    Ok(out)
}

fn provider_name(id: &str) -> &'static str {
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

fn client(long_request: bool) -> Result<&'static Client, String> {
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

fn network_error(err: reqwest::Error) -> String {
    if err.is_timeout() {
        "请求超时".to_string()
    } else if err.is_connect() {
        "连接失败".to_string()
    } else {
        format!("网络请求失败: {err}")
    }
}

async fn parse_json(res: Response) -> Result<Value, String> {
    if !res.status().is_success() {
        return Err(format!("HTTP {}", res.status()));
    }
    res.json::<Value>()
        .await
        .map_err(|e| format!("解析响应失败: {e}"))
}

fn extract_text(value: Option<&Value>) -> Result<String, String> {
    let text = value.and_then(Value::as_str).unwrap_or_default().trim();
    if text.is_empty() {
        return Err("返回结果为空".into());
    }
    Ok(text.to_string())
}

fn scalar(value: Option<&Value>) -> Option<String> {
    match value {
        Some(Value::String(text)) => Some(text.clone()),
        Some(Value::Number(number)) => Some(number.to_string()),
        _ => None,
    }
}

fn truncate(value: &str, max: usize) -> String {
    if value.chars().count() <= max {
        value.to_string()
    } else {
        value.chars().take(max).collect()
    }
}

fn baidu_sign(app_id: &str, text: &str, salt: &str, secret: &str) -> String {
    use md5::{Digest, Md5};
    let mut hasher = Md5::new();
    hasher.update(format!("{app_id}{text}{salt}{secret}").as_bytes());
    format!("{:x}", hasher.finalize())
}

fn youdao_truncate(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    if len <= 20 {
        return text.to_string();
    }
    let head: String = chars[..10].iter().collect();
    let tail: String = chars[len - 10..].iter().collect();
    format!("{head}{len}{tail}")
}

fn youdao_sign(app_key: &str, text: &str, salt: &str, curtime: &str, secret: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(
        format!("{app_key}{}{salt}{curtime}{secret}", youdao_truncate(text)).as_bytes(),
    );
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

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
