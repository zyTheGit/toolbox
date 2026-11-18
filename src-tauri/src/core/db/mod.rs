//! 数据库会话管理：每个打开的连接对应一个只读会话。

pub mod mysql;
pub mod postgres;
pub mod sessions;
pub mod sql;
pub mod sqlite;

pub use sessions::DbManager;

use crate::models::{DbDatabase, DbQueryResult, DbSchema};
use mysql::MysqlSession;
use postgres::PostgresSession;
use sqlite::SqliteSession;

#[derive(Clone)]
pub enum Session {
    Sqlite(SqliteSession),
    Postgres(PostgresSession),
    Mysql(MysqlSession),
}

impl Session {
    pub fn kind(&self) -> &'static str {
        match self {
            Session::Sqlite(session) => session.kind(),
            Session::Postgres(session) => session.kind(),
            Session::Mysql(session) => session.kind(),
        }
    }

    pub fn label(&self) -> String {
        match self {
            Session::Sqlite(session) => session.label().to_string(),
            Session::Postgres(session) => session.label().to_string(),
            Session::Mysql(session) => session.label(),
        }
    }

    pub fn schema(&self) -> DbSchema {
        match self {
            Session::Sqlite(session) => session.schema().clone(),
            Session::Postgres(session) => session.schema().clone(),
            Session::Mysql(session) => session.schema(),
        }
    }

    pub fn databases(&self) -> Vec<DbDatabase> {
        match self {
            Session::Sqlite(_) | Session::Postgres(_) => Vec::new(),
            Session::Mysql(session) => session.databases(),
        }
    }

    pub async fn select_database(&self, database: &str) -> Result<DbSchema, String> {
        match self {
            Session::Mysql(session) => session.select_database(database).await,
            _ => Err("[PARAM] 只有 MySQL 支持切换数据库".into()),
        }
    }

    pub async fn query(
        &self,
        raw: &str,
        offset: usize,
        limit: usize,
    ) -> Result<DbQueryResult, String> {
        match self {
            Session::Sqlite(session) => {
                let session = session.clone();
                let raw = raw.to_string();
                tauri::async_runtime::spawn_blocking(move || session.query(&raw, offset, limit))
                    .await
                    .map_err(|e| format!("[DB] 查询线程异常: {e}"))?
            }
            Session::Postgres(session) => session.query(raw, offset, limit).await,
            Session::Mysql(session) => session.query(raw, offset, limit).await,
        }
    }
}

/// 历史记录列表展示用标签：保留能识别目标的部分，去掉密码等凭据。
pub fn safe_history_label(kind: &str, raw: &str) -> String {
    let text = raw.trim();
    let label = match kind {
        "sqlite" => text.to_string(),
        "mysql" | "postgres" => sanitize_remote(text),
        _ => String::new(),
    };
    truncate_chars(&label, 160)
}

fn sanitize_remote(raw: &str) -> String {
    let Some((scheme, rest)) = raw.split_once("://") else {
        return strip_keyword_secrets(raw);
    };
    let Some(at) = rest.rfind('@') else {
        return raw.to_string();
    };
    let user = rest[..at].split(':').next().unwrap_or("").trim();
    let host = strip_url_secrets(&rest[at + 1..]);
    if user.is_empty() {
        format!("{scheme}://{host}")
    } else {
        format!("{scheme}://{user}@{host}")
    }
}

fn strip_keyword_secrets(raw: &str) -> String {
    let mut kept = Vec::new();
    let mut skip_next = false;
    for token in raw.split_whitespace() {
        if skip_next {
            skip_next = false;
            continue;
        }
        let lower = token.to_ascii_lowercase();
        if matches!(lower.as_str(), "password" | "passwd" | "pwd") {
            skip_next = true;
            continue;
        }
        if lower.starts_with("password=")
            || lower.starts_with("passwd=")
            || lower.starts_with("pwd=")
        {
            continue;
        }
        kept.push(token);
    }
    kept.join(" ")
}

fn strip_url_secrets(rest: &str) -> String {
    let Some((base, query)) = rest.split_once('?') else {
        return rest.to_string();
    };
    let kept = query
        .split('&')
        .filter(|part| {
            let key = part.split('=').next().unwrap_or("").to_ascii_lowercase();
            !matches!(key.as_str(), "password" | "passwd" | "pwd")
        })
        .collect::<Vec<_>>();
    if kept.is_empty() {
        base.to_string()
    } else {
        format!("{base}?{}", kept.join("&"))
    }
}

fn truncate_chars(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    let mut out: String = text.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}

#[cfg(test)]
mod tests {
    use super::safe_history_label;

    #[test]
    fn history_labels_strip_credentials() {
        assert_eq!(
            safe_history_label(
                "mysql",
                "mysql://root:qkS*0@&iKvQOHRISxoNW@192.168.0.207:3306/mysql"
            ),
            "mysql://root@192.168.0.207:3306/mysql"
        );
        assert_eq!(
            safe_history_label(
                "postgres",
                "postgres://me:secret@db:5432/app?sslmode=require&password=extra"
            ),
            "postgres://me@db:5432/app?sslmode=require"
        );
        assert_eq!(
            safe_history_label(
                "postgres",
                "host=db user=me password=secret dbname=app"
            ),
            "host=db user=me dbname=app"
        );
        assert_eq!(
            safe_history_label("sqlite", "C:/data/app.db"),
            "C:/data/app.db"
        );
    }
}
