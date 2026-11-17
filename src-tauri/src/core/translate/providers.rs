//! 翻译服务实现：微软、Google、OpenAI、DeepL、百度、有道。

use serde_json::{json, Value};

use super::{client, extract_text, network_error, parse_json, scalar, truncate, MAX_DETAIL};
use crate::models::AppConfig;

const DEEPL_DEFAULT_ENDPOINT: &str = "https://api-free.deepl.com/v2/translate";

pub(crate) async fn microsoft(text: &str, source: &str, target: &str) -> Result<String, String> {
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

pub(crate) async fn google(text: &str, target: &str) -> Result<String, String> {
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

pub(crate) async fn openai(text: &str, target: &str, config: &AppConfig) -> Result<String, String> {
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

pub(crate) async fn deepl(
    text: &str,
    source: &str,
    target: &str,
    config: &AppConfig,
) -> Result<String, String> {
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

pub(crate) async fn baidu(text: &str, target: &str, config: &AppConfig) -> Result<String, String> {
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

pub(crate) async fn youdao(text: &str, target: &str, config: &AppConfig) -> Result<String, String> {
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

pub(crate) fn baidu_sign(app_id: &str, text: &str, salt: &str, secret: &str) -> String {
    use md5::{Digest, Md5};
    let mut hasher = Md5::new();
    hasher.update(format!("{app_id}{text}{salt}{secret}").as_bytes());
    format!("{:x}", hasher.finalize())
}

pub(crate) fn youdao_truncate(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    if len <= 20 {
        return text.to_string();
    }
    let head: String = chars[..10].iter().collect();
    let tail: String = chars[len - 10..].iter().collect();
    format!("{head}{len}{tail}")
}

pub(crate) fn youdao_sign(
    app_key: &str,
    text: &str,
    salt: &str,
    curtime: &str,
    secret: &str,
) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(
        format!("{app_key}{}{salt}{curtime}{secret}", youdao_truncate(text)).as_bytes(),
    );
    format!("{:x}", hasher.finalize())
}
