use std::path::PathBuf;

use chrono::{Local, NaiveDateTime};
use parking_lot::{Mutex, MutexGuard};
use uuid::Uuid;

use crate::core::persist;
use crate::models::{HistoryEntry, HistoryPage, HistoryQuery, TranslateResult};

const DEDUP_WINDOW_SECS: i64 = 5 * 60;
const DEFAULT_LIMIT: u32 = 200;
const TIME_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

pub struct HistoryStore {
    path: PathBuf,
    entries: Mutex<Vec<HistoryEntry>>,
}

impl HistoryStore {
    pub fn new(path: PathBuf) -> (Self, Option<String>) {
        let (entries, warning) = match persist::load_json::<Vec<HistoryEntry>>(&path) {
            Ok(Some(entries)) => (entries, None),
            Ok(None) => (Vec::new(), None),
            Err(err) => (Vec::new(), Some(err)),
        };
        (
            Self {
                path,
                entries: Mutex::new(entries),
            },
            warning,
        )
    }

    pub fn add(
        &self,
        text: &str,
        result: &TranslateResult,
        max_entries: u32,
    ) -> Result<HistoryEntry, String> {
        let text = text.trim().to_string();
        if text.is_empty() {
            return Err("翻译内容不能为空".into());
        }
        let direction = if result.source == "zh" { "zh2en" } else { "en2zh" };
        let now = now_string();

        let mut entries = self.entries.lock();
        let previous = entries.clone();
        let entry = {
            let existing = entries.iter().position(|item| {
                item.text == text
                    && item.direction == direction
                    && item.provider == result.provider
                    && within_window(&item.updated_at)
            });

            if let Some(index) = existing {
                let mut entry = entries.remove(index);
                entry.result = result.text.clone();
                entry.updated_at = now;
                entries.insert(0, entry.clone());
                entry
            } else {
                let entry = HistoryEntry {
                    id: Uuid::new_v4().to_string(),
                    text,
                    result: result.text.clone(),
                    source: result.source.clone(),
                    target: result.target.clone(),
                    direction: direction.to_string(),
                    provider: result.provider.clone(),
                    provider_name: result.provider_name.clone(),
                    favorite: false,
                    created_at: now.clone(),
                    updated_at: now,
                };
                entries.insert(0, entry.clone());
                trim(&mut entries, max_entries);
                entry
            }
        };

        self.commit(&mut entries, previous)?;
        Ok(entry)
    }

    pub fn query(&self, query: &HistoryQuery) -> HistoryPage {
        let entries = self.entries.lock();
        let keyword = query
            .keyword
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| value.to_lowercase());
        let direction = query
            .direction
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());
        let provider = query
            .provider
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());

        let mut filtered: Vec<HistoryEntry> = entries
            .iter()
            .filter(|item| {
                if query.favorites_only && !item.favorite {
                    return false;
                }
                if let Some(value) = direction {
                    if item.direction != value {
                        return false;
                    }
                }
                if let Some(value) = provider {
                    if item.provider != value {
                        return false;
                    }
                }
                if let Some(value) = &keyword {
                    if !item.text.to_lowercase().contains(value)
                        && !item.result.to_lowercase().contains(value)
                    {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect();

        filtered.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

        let total = filtered.len() as u32;
        let limit = if query.limit == 0 {
            DEFAULT_LIMIT
        } else {
            query.limit
        } as usize;
        let offset = (query.offset as usize).min(filtered.len());
        let end = offset.saturating_add(limit).min(filtered.len());
        let entries = filtered[offset..end].to_vec();

        HistoryPage { entries, total }
    }

    pub fn delete(&self, id: &str) -> Result<(), String> {
        let mut entries = self.entries.lock();
        let previous = entries.clone();
        let index = entries
            .iter()
            .position(|item| item.id == id)
            .ok_or_else(|| "历史记录不存在".to_string())?;
        entries.remove(index);
        self.commit(&mut entries, previous)
    }

    pub fn clear(&self) -> Result<(), String> {
        let mut entries = self.entries.lock();
        let previous = std::mem::take(&mut *entries);
        self.commit(&mut entries, previous)
    }

    pub fn set_favorite(&self, id: &str, favorite: bool) -> Result<(), String> {
        let mut entries = self.entries.lock();
        let previous = entries.clone();
        let entry = entries
            .iter_mut()
            .find(|item| item.id == id)
            .ok_or_else(|| "历史记录不存在".to_string())?;
        entry.favorite = favorite;
        self.commit(&mut entries, previous)
    }

    /// Persist while the entries lock is held: writes are serialized and a
    /// failure rolls memory back so disk and memory never silently diverge.
    fn commit(
        &self,
        entries: &mut MutexGuard<'_, Vec<HistoryEntry>>,
        previous: Vec<HistoryEntry>,
    ) -> Result<(), String> {
        if let Err(err) = persist::save_json(&self.path, &**entries) {
            **entries = previous;
            return Err(err);
        }
        Ok(())
    }
}

fn trim(entries: &mut Vec<HistoryEntry>, max_entries: u32) {
    let max = max_entries.max(1) as usize;
    entries.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    let mut kept = 0usize;
    entries.retain(|item| {
        if item.favorite {
            true
        } else if kept < max {
            kept += 1;
            true
        } else {
            false
        }
    });
}

fn within_window(updated_at: &str) -> bool {
    let Ok(stamp) = NaiveDateTime::parse_from_str(updated_at, TIME_FORMAT) else {
        return false;
    };
    let diff = (Local::now().naive_local() - stamp).num_seconds();
    (0..=DEDUP_WINDOW_SECS).contains(&diff)
}

fn now_string() -> String {
    Local::now().format(TIME_FORMAT).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> HistoryStore {
        let path = std::env::temp_dir().join(format!("toolbox-history-{}.json", Uuid::new_v4()));
        HistoryStore::new(path).0
    }

    fn result(text: &str, provider: &str, source: &str) -> TranslateResult {
        TranslateResult {
            text: text.into(),
            provider: provider.into(),
            provider_name: provider.into(),
            source: source.into(),
            target: if source == "zh" { "en" } else { "zh" }.into(),
            latency_ms: 10,
        }
    }

    #[test]
    fn dedupes_within_five_minutes() {
        let store = store();
        store
            .add("你好", &result("hello", "microsoft", "zh"), 200)
            .unwrap();
        let updated = store
            .add("你好", &result("hi there", "microsoft", "zh"), 200)
            .unwrap();
        assert_eq!(updated.result, "hi there");
        assert_eq!(updated.direction, "zh2en");

        let page = store.query(&HistoryQuery::default());
        assert_eq!(page.total, 1);
        assert_eq!(page.entries[0].result, "hi there");

        // 不同服务不合并
        store
            .add("你好", &result("hi", "google", "zh"), 200)
            .unwrap();
        assert_eq!(store.query(&HistoryQuery::default()).total, 2);
    }

    #[test]
    fn favorites_do_not_consume_quota() {
        let store = store();
        let first = store
            .add("第一句", &result("first", "microsoft", "zh"), 2)
            .unwrap();
        store.set_favorite(&first.id, true).unwrap();
        store
            .add("第二句", &result("second", "microsoft", "zh"), 2)
            .unwrap();
        store
            .add("第三句", &result("third", "microsoft", "zh"), 2)
            .unwrap();

        let page = store.query(&HistoryQuery::default());
        assert_eq!(page.total, 3);
        let plain = page.entries.iter().filter(|item| !item.favorite).count();
        assert_eq!(plain, 2);
        assert!(page.entries.iter().any(|item| item.id == first.id));
    }

    #[test]
    fn filters_and_paginates() {
        let store = store();
        for index in 0..5 {
            let text = format!("文本{index}");
            let translation = format!("text {index}");
            let provider = if index % 2 == 0 { "microsoft" } else { "google" };
            store
                .add(&text, &result(&translation, provider, "zh"), 200)
                .unwrap();
        }

        let page = store.query(&HistoryQuery {
            limit: 2,
            offset: 1,
            ..Default::default()
        });
        assert_eq!(page.total, 5);
        assert_eq!(page.entries.len(), 2);

        let keyword = store.query(&HistoryQuery {
            keyword: Some("文本3".into()),
            ..Default::default()
        });
        assert_eq!(keyword.total, 1);
        assert_eq!(keyword.entries[0].text, "文本3");

        let provider = store.query(&HistoryQuery {
            provider: Some("google".into()),
            ..Default::default()
        });
        assert_eq!(provider.total, 2);

        let direction = store.query(&HistoryQuery {
            direction: Some("en2zh".into()),
            ..Default::default()
        });
        assert_eq!(direction.total, 0);

        let empty_limit = store.query(&HistoryQuery {
            limit: 0,
            ..Default::default()
        });
        assert_eq!(empty_limit.entries.len(), 5);
    }

    #[test]
    fn favorite_toggle_keeps_order_and_delete_errors() {
        let store = store();
        let entry = store
            .add("一句话", &result("a sentence", "microsoft", "zh"), 200)
            .unwrap();
        let before = store.query(&HistoryQuery::default());
        store.set_favorite(&entry.id, true).unwrap();
        let after = store.query(&HistoryQuery::default());
        assert_eq!(after.entries[0].updated_at, before.entries[0].updated_at);

        assert!(store.delete("missing-id").is_err());
        store.delete(&entry.id).unwrap();
        assert_eq!(store.query(&HistoryQuery::default()).total, 0);
    }
}
