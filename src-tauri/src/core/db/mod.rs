//! 数据库会话管理：每个打开的连接对应一个只读会话。

pub mod mysql;
pub mod postgres;
pub mod sql;
pub mod sqlite;

use std::collections::HashMap;

use parking_lot::Mutex;

use crate::models::{DbQueryResult, DbSchema};
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

    pub fn label(&self) -> &str {
        match self {
            Session::Sqlite(session) => session.label(),
            Session::Postgres(session) => session.label(),
            Session::Mysql(session) => session.label(),
        }
    }

    pub fn schema(&self) -> &DbSchema {
        match self {
            Session::Sqlite(session) => session.schema(),
            Session::Postgres(session) => session.schema(),
            Session::Mysql(session) => session.schema(),
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

pub struct DbManager {
    sessions: Mutex<HashMap<String, Session>>,
}

impl DbManager {
    pub fn new() -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
        }
    }

    pub fn add(&self, session: Session) -> String {
        let id = uuid::Uuid::new_v4().to_string();
        self.sessions.lock().insert(id.clone(), session);
        id
    }

    pub fn get(&self, id: &str) -> Result<Session, String> {
        self.sessions
            .lock()
            .get(id)
            .cloned()
            .ok_or_else(|| "[PARAM] 数据库会话不存在，请重新连接".to_string())
    }

    pub fn remove(&self, id: &str) {
        self.sessions.lock().remove(id);
    }
}
