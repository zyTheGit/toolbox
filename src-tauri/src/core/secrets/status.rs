//! 密钥状态摘要与过期提示。
//!
//! `protection` 只描述平台真实提供的保护级别，不做夸大：
//! - `dpapi`：Windows 由系统 DPAPI 绑定当前用户；
//! - `file-permissions`：macOS / Linux 当前仅靠 0700 / 0600 文件权限，内容等同明文；
//! - `unknown`：其它平台，未确认。

use chrono::{Local, NaiveDate};
use serde::{Deserialize, Serialize};

use super::{codec, SecretStore};

const EXPIRING_DAYS: i64 = 7;

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SecretStatus {
    pub openai: bool,
    pub deepl: bool,
    pub baidu: bool,
    pub youdao: bool,
    /// 平台保护级别：`dpapi` / `file-permissions` / `unknown`。
    pub protection: String,
}

impl SecretStatus {
    pub fn protection_level() -> &'static str {
        codec::protection()
    }
}

impl SecretStore {
    /// 临近或已经过期的凭据提示，供启动时展示。
    pub fn expiring_warnings(&self) -> Vec<String> {
        self.list()
            .into_iter()
            .filter_map(|entry| {
                let days = entry.days_left?;
                if days > EXPIRING_DAYS {
                    return None;
                }
                let when = if days < 0 {
                    format!("已过期 {} 天", -days)
                } else if days == 0 {
                    "今天到期".to_string()
                } else {
                    format!("{days} 天后到期")
                };
                Some(format!("凭据「{}」{when}，请及时更新", entry.name))
            })
            .collect()
    }
}

pub(super) fn validate_expiry(value: &str) -> Result<(), String> {
    if value.is_empty() || NaiveDate::parse_from_str(value, "%Y-%m-%d").is_ok() {
        Ok(())
    } else {
        Err("[PARAM] 过期日期格式应为 YYYY-MM-DD".into())
    }
}

pub(super) fn expiry(expires_on: &str) -> (bool, Option<i64>) {
    let Ok(date) = NaiveDate::parse_from_str(expires_on.trim(), "%Y-%m-%d") else {
        return (false, None);
    };
    let days = (date - Local::now().date_naive()).num_days();
    (days < 0, Some(days))
}
