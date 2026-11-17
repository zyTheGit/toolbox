//! LLM 调试台：并行请求 OpenAI 兼容接口，逐模型返回结果。

mod usage;

use std::sync::OnceLock;
use std::time::{Duration, Instant};

use futures_util::future::join_all;
use reqwest::Client;
use serde_json::{json, Value};

use crate::core::secrets::SecretStore;
use crate::models::{AppConfig, LlmProviderConfig, LlmRequest, LlmResponse};

pub use usage::LlmUsageStore;

const MAX_DETAIL: usize = 300;

static CLIENT: OnceLock<Client> = OnceLock::new();

pub fn validate_request(request: &LlmRequest) -> Result<(), String> {
    if request.prompt.trim().is_empty() {
        return Err("[PARAM] 请输入 Prompt".into());
    }
    if request.provider_ids.is_empty() {
        return Err("[PARAM] 请至少选择一个模型".into());
    }
    for name in placeholders(request.prompt.trim())? {
        return Err(format!("[PARAM] 变量 {{{{{name}}}}} 缺少值"));
    }
    if let Some(max) = request.max_tokens {
        if max == 0 || max > 32_768 {
            return Err("[PARAM] 最大输出 Token 需在 1 到 32768 之间".into());
        }
    }
    Ok(())
}

/// 提取仍未替换的 `{{name}}` 占位符；未闭合或空占位符直接报错。
pub fn placeholders(text: &str) -> Result<Vec<String>, String> {
    let mut out: Vec<String> = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find("{{") {
        let after = &rest[start + 2..];
        let Some(end) = after.find("}}") else {
            return Err("[PARAM] Prompt 中存在未闭合的变量占位符".into());
        };
        let name = after[..end].trim();
        if name.is_empty() {
            return Err("[PARAM] Prompt 中存在空的变量占位符".into());
        }
        if !out.iter().any(|item| item == name) {
            out.push(name.to_string());
        }
        rest = &after[end + 2..];
    }
    Ok(out)
}

pub async fn run(
    request: &LlmRequest,
    config: &AppConfig,
    secrets: &SecretStore,
) -> Vec<LlmResponse> {
    let mut seen = std::collections::HashSet::new();
    let tasks = request
        .provider_ids
        .iter()
        .filter(|id| seen.insert((*id).clone()))
        .map(|id| {
            let provider = config.llm.provider(id).cloned();
            let request = request.clone();
            let id = id.clone();
            async move {
                match provider {
                    Some(provider) => run_one(&provider, &request, secrets).await,
                    None => LlmResponse {
                        provider_id: id,
                        provider_name: "未知模型".into(),
                        error: "[PARAM] 模型配置不存在，可能已被删除".into(),
                        ..LlmResponse::default()
                    },
                }
            }
        });
    join_all(tasks).await
}

async fn run_one(
    provider: &LlmProviderConfig,
    request: &LlmRequest,
    secrets: &SecretStore,
) -> LlmResponse {
    let started = Instant::now();
    let mut response = LlmResponse {
        provider_id: provider.id.clone(),
        provider_name: provider.name.clone(),
        model: provider.model.clone(),
        ..LlmResponse::default()
    };

    let Some(api_key) = secrets.secret_for(&provider.key_entry_id) else {
        response.error = "[PARAM] 未选择凭据或凭据内容为空，请先在 API Key 管理器中配置".into();
        return response;
    };
    let endpoint = match endpoint(&provider.base_url) {
        Ok(endpoint) => endpoint,
        Err(err) => {
            response.error = err;
            return response;
        }
    };

    let temperature = request
        .temperature
        .unwrap_or(provider.temperature)
        .clamp(0.0, 2.0);
    let mut messages = Vec::new();
    let system = request.system.trim();
    if !system.is_empty() {
        messages.push(json!({ "role": "system", "content": system }));
    }
    messages.push(json!({ "role": "user", "content": request.prompt.trim() }));

    let mut body = json!({
        "model": provider.model,
        "messages": messages,
        "temperature": temperature,
        "stream": false
    });
    if let Some(max_tokens) = request.max_tokens {
        body["max_tokens"] = json!(max_tokens);
    }

    let client = match client() {
        Ok(client) => client,
        Err(err) => {
            response.error = err;
            return response;
        }
    };
    let sent = client
        .post(endpoint)
        .bearer_auth(api_key)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .json(&body)
        .send()
        .await;

    response.latency_ms = started.elapsed().as_millis() as u64;
    let res = match sent {
        Ok(res) => res,
        Err(err) => {
            response.error = if err.is_timeout() {
                "[NETWORK] 请求超时".into()
            } else if err.is_connect() {
                "[NETWORK] 连接失败".into()
            } else {
                format!("[NETWORK] 请求失败: {err}")
            };
            return response;
        }
    };

    let status = res.status();
    if !status.is_success() {
        let detail = res.text().await.unwrap_or_default();
        let detail = detail.trim().replace(['\r', '\n'], " ");
        response.error = if detail.is_empty() {
            format!("[NETWORK] HTTP {status}")
        } else {
            format!("[NETWORK] HTTP {status} {}", truncate(&detail, MAX_DETAIL))
        };
        return response;
    }

    let data = match res.json::<Value>().await {
        Ok(data) => data,
        Err(err) => {
            response.error = format!("[DATA] 解析响应失败: {err}");
            return response;
        }
    };

    if let Some(model) = data.get("model").and_then(Value::as_str) {
        if !model.trim().is_empty() {
            response.model = model.trim().to_string();
        }
    }
    response.content = content_of(&data);
    if response.content.trim().is_empty() {
        response.error = "[DATA] 模型返回内容为空".into();
        return response;
    }
    fill_usage(&mut response, data.get("usage"));
    response
}

fn endpoint(base: &str) -> Result<String, String> {
    let base = base.trim().trim_end_matches('/');
    if base.is_empty() {
        return Err("[PARAM] 未配置接口地址".into());
    }
    if base.ends_with("/chat/completions") {
        Ok(base.to_string())
    } else {
        Ok(format!("{base}/chat/completions"))
    }
}

fn content_of(data: &Value) -> String {
    let Some(content) = data
        .get("choices")
        .and_then(|item| item.get(0))
        .and_then(|item| item.get("message"))
        .and_then(|item| item.get("content"))
    else {
        return String::new();
    };
    match content {
        Value::String(text) => text.clone(),
        Value::Array(items) => items
            .iter()
            .filter_map(|item| item.get("text").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join(""),
        _ => String::new(),
    }
}

fn fill_usage(response: &mut LlmResponse, usage: Option<&Value>) {
    let Some(usage) = usage else { return };
    let pick = |keys: [&str; 2]| {
        keys.iter()
            .find_map(|key| usage.get(*key).and_then(Value::as_u64))
            .unwrap_or(0)
    };
    response.prompt_tokens = pick(["prompt_tokens", "input_tokens"]);
    response.completion_tokens = pick(["completion_tokens", "output_tokens"]);
    response.total_tokens = usage
        .get("total_tokens")
        .and_then(Value::as_u64)
        .unwrap_or(response.prompt_tokens + response.completion_tokens);
}

fn client() -> Result<&'static Client, String> {
    if let Some(existing) = CLIENT.get() {
        return Ok(existing);
    }
    let built = Client::builder()
        .timeout(Duration::from_secs(120))
        .connect_timeout(Duration::from_secs(15))
        .build()
        .map_err(|err| format!("[NETWORK] 初始化网络客户端失败: {err}"))?;
    let _ = CLIENT.set(built);
    CLIENT
        .get()
        .ok_or_else(|| "[NETWORK] 初始化网络客户端失败".to_string())
}

fn truncate(value: &str, max: usize) -> String {
    if value.chars().count() <= max {
        value.to_string()
    } else {
        value.chars().take(max).collect::<String>() + "…"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholders_detect_missing_vars() {
        assert_eq!(placeholders("hi").unwrap(), Vec::<String>::new());
        assert_eq!(
            placeholders("翻译{{text}}成{{target}}，{{text}}").unwrap(),
            vec!["text".to_string(), "target".to_string()]
        );
        assert!(placeholders("bad {{name").is_err());
        assert!(placeholders("bad {{  }}").is_err());
    }

    #[test]
    fn endpoint_appends_chat_completions_once() {
        assert_eq!(
            endpoint("https://api.openai.com/v1").unwrap(),
            "https://api.openai.com/v1/chat/completions"
        );
        assert_eq!(
            endpoint("https://api.openai.com/v1/chat/completions/").unwrap(),
            "https://api.openai.com/v1/chat/completions"
        );
        assert!(endpoint("  ").is_err());
    }

    #[test]
    fn content_supports_text_parts() {
        let plain = json!({ "choices": [{ "message": { "content": "hello" } }] });
        assert_eq!(content_of(&plain), "hello");
        let parts = json!({
            "choices": [{ "message": { "content": [
                { "type": "text", "text": "a" }, { "type": "text", "text": "b" }
            ] } }]
        });
        assert_eq!(content_of(&parts), "ab");
        assert_eq!(content_of(&json!({})), "");
    }

    #[test]
    fn validate_rejects_unreplaced_vars() {
        let request = LlmRequest {
            provider_ids: vec!["openai".into()],
            prompt: "翻译 {{text}}".into(),
            ..LlmRequest::default()
        };
        assert!(validate_request(&request).is_err());
        let request = LlmRequest {
            provider_ids: vec!["openai".into()],
            prompt: "翻译 done".into(),
            ..LlmRequest::default()
        };
        assert!(validate_request(&request).is_ok());
    }
}
