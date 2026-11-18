//! 凭据存储：可扩展条目列表 + 旧版固定字段迁移。
//!
//! Windows 下文件经 DPAPI 保护并以 hex 编码；macOS / Linux 目前只靠
//! 目录 0700 与 `secrets.bin` 0600 的文件权限，内容等同明文，见 `codec`。

use std::path::PathBuf;

use chrono::{Local, SecondsFormat};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use crate::core::{config, persist};
use crate::models::AppConfig;

mod codec;
mod db_history;
mod status;
#[cfg(test)]
mod tests;

pub use status::SecretStatus;

pub const BUILTIN_OPENAI: &str = "translate.openai";
pub const BUILTIN_DEEPL: &str = "translate.deepl";
pub const BUILTIN_BAIDU: &str = "translate.baidu";
pub const BUILTIN_YOUDAO: &str = "translate.youdao";

const MAX_SECRET_LEN: usize = 20_000;

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct SecretEntry {
    pub id: String,
    pub name: String,
    /// 脱敏后的连接目标，数据库历史专用；普通凭据为空。
    pub label: String,
    pub service: String,
    pub secret: String,
    pub note: String,
    pub expires_on: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Secrets {
    pub version: u32,
    pub entries: Vec<SecretEntry>,
}

impl Default for Secrets {
    fn default() -> Self {
        Self {
            version: 2,
            entries: Vec::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub(super) struct SecretsV1 {
    pub openai_api_key: String,
    pub deepl_api_key: String,
    pub baidu_secret_key: String,
    pub youdao_app_secret: String,
}

/// Write-only update for the four built-in translation keys. `None` keeps the
/// current value, `Some("")` clears it, any other value replaces it.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct SecretPatch {
    pub openai_api_key: Option<String>,
    pub deepl_api_key: Option<String>,
    pub baidu_secret_key: Option<String>,
    pub youdao_app_secret: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct SecretEntryInput {
    pub id: String,
    pub name: String,
    pub service: String,
    /// 留空表示保持原值（编辑时）；新建必须提供。
    pub secret: String,
    pub note: String,
    pub expires_on: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SecretEntryView {
    pub id: String,
    pub name: String,
    pub service: String,
    pub note: String,
    pub expires_on: String,
    pub created_at: String,
    pub updated_at: String,
    pub builtin: bool,
    pub masked: String,
    pub days_left: Option<i64>,
    pub expired: bool,
}

pub struct SecretStore {
    path: PathBuf,
    /// 与 `secrets.bin` 同级的 config.json，用于迁移后立即写回已清空的配置。
    config_path: PathBuf,
    secrets: Mutex<Secrets>,
    /// 一次性提示（迁移 / 权限收紧），由首次 `get_app_snapshot` 读取。
    notice: Mutex<Option<String>>,
}

impl SecretStore {
    pub fn load(path: PathBuf) -> (Self, Option<String>) {
        let mut warning = None;
        let secrets = match persist::load_bytes(&path) {
            Ok(Some(bytes)) => match codec::decode(&bytes) {
                Ok(codec::Decoded::Current(secrets)) => secrets,
                Ok(codec::Decoded::Legacy(old)) => {
                    let migrated = Secrets::from_legacy(old);
                    if let Err(err) = codec::encode(&migrated)
                        .and_then(|encoded| persist::write_private_bytes(&path, &encoded))
                    {
                        warning = Some(format!("密钥文件升级失败（{err}），本次运行使用内存中的新格式"));
                    }
                    migrated
                }
                Err(err) => {
                    warning = Some(match persist::backup_corrupt(&path) {
                        Some(backup) => format!(
                            "读取密钥文件失败（{err}），原文件已备份为 {}",
                            backup.display()
                        ),
                        None => format!("读取密钥文件失败（{err}）"),
                    });
                    Secrets::default()
                }
            },
            Ok(None) => Secrets::default(),
            Err(err) => {
                warning = Some(err);
                Secrets::default()
            }
        };
        // 旧版本可能留下 0644 的 secrets.bin：读取时统一收紧为 0600。
        let notice = if persist::restrict_private(&path) {
            Some("已收紧密钥文件权限为 0600（仅当前用户可读写）".to_string())
        } else {
            None
        };
        (
            Self {
                config_path: path.with_file_name("config.json"),
                path,
                secrets: Mutex::new(secrets),
                notice: Mutex::new(notice),
            },
            warning,
        )
    }

    pub fn status(&self) -> SecretStatus {
        let secrets = self.secrets.lock();
        SecretStatus {
            openai: has_secret(&secrets, BUILTIN_OPENAI),
            deepl: has_secret(&secrets, BUILTIN_DEEPL),
            baidu: has_secret(&secrets, BUILTIN_BAIDU),
            youdao: has_secret(&secrets, BUILTIN_YOUDAO),
            protection: SecretStatus::protection_level().to_string(),
        }
    }

    /// 读取并清除一次性的密钥迁移 / 权限提示（启动后首次快照展示）。
    pub fn take_migration_notice(&self) -> Option<String> {
        self.notice.lock().take()
    }

    pub fn secret_for(&self, id: &str) -> Option<String> {
        let secrets = self.secrets.lock();
        secrets
            .entry(id)
            .map(|entry| entry.secret.clone())
            .filter(|value| !value.trim().is_empty())
    }

    pub fn list(&self) -> Vec<SecretEntryView> {
        let secrets = self.secrets.lock();
        let mut views: Vec<SecretEntryView> = secrets
            .entries
            .iter()
            .filter(|entry| {
                !entry.secret.trim().is_empty() && !db_history::is_db_entry(entry)
            })
            .map(|entry| {
                let (expired, days_left) = status::expiry(&entry.expires_on);
                SecretEntryView {
                    id: entry.id.clone(),
                    name: entry.name.clone(),
                    service: entry.service.clone(),
                    note: entry.note.clone(),
                    expires_on: entry.expires_on.clone(),
                    created_at: entry.created_at.clone(),
                    updated_at: entry.updated_at.clone(),
                    builtin: builtin_meta(&entry.id).is_some(),
                    masked: mask_secret(&entry.secret),
                    days_left,
                    expired,
                }
            })
            .collect();
        views.sort_by(|a, b| {
            let rank = |item: &SecretEntryView| builtin_rank(&item.id).unwrap_or(99);
            rank(a).cmp(&rank(b)).then_with(|| a.name.cmp(&b.name))
        });
        views
    }

    pub fn save_entry(&self, input: SecretEntryInput) -> Result<(), String> {
        let mut secrets = self.secrets.lock();
        let previous = secrets.clone();
        let now = timestamp();
        let id = input.id.trim().to_string();
        let name = input.name.trim().to_string();
        let service = input.service.trim().to_string();
        let secret = input.secret.trim().to_string();
        let note = input.note.trim().to_string();
        let expires_on = input.expires_on.trim().to_string();

        status::validate_expiry(&expires_on)?;
        if name.is_empty() {
            return Err("[PARAM] 请输入凭据名称".into());
        }
        if name.chars().count() > 60 || note.chars().count() > 200 || service.chars().count() > 40 {
            return Err("[PARAM] 凭据名称、服务或备注过长".into());
        }
        if secret.chars().count() > MAX_SECRET_LEN {
            return Err("[PARAM] 密钥内容过长".into());
        }

        let index = secrets.entries.iter().position(|entry| entry.id == id);
        match index {
            None if !id.is_empty() => return Err("[DATA] 凭据不存在，可能已被删除".into()),
            None => {
                if secret.is_empty() {
                    return Err("[PARAM] 请输入密钥内容".into());
                }
                if service.is_empty() {
                    return Err("[PARAM] 请输入所属服务".into());
                }
                let new_id = uuid::Uuid::new_v4().to_string();
                secrets.entries.push(SecretEntry {
                    id: new_id,
                    name,
                    label: String::new(),
                    service,
                    secret,
                    note,
                    expires_on,
                    created_at: now.clone(),
                    updated_at: now,
                });
            }
            Some(index) => {
                let builtin = builtin_meta(&id).map(|(_, service)| service.to_string());
                let entry = &mut secrets.entries[index];
                if !secret.is_empty() {
                    entry.secret = secret;
                }
                if entry.secret.trim().is_empty() {
                    return Err("[PARAM] 请输入密钥内容".into());
                }
                entry.name = name;
                entry.service = match builtin {
                    Some(service) => service,
                    None if service.is_empty() => {
                        return Err("[PARAM] 请输入所属服务".into());
                    }
                    None => service,
                };
                entry.note = note;
                entry.expires_on = expires_on;
                entry.updated_at = now;
            }
        }

        self.persist(&mut secrets, previous)
    }

    pub fn delete_entry(&self, id: &str) -> Result<(), String> {
        let mut secrets = self.secrets.lock();
        let previous = secrets.clone();
        let before = secrets.entries.len();
        secrets.entries.retain(|entry| entry.id != id);
        if secrets.entries.len() == before {
            return Ok(());
        }
        self.persist(&mut secrets, previous)
    }

    pub fn reveal(&self, id: &str) -> Result<String, String> {
        let secrets = self.secrets.lock();
        secrets
            .entry(id)
            .map(|entry| entry.secret.clone())
            .filter(|value| !value.is_empty())
            .ok_or_else(|| "[DATA] 凭据不存在或内容为空".to_string())
    }

    /// 把密钥库中的内置翻译密钥填充到内存配置（`config.json` 永远不保存密钥）。
    pub fn apply_to(&self, config: &mut AppConfig) {
        let secrets = self.secrets.lock();
        config.translate.openai.api_key = secrets.secret_of(BUILTIN_OPENAI);
        config.translate.deepl.api_key = secrets.secret_of(BUILTIN_DEEPL);
        config.translate.baidu.secret_key = secrets.secret_of(BUILTIN_BAIDU);
        config.translate.youdao.app_secret = secrets.secret_of(BUILTIN_YOUDAO);
    }

    /// Move keys embedded in an older config.json into the secret store,
    /// immediately rewrite the cleared config back to disk, and record a
    /// one-shot notice for the first snapshot. Returns whether anything migrated.
    pub fn migrate_inline(&self, config: &mut AppConfig) -> Result<bool, String> {
        let patch = SecretPatch {
            openai_api_key: take_secret(&mut config.translate.openai.api_key),
            deepl_api_key: take_secret(&mut config.translate.deepl.api_key),
            baidu_secret_key: take_secret(&mut config.translate.baidu.secret_key),
            youdao_app_secret: take_secret(&mut config.translate.youdao.app_secret),
        };
        let changed = !patch_is_empty(&patch);
        if !changed {
            return Ok(false);
        }
        self.apply_patch(&patch)?;

        // 密钥已安全落库；立即原子写回不含密钥的 config.json，
        // 避免明文继续留在磁盘上等下一次保存。
        let mut stored = config.clone();
        config::clear_secrets(&mut stored);
        let notice = match persist::save_json(&self.config_path, &stored) {
            Ok(()) => "已将 config.json 中的内联密钥迁移到密钥文件，并从配置文件清除".to_string(),
            Err(err) => format!(
                "内联密钥已迁移到密钥文件，但清理 config.json 失败（{err}），请手动删除配置中的密钥"
            ),
        };
        *self.notice.lock() = Some(notice);
        Ok(true)
    }

    pub fn apply_patch(&self, patch: &SecretPatch) -> Result<(), String> {
        let mut secrets = self.secrets.lock();
        let previous = secrets.clone();
        let entries = [
            (BUILTIN_OPENAI, patch.openai_api_key.as_deref()),
            (BUILTIN_DEEPL, patch.deepl_api_key.as_deref()),
            (BUILTIN_BAIDU, patch.baidu_secret_key.as_deref()),
            (BUILTIN_YOUDAO, patch.youdao_app_secret.as_deref()),
        ];
        for (id, value) in entries {
            set_builtin(&mut secrets, id, value);
        }
        self.persist(&mut secrets, previous)
    }

    fn persist(&self, secrets: &mut Secrets, previous: Secrets) -> Result<(), String> {
        if *secrets == previous {
            return Ok(());
        }
        let encoded = match codec::encode(secrets) {
            Ok(encoded) => encoded,
            Err(err) => {
                *secrets = previous;
                return Err(format!("[DATA] 保存密钥失败: {err}"));
            }
        };
        if let Err(err) = persist::write_private_bytes(&self.path, &encoded) {
            *secrets = previous;
            return Err(format!("[DATA] 保存密钥失败: {err}"));
        }
        Ok(())
    }
}

impl Secrets {
    fn entry(&self, id: &str) -> Option<&SecretEntry> {
        self.entries.iter().find(|entry| entry.id == id)
    }

    fn secret_of(&self, id: &str) -> String {
        self.entry(id).map(|entry| entry.secret.clone()).unwrap_or_default()
    }

    pub(super) fn from_legacy(old: SecretsV1) -> Self {
        let mut secrets = Secrets::default();
        let legacy = [
            (BUILTIN_OPENAI, old.openai_api_key),
            (BUILTIN_DEEPL, old.deepl_api_key),
            (BUILTIN_BAIDU, old.baidu_secret_key),
            (BUILTIN_YOUDAO, old.youdao_app_secret),
        ];
        for (id, value) in legacy {
            set_builtin(&mut secrets, id, Some(&value));
        }
        secrets
    }
}

pub fn mask_config(config: &mut AppConfig) {
    config::clear_secrets(config);
}

fn set_builtin(secrets: &mut Secrets, id: &str, value: Option<&str>) {
    let Some(value) = value else { return };
    let value = value.trim();
    let now = timestamp();
    match secrets.entries.iter().position(|entry| entry.id == id) {
        Some(index) if value.is_empty() => {
            secrets.entries.remove(index);
        }
        Some(index) => {
            let entry = &mut secrets.entries[index];
            if entry.secret != value {
                entry.secret = value.to_string();
                entry.updated_at = now;
            }
        }
        None if !value.is_empty() => {
            let (name, service) = builtin_meta(id).unwrap_or(("未命名", "custom"));
            secrets.entries.push(SecretEntry {
                id: id.to_string(),
                name: name.to_string(),
                label: String::new(),
                service: service.to_string(),
                secret: value.to_string(),
                note: String::new(),
                expires_on: String::new(),
                created_at: now.clone(),
                updated_at: now,
            });
        }
        None => {}
    }
}

fn has_secret(secrets: &Secrets, id: &str) -> bool {
    secrets
        .entry(id)
        .map(|entry| !entry.secret.trim().is_empty())
        .unwrap_or(false)
}

pub(super) fn builtin_meta(id: &str) -> Option<(&'static str, &'static str)> {
    match id {
        BUILTIN_OPENAI => Some(("OpenAI", "openai")),
        BUILTIN_DEEPL => Some(("DeepL", "deepl")),
        BUILTIN_BAIDU => Some(("百度翻译", "baidu")),
        BUILTIN_YOUDAO => Some(("有道翻译", "youdao")),
        _ => None,
    }
}

fn builtin_rank(id: &str) -> Option<usize> {
    [BUILTIN_OPENAI, BUILTIN_DEEPL, BUILTIN_BAIDU, BUILTIN_YOUDAO]
        .iter()
        .position(|item| *item == id)
}

fn mask_secret(secret: &str) -> String {
    let chars: Vec<char> = secret.chars().collect();
    if chars.len() <= 8 {
        return "••••••".into();
    }
    let head: String = chars[..3].iter().collect();
    let tail: String = chars[chars.len() - 4..].iter().collect();
    format!("{head}…{tail}")
}

fn take_secret(value: &mut String) -> Option<String> {
    let taken = value.trim().to_string();
    value.clear();
    if taken.is_empty() {
        None
    } else {
        Some(taken)
    }
}

fn patch_is_empty(patch: &SecretPatch) -> bool {
    patch.openai_api_key.is_none()
        && patch.deepl_api_key.is_none()
        && patch.baidu_secret_key.is_none()
        && patch.youdao_app_secret.is_none()
}

fn timestamp() -> String {
    Local::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}
