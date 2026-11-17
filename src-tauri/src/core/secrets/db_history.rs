//! 数据库连接历史：复用加密凭据文件，列表只展示脱敏后的连接目标。

use crate::core::db::safe_history_label;
use crate::models::DbHistoryEntryView;

use super::{timestamp, SecretEntry, SecretStore, Secrets, MAX_SECRET_LEN};

const DB_ID_PREFIX: &str = "db:";
const DB_SERVICE_PREFIX: &str = "db:";
const MAX_DB_HISTORY: usize = 20;
const MAX_DB_NAME_LEN: usize = 60;

pub(super) fn is_db_entry(entry: &SecretEntry) -> bool {
    entry.id.starts_with(DB_ID_PREFIX)
}

/// 旧数据把脱敏目标存在 name 里，label 为空；读取时回退，写入时迁移。
fn entry_label(entry: &SecretEntry) -> String {
    if entry.label.trim().is_empty() {
        entry.name.clone()
    } else {
        entry.label.clone()
    }
}

impl SecretStore {
    pub fn record_db_connection(&self, kind: &str, connection: &str) -> Result<(), String> {
        let kind = kind.trim().to_ascii_lowercase();
        if !matches!(kind.as_str(), "sqlite" | "postgres" | "mysql") {
            return Err("[PARAM] 不支持的数据库类型".into());
        }
        let connection = connection.trim();
        if connection.is_empty() {
            return Err("[PARAM] 连接信息为空".into());
        }
        if connection.chars().count() > MAX_SECRET_LEN {
            return Err("[PARAM] 连接信息过长".into());
        }
        let label = safe_history_label(&kind, connection);
        if label.is_empty() {
            return Err("[PARAM] 无法识别连接目标".into());
        }

        let service = format!("{DB_SERVICE_PREFIX}{kind}");
        let mut secrets = self.secrets.lock();
        let previous = secrets.clone();
        let now = timestamp();
        let existing = secrets.entries.iter().position(|entry| {
            is_db_entry(entry) && entry.service == service && entry_label(entry) == label
        });
        match existing {
            Some(index) => {
                let entry = &mut secrets.entries[index];
                if entry.label.trim().is_empty() {
                    entry.label = label;
                }
                entry.secret = connection.to_string();
                entry.updated_at = now;
            }
            None => {
                secrets.entries.push(SecretEntry {
                    id: format!("{DB_ID_PREFIX}{}", uuid::Uuid::new_v4()),
                    name: label.clone(),
                    label,
                    service,
                    secret: connection.to_string(),
                    note: String::new(),
                    expires_on: String::new(),
                    created_at: now.clone(),
                    updated_at: now,
                });
            }
        }
        prune(&mut secrets);
        self.persist(&mut secrets, previous)
    }

    pub fn list_db_connections(&self) -> Vec<DbHistoryEntryView> {
        let secrets = self.secrets.lock();
        let mut views = secrets
            .entries
            .iter()
            .filter(|entry| is_db_entry(entry))
            .map(|entry| DbHistoryEntryView {
                id: entry.id.clone(),
                kind: entry
                    .service
                    .strip_prefix(DB_SERVICE_PREFIX)
                    .unwrap_or_default()
                    .to_string(),
                label: entry_label(entry),
                name: entry.name.clone(),
                updated_at: entry.updated_at.clone(),
            })
            .collect::<Vec<_>>();
        views.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        views
    }

    /// 自定义记录名称；脱敏连接目标单独保存，重命名不会影响去重。
    pub fn rename_db_connection(&self, id: &str, name: &str) -> Result<(), String> {
        let name = name.trim();
        if name.is_empty() {
            return Err("[PARAM] 请输入记录名称".into());
        }
        if name.chars().count() > MAX_DB_NAME_LEN {
            return Err("[PARAM] 记录名称过长".into());
        }
        let mut secrets = self.secrets.lock();
        let previous = secrets.clone();
        let Some(entry) = secrets
            .entries
            .iter_mut()
            .find(|entry| entry.id == id && is_db_entry(entry))
        else {
            return Err("[DATA] 历史连接不存在".into());
        };
        if entry.name == name && !entry.label.trim().is_empty() {
            return Ok(());
        }
        if entry.label.trim().is_empty() {
            entry.label = entry.name.clone();
        }
        entry.name = name.to_string();
        self.persist(&mut secrets, previous)
    }

    pub fn reveal_db_connection(&self, id: &str) -> Result<String, String> {
        let secrets = self.secrets.lock();
        secrets
            .entries
            .iter()
            .find(|entry| entry.id == id && is_db_entry(entry))
            .map(|entry| entry.secret.clone())
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| "[DATA] 历史连接不存在".into())
    }
}

fn prune(secrets: &mut Secrets) {
    let mut items = secrets
        .entries
        .iter()
        .enumerate()
        .filter(|(_, entry)| is_db_entry(entry))
        .map(|(index, entry)| (index, entry.updated_at.clone()))
        .collect::<Vec<_>>();
    if items.len() <= MAX_DB_HISTORY {
        return;
    }
    items.sort_by(|a, b| b.1.cmp(&a.1));
    let mut remove = items
        .into_iter()
        .skip(MAX_DB_HISTORY)
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    remove.sort_unstable_by(|a, b| b.cmp(a));
    for index in remove {
        secrets.entries.remove(index);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_path() -> PathBuf {
        std::env::temp_dir().join(format!("toolbox-db-history-{}.bin", uuid::Uuid::new_v4()))
    }

    #[test]
    fn db_connections_are_deduped_and_hidden_from_secret_list() {
        let path = temp_path();
        let (store, warning) = SecretStore::load(path.clone());
        assert!(warning.is_none());

        store
            .record_db_connection("mysql", "mysql://root:old@db:3306/app")
            .unwrap();
        store
            .record_db_connection("mysql", "mysql://root:new@db:3306/app")
            .unwrap();

        assert!(store.list().is_empty());
        let history = store.list_db_connections();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].kind, "mysql");
        assert_eq!(history[0].label, "mysql://root@db:3306/app");
        assert_eq!(
            store.reveal_db_connection(&history[0].id).unwrap(),
            "mysql://root:new@db:3306/app"
        );

        let (reloaded, warning) = SecretStore::load(path.clone());
        assert!(warning.is_none());
        assert_eq!(reloaded.list_db_connections().len(), 1);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn rename_keeps_sanitized_label_and_dedupe() {
        let path = temp_path();
        let (store, warning) = SecretStore::load(path.clone());
        assert!(warning.is_none());
        store
            .record_db_connection("mysql", "mysql://root:old@db:3306/app")
            .unwrap();
        let id = store.list_db_connections()[0].id.clone();

        store.rename_db_connection(&id, "  生产库  ").unwrap();
        let history = store.list_db_connections();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].name, "生产库");
        assert_eq!(history[0].label, "mysql://root@db:3306/app");

        store
            .record_db_connection("mysql", "mysql://root:new@db:3306/app")
            .unwrap();
        let history = store.list_db_connections();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].name, "生产库");
        assert_eq!(
            store.reveal_db_connection(&id).unwrap(),
            "mysql://root:new@db:3306/app"
        );

        let (reloaded, warning) = SecretStore::load(path.clone());
        assert!(warning.is_none());
        assert_eq!(reloaded.list_db_connections()[0].name, "生产库");
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn legacy_entry_without_label_is_migrated_on_rename() {
        let path = temp_path();
        let db_file = temp_path();
        let db_file = db_file.to_string_lossy().to_string();
        let (store, warning) = SecretStore::load(path.clone());
        assert!(warning.is_none());
        store.record_db_connection("sqlite", &db_file).unwrap();
        {
            let mut secrets = store.secrets.lock();
            let entry = &mut secrets.entries[0];
            entry.name = db_file.clone();
            entry.label.clear();
        }
        let history = store.list_db_connections();
        assert_eq!(history[0].name, db_file);
        assert_eq!(history[0].label, db_file);

        store
            .rename_db_connection(&history[0].id, "本地测试")
            .unwrap();
        let history = store.list_db_connections();
        assert_eq!(history[0].name, "本地测试");
        assert_eq!(history[0].label, db_file);
        assert!(store.rename_db_connection(&history[0].id, " ").is_err());
        let _ = std::fs::remove_file(path);
    }
}
