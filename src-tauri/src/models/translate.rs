//! 翻译与历史记录模型。

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ProviderInfo {
    pub id: String,
    pub name: String,
    pub requires_key: bool,
    pub ready: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TranslateRequest {
    pub text: String,
    pub direction: String,
    pub provider: Option<String>,
}

impl Default for TranslateRequest {
    fn default() -> Self {
        Self {
            text: String::new(),
            direction: "auto".into(),
            provider: None,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct TranslateResult {
    pub text: String,
    pub provider: String,
    pub provider_name: String,
    pub source: String,
    pub target: String,
    pub latency_ms: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub id: String,
    pub text: String,
    pub result: String,
    pub source: String,
    pub target: String,
    pub direction: String,
    pub provider: String,
    pub provider_name: String,
    pub favorite: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase", default)]
pub struct HistoryQuery {
    pub keyword: Option<String>,
    pub direction: Option<String>,
    pub provider: Option<String>,
    pub favorites_only: bool,
    pub limit: u32,
    pub offset: u32,
}

impl Default for HistoryQuery {
    fn default() -> Self {
        Self {
            keyword: None,
            direction: None,
            provider: None,
            favorites_only: false,
            limit: 200,
            offset: 0,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct HistoryPage {
    pub entries: Vec<HistoryEntry>,
    pub total: u32,
}
