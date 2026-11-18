//! SQLite 只读会话：打开时即开启只读标志，所有写操作由驱动直接拒绝。
//! 长查询由 `progress_handler` 看门狗在 15 秒后中断，中断后连接仍可继续使用。

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use rusqlite::types::ValueRef;
use rusqlite::{Connection, OpenFlags};
use serde_json::{json, Value};

use super::sql;
use crate::models::{DbColumn, DbQueryResult, DbSchema, DbTable};

/// 单条查询的最长执行时间；超时后由看门狗中断查询（不是 busy_timeout）。
const QUERY_TIMEOUT: Duration = Duration::from_secs(15);
/// 每执行多少条虚拟机指令检查一次是否超时。
const PROGRESS_STEPS: i32 = 10_000;

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
        self.query_with_timeout(raw, offset, limit, QUERY_TIMEOUT)
    }

    /// 带超时的查询：`progress_handler` 到期返回 true 让 SQLite 中断当前语句。
    fn query_with_timeout(
        &self,
        raw: &str,
        offset: usize,
        limit: usize,
        timeout: Duration,
    ) -> Result<DbQueryResult, String> {
        let started = Instant::now();
        let statement = sql::single_statement(raw)?;
        let keyword = sql::first_keyword(&statement);
        let conn = self.conn.lock();

        let deadline = started + timeout;
        let interrupted = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&interrupted);
        conn.progress_handler(PROGRESS_STEPS, Some(move || {
            if Instant::now() >= deadline {
                flag.store(true, Ordering::SeqCst);
                true
            } else {
                false
            }
        }));
        let outcome = run_query(&conn, &statement, &keyword, offset, limit, started);
        // 必须摘掉看门狗，否则旧截止时间会让后续查询立即被中断
        conn.progress_handler(0, None::<fn() -> bool>);

        match outcome {
            Err(_) if interrupted.load(Ordering::SeqCst) => Err(format!(
                "[TIMEOUT] SQLite 查询超过 {} 秒已中断，请缩小查询范围或改用分页",
                timeout.as_secs()
            )),
            other => other,
        }
    }
}

fn run_query(
    conn: &Connection,
    statement: &str,
    keyword: &str,
    offset: usize,
    limit: usize,
    started: Instant,
) -> Result<DbQueryResult, String> {
    let (sql_text, in_memory_page) = if sql::wrappable(keyword) {
        (sql::wrap_page(statement, limit, offset), false)
    } else {
        (statement.to_string(), true)
    };
    let mut stmt = conn
        .prepare(&sql_text)
        .map_err(|e| format!("[DB] 无法执行 {}: {e}", sql::describe(keyword)))?;
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

#[cfg(test)]
/// 测试用：建一个临时 SQLite 库并返回（目录, 会话）；用完先 drop 会话再删目录。
pub fn temp_session(tag: &str) -> (std::path::PathBuf, SqliteSession) {
    let dir = std::env::temp_dir().join(format!(
        "toolbox-db-{}-{}",
        tag,
        uuid::Uuid::new_v4()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("data.db");
    {
        let conn = Connection::open(&path).unwrap();
        conn.execute_batch("CREATE TABLE t (x INTEGER); INSERT INTO t VALUES (1), (2);")
            .unwrap();
    }
    let session = SqliteSession::open(path.to_str().unwrap()).unwrap();
    (dir, session)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_pages_and_reports_more_rows() {
        let (dir, session) = temp_session("page");
        let result = session.query("SELECT x FROM t ORDER BY x", 0, 1).unwrap();
        assert_eq!(result.rows.len(), 1);
        assert!(result.has_more);
        let result = session.query("SELECT x FROM t ORDER BY x", 1, 1).unwrap();
        assert_eq!(result.rows, vec![vec![json!(2)]]);
        assert!(!result.has_more);
        drop(session);
        std::fs::remove_dir_all(dir).ok();
    }

    /// 递归 CTE 无限展开，必须被看门狗在 15 秒左右中断，且中断后连接仍可用。
    #[test]
    fn recursive_cte_times_out_and_connection_still_usable() {
        let (dir, session) = temp_session("timeout");
        let started = Instant::now();
        let err = session
            .query(
                "SELECT count(*) FROM (WITH RECURSIVE c(x) AS \
                 (SELECT 1 UNION ALL SELECT x + 1 FROM c) SELECT x FROM c)",
                0,
                10,
            )
            .unwrap_err();
        assert!(err.starts_with("[TIMEOUT]"), "错误信息不符合预期: {err}");
        // 中断由 15 秒截止时间触发，允许少量调度误差
        assert!(
            started.elapsed() >= QUERY_TIMEOUT - Duration::from_secs(1),
            "查询过早结束: {:?}",
            started.elapsed()
        );
        assert!(started.elapsed() < Duration::from_secs(60));

        // 看门狗已摘除，连接可以继续执行短查询
        let result = session.query("SELECT x FROM t ORDER BY x", 0, 10).unwrap();
        assert_eq!(result.rows.len(), 2);
        assert!(!result.has_more);
        drop(session);
        std::fs::remove_dir_all(dir).ok();
    }
}
