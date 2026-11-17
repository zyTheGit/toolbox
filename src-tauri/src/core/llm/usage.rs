//! LLM 用量记录与聚合。

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

use chrono::{Local, SecondsFormat};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use crate::core::persist;
use crate::models::{LlmResponse, LlmUsageDay, LlmUsageModel, LlmUsageSummary};

const MAX_RECORDS: usize = 5000;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UsageRecord {
    pub at: String,
    pub provider_name: String,
    pub model: String,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
}

pub struct LlmUsageStore {
    path: PathBuf,
    records: Mutex<Vec<UsageRecord>>,
}

impl LlmUsageStore {
    pub fn load(path: PathBuf) -> (Self, Option<String>) {
        let (records, warning) = match persist::load_json::<Vec<UsageRecord>>(&path) {
            Ok(Some(records)) => (records, None),
            Ok(None) => (Vec::new(), None),
            Err(err) => (Vec::new(), Some(err)),
        };
        (
            Self {
                path,
                records: Mutex::new(records),
            },
            warning,
        )
    }

    pub fn record(&self, responses: &[LlmResponse]) -> Result<(), String> {
        let mut records = self.records.lock();
        let previous_len = records.len();
        let now = Local::now().to_rfc3339_opts(SecondsFormat::Secs, true);
        for response in responses {
            if !response.error.is_empty() {
                continue;
            }
            records.push(UsageRecord {
                at: now.clone(),
                provider_name: response.provider_name.clone(),
                model: response.model.clone(),
                prompt_tokens: response.prompt_tokens,
                completion_tokens: response.completion_tokens,
            });
        }
        if records.len() > MAX_RECORDS {
            let excess = records.len() - MAX_RECORDS;
            records.drain(0..excess);
        }
        if records.len() == previous_len {
            return Ok(());
        }
        if let Err(err) = persist::save_json(&self.path, &*records) {
            records.truncate(previous_len);
            return Err(format!("[DATA] 保存 LLM 用量失败: {err}"));
        }
        Ok(())
    }

    pub fn summary(&self, days: u32) -> LlmUsageSummary {
        summarize(&self.records.lock(), days)
    }
}

pub(crate) fn summarize(records: &[UsageRecord], days: u32) -> LlmUsageSummary {
    let days = days.clamp(1, 365) as i64;
    let cutoff = (Local::now().date_naive() - chrono::Duration::days(days - 1))
        .format("%Y-%m-%d")
        .to_string();

    let mut by_day: BTreeMap<String, LlmUsageDay> = BTreeMap::new();
    let mut by_model: HashMap<(String, String), LlmUsageModel> = HashMap::new();
    let mut total = LlmUsageSummary::default();

    for record in records {
        let date = record.at.get(..10).unwrap_or_default();
        if date.len() != 10 || date < cutoff.as_str() {
            continue;
        }
        let day = by_day.entry(date.to_string()).or_insert_with(|| LlmUsageDay {
            date: date.to_string(),
            ..LlmUsageDay::default()
        });
        day.calls += 1;
        day.prompt_tokens += record.prompt_tokens;
        day.completion_tokens += record.completion_tokens;

        let key = (record.provider_name.clone(), record.model.clone());
        let model = by_model.entry(key).or_insert_with(|| LlmUsageModel {
            model: record.model.clone(),
            provider_name: record.provider_name.clone(),
            ..LlmUsageModel::default()
        });
        model.calls += 1;
        model.prompt_tokens += record.prompt_tokens;
        model.completion_tokens += record.completion_tokens;

        total.total_calls += 1;
        total.total_prompt_tokens += record.prompt_tokens;
        total.total_completion_tokens += record.completion_tokens;
    }

    total.days = by_day.into_values().collect();
    let mut models: Vec<LlmUsageModel> = by_model.into_values().collect();
    models.sort_by(|a, b| {
        let left = a.prompt_tokens + a.completion_tokens;
        let right = b.prompt_tokens + b.completion_tokens;
        right.cmp(&left).then_with(|| a.model.cmp(&b.model))
    });
    total.models = models;
    total
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(at: &str, model: &str, prompt: u64, completion: u64) -> UsageRecord {
        UsageRecord {
            at: format!("{at}T10:00:00+08:00"),
            provider_name: "测试".into(),
            model: model.into(),
            prompt_tokens: prompt,
            completion_tokens: completion,
        }
    }

    #[test]
    fn summarize_aggregates_days_and_models() {
        let today = Local::now().date_naive().format("%Y-%m-%d").to_string();
        let yesterday = (Local::now().date_naive() - chrono::Duration::days(1))
            .format("%Y-%m-%d")
            .to_string();
        let old = (Local::now().date_naive() - chrono::Duration::days(40))
            .format("%Y-%m-%d")
            .to_string();
        let records = vec![
            record(&today, "gpt-4o-mini", 10, 20),
            record(&today, "gpt-4o-mini", 5, 5),
            record(&yesterday, "deepseek-chat", 7, 3),
            record(&old, "old-model", 100, 100),
        ];
        let summary = summarize(&records, 7);
        assert_eq!(summary.total_calls, 3);
        assert_eq!(summary.total_prompt_tokens, 22);
        assert_eq!(summary.total_completion_tokens, 28);
        assert_eq!(summary.days.len(), 2);
        assert_eq!(summary.models.len(), 2);
        assert_eq!(summary.models[0].model, "gpt-4o-mini");
        assert_eq!(summary.models[0].calls, 2);
    }

    #[test]
    fn summarize_ignores_malformed_timestamps() {
        let mut bad = record("2026-01-01", "x", 1, 1);
        bad.at = "oops".into();
        let summary = summarize(&[bad], 7);
        assert_eq!(summary.total_calls, 0);
    }
}
