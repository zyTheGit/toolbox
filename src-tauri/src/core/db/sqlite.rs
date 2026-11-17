//! SQLite 只读会话：打开时即开启只读标志，所有写操作由驱动直接拒绝。

use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use parking_lot::Mutex;
use rusqlite::types::ValueRef;
use rusqlite::{Connection, OpenFlags};
use serde_json::{json, Value};

use super::sql;
use crate::models::{DbColumn, DbQueryResult, DbSchema, DbTable};

#[derive(Clone)]
pub struct SqliteSession {
    label: String,
    schema: DbSchema,
    conn: Arc<Mutex<Connection>>,
}

impl SqliteSession {
    pub fn open(path: &str) -> Result<Self, String> {
        let file = Path::new(path);
        if !file.is_file() {
            return Err(format!("[PATH] 数据库文件不存在: {}", file.display()));
        }
        let conn = Connection::open_with_flags(
            file,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(|e| format!("[DB] 打开 SQLite 失败: {e}"))?;
        let schema = load_schema(&conn)?;
        Ok(Self {
            label: file
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_else(|| file.to_string_lossy().to_string()),
            schema,
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn kind(&self) -> &'static str {
        "sqlite"
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn schema(&self) -> &DbSchema {
        &self.schema
    }

    pub fn query(&self, raw: &str, offset: usize, limit: usize) -> Result<DbQueryResult, String> {
        let started = Instant::now();
        let statement = sql::single_statement(raw)?;
        let keyword = sql::first_keyword(&statement);
        let conn = self.conn.lock();

        let (sql_text, in_memory_page) = if sql::wrappable(&keyword) {
            (sql::wrap_page(&statement, limit, offset), false)
        } else {
            (statement, true)
        };
        let mut stmt = conn
            .prepare(&sql_text)
            .map_err(|e| format!("[DB] 无法执行 {}: {e}", sql::describe(&keyword)))?;
        let columns = stmt
            .column_names()
            .iter()
            .map(|name| name.to_string())
            .collect::<Vec<_>>();

        let mut rows = stmt
            .query([])
            .map_err(|e| format!("[DB] 查询失败: {e}"))?;
        let mut values: Vec<Vec<Value>> = Vec::new();
        let mut skipped = 0usize;
        while values.len() <= limit {
            let Some(row) = rows.next().map_err(|e| format!("[DB] 读取结果失败: {e}"))? else {
                break;
            };
            if in_memory_page && skipped < offset {
                skipped += 1;
                continue;
            }
            values.push(row_values(row)?);
        }

        let has_more = values.len() > limit;
        values.truncate(limit);
        Ok(DbQueryResult {
            columns,
            rows: values,
            offset,
            limit,
            has_more,
            elapsed_ms: started.elapsed().as_millis() as u64,
        })
    }
}

fn load_schema(conn: &Connection) -> Result<DbSchema, String> {
    let mut stmt = conn
        .prepare(
            "SELECT name, type FROM sqlite_master \
             WHERE type IN ('table','view') AND name NOT LIKE 'sqlite_%' ORDER BY name",
        )
        .map_err(|e| format!("[DB] 读取 SQLite 结构失败: {e}"))?;
    let items = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|e| format!("[DB] 读取 SQLite 结构失败: {e}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("[DB] 读取 SQLite 结构失败: {e}"))?;

    let mut tables = Vec::with_capacity(items.len());
    for (name, kind) in items {
        let mut pragma = conn
            .prepare(
                "SELECT name, type, \"notnull\", pk FROM pragma_table_info(?1) \
                 ORDER BY cid",
            )
            .map_err(|e| format!("[DB] 读取表 {name} 结构失败: {e}"))?;
        let columns = pragma
            .query_map(rusqlite::params![name], |row| {
                Ok(DbColumn {
                    name: row.get::<_, String>(0)?,
                    data_type: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                    nullable: row.get::<_, i64>(2)? == 0,
                    primary_key: row.get::<_, i64>(3)? > 0,
                })
            })
            .map_err(|e| format!("[DB] 读取表 {name} 结构失败: {e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("[DB] 读取表 {name} 结构失败: {e}"))?;
        tables.push(DbTable {
            name: name.clone(),
            schema: String::new(),
            kind,
            columns,
        });
    }
    Ok(DbSchema { tables })
}

fn row_values(row: &rusqlite::Row<'_>) -> Result<Vec<Value>, String> {
    (0..row.as_ref().column_count())
        .map(|index| {
            let value = row
                .get_ref(index)
                .map_err(|e| format!("[DB] 读取结果失败: {e}"))?;
            Ok(match value {
                ValueRef::Null => Value::Null,
                ValueRef::Integer(v) => json!(v),
                ValueRef::Real(v) if v.is_finite() => json!(v),
                ValueRef::Real(_) => Value::Null,
                ValueRef::Text(bytes) => json!(String::from_utf8_lossy(bytes)),
                ValueRef::Blob(bytes) => json!(format!("<BLOB {} bytes>", bytes.len())),
            })
        })
        .collect()
}
