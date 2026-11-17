//! LLM 调试台：模型配置、请求与用量统计模型。

use serde::{Deserialize, Serialize};

fn default_temperature() -> f64 {
    0.7
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase", default)]
pub struct LlmProviderConfig {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub model: String,
    /// 引用凭据管理器中的条目，空串表示尚未选择。
    pub key_entry_id: String,
    pub temperature: f64,
}

impl Default for LlmProviderConfig {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            base_url: String::new(),
            model: String::new(),
            key_entry_id: String::new(),
            temperature: default_temperature(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct LlmTemplate {
    pub id: String,
    pub name: String,
    pub content: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase", default)]
pub struct LlmConfig {
    pub providers: Vec<LlmProviderConfig>,
    pub templates: Vec<LlmTemplate>,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            providers: default_providers(),
            templates: default_templates(),
        }
    }
}

impl LlmConfig {
    pub fn provider(&self, id: &str) -> Option<&LlmProviderConfig> {
        self.providers.iter().find(|item| item.id == id)
    }
}

fn provider(id: &str, name: &str, base_url: &str, model: &str, key_entry_id: &str) -> LlmProviderConfig {
    LlmProviderConfig {
        id: id.into(),
        name: name.into(),
        base_url: base_url.into(),
        model: model.into(),
        key_entry_id: key_entry_id.into(),
        temperature: default_temperature(),
    }
}

pub fn default_providers() -> Vec<LlmProviderConfig> {
    vec![
        provider(
            "openai",
            "OpenAI",
            "https://api.openai.com/v1",
            "gpt-4o-mini",
            crate::core::secrets::BUILTIN_OPENAI,
        ),
        provider(
            "glm",
            "智谱 GLM",
            "https://open.bigmodel.cn/api/paas/v4",
            "glm-4-flash",
            "",
        ),
        provider(
            "deepseek",
            "DeepSeek",
            "https://api.deepseek.com/v1",
            "deepseek-chat",
            "",
        ),
    ]
}

fn default_templates() -> Vec<LlmTemplate> {
    vec![
        LlmTemplate {
            id: "template-translate".into(),
            name: "翻译".into(),
            content: "请把下面的文本翻译成{{target}}，保持原意与格式，只输出译文：\n\n{{text}}".into(),
        },
        LlmTemplate {
            id: "template-explain".into(),
            name: "代码解释".into(),
            content: "请解释以下{{language}}代码的作用，并指出潜在问题：\n\n{{code}}".into(),
        },
    ]
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct LlmRequest {
    pub provider_ids: Vec<String>,
    pub system: String,
    pub prompt: String,
    pub temperature: Option<f64>,
    pub max_tokens: Option<u32>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct LlmResponse {
    pub provider_id: String,
    pub provider_name: String,
    pub model: String,
    pub content: String,
    pub error: String,
    pub latency_ms: u64,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct LlmUsageDay {
    pub date: String,
    pub calls: u64,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct LlmUsageModel {
    pub model: String,
    pub provider_name: String,
    pub calls: u64,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct LlmUsageSummary {
    pub days: Vec<LlmUsageDay>,
    pub models: Vec<LlmUsageModel>,
    pub total_calls: u64,
    pub total_prompt_tokens: u64,
    pub total_completion_tokens: u64,
}
