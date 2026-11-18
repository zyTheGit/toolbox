//! MySQL 只读会话：连接后固定为只读事务，限制语句执行时间，并只放行查询类语句。

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Instant;

use mysql_async::prelude::Queryable;
use mysql_async::{params, Conn, Opts, Row, Value};
use parking_lot::RwLock;
use serde_json::{json, Value as JsonValue};
use tokio::sync::Mutex;

use super::sql;
use crate::models::{DbColumn, DbDatabase, DbQueryResult, DbSchema, DbTable};

#[derive(Clone)]
pub struct MysqlSession {
    label: Arc<RwLock<String>>,
    schema: Arc<RwLock<DbSchema>>,
    databases: Vec<DbDatabase>,
    conn: Arc<Mutex<Conn>>,
}

impl MysqlSession {
    pub async fn open(connection_string: &str) -> Result<Self, String> {
        let trimmed = connection_string.trim();
        if trimmed.is_empty() {
            return Err("[PARAM] 请填写 MySQL 连接串".into());
        }
        let opts = Opts::from_url(trimmed)
            .map_err(|e| format!("[PARAM] MySQL 连接串无效: {e}"))?;
        let explicit_db = opts
            .db_name()
            .filter(|name| !name.is_empty())
            .map(str::to_string);

        let mut conn = Conn::new(opts)
            .await
            .map_err(|e| format!("[DB] 连接 MySQL 失败: {e}"))?;
        conn.query_drop("SET SESSION TRANSACTION READ ONLY")
            .await
            .map_err(|e| format!("[DB] 初始化只读会话失败: {e}"))?;
        if conn
            .query_drop("SET SESSION max_execution_time = 15000")
            .await
            .is_err()
            && conn
                .query_drop("SET SESSION max_statement_time = 15")
                .await
                .is_err()
        {
            return Err("[DB] 初始化 MySQL 语句超时失败，请确认服务器支持超时设置".into());
        }

        let mut databases = list_databases(&mut conn).await?;
        if let Some(name) = &explicit_db {
            if !databases.iter().any(|db| db.name == *name) {
                databases.push(DbDatabase {
                    name: name.clone(),
                    table_count: 0,
                });
                databases.sort_by(|a, b| a.name.cmp(&b.name));
            }
        } else if databases.is_empty() {
            return Err("[DB] 未找到可浏览的数据库，请在连接串末尾指定库名".into());
        }
        // 未指定库名时默认打开第一个库，避免左侧结构为空
        let selected = explicit_db.unwrap_or_else(|| databases[0].name.clone());
        use_database(&mut conn, &selected).await?;
        let schema = load_schema(&mut conn, &selected).await?;
        let label = selected;
        if let Some(item) = databases.iter_mut().find(|db| db.name == label) {
            item.table_count = schema.tables.len();
        }
        Ok(Self {
            label: Arc::new(RwLock::new(label)),
            schema: Arc::new(RwLock::new(schema)),
            databases,
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn kind(&self) -> &'static str {
        "mysql"
    }

    pub fn label(&self) -> String {
        self.label.read().clone()
    }

    pub fn schema(&self) -> DbSchema {
        self.schema.read().clone()
    }

    pub fn databases(&self) -> Vec<DbDatabase> {
        self.databases.clone()
    }

    /// 主动断开连接：会话被淘汰、超时回收或关闭全部时调用，避免连接堆积。
    /// 仍有其它引用（例如并发查询正在使用）时不再强断，交给 Drop 关闭 socket。
    pub async fn shutdown(self) {
        let Some(mutex) = Arc::into_inner(self.conn) else {
            return;
        };
        if let Err(err) = mutex.into_inner().disconnect().await {
            eprintln!("[DB] 关闭 MySQL 会话失败: {err}");
        }
    }

    pub async fn select_database(&self, database: &str) -> Result<DbSchema, String> {
        let database = database.trim();
        if !self.databases.iter().any(|item| item.name == database) {
            return Err("[PARAM] 数据库不存在或不可见".into());
        }
        let mut conn = self.conn.lock().await;
        use_database(&mut conn, database).await?;
        let schema = load_schema(&mut conn, database).await?;
        *self.schema.write() = schema.clone();
        *self.label.write() = database.to_string();
        Ok(schema)
    }

    pub async fn query(
        &self,
        raw: &str,
        offset: usize,
        limit: usize,
    ) -> Result<DbQueryResult, String> {
        let started = Instant::now();
        let statement = sql::single_statement(raw)?;
        let keyword = sql::first_keyword(&statement);
        sql::ensure_read_only("MySQL", &keyword)?;

        let paged = sql::wrappable(&keyword);
        let text = if paged {
            sql::wrap_page(&statement, limit, offset)
        } else {
            statement
        };

        let mut conn = self.conn.lock().await;
        let mut result = conn
            .query_iter(&text)
            .await
            .map_err(|e| format!("[DB] 查询失败: {e}"))?;
        let columns = result
            .columns_ref()
            .iter()
            .map(|column| column.name_str().to_string())
            .collect::<Vec<_>>();
        let collected = result
            .collect::<Row>()
            .await
            .map_err(|e| format!("[DB] 读取结果失败: {e}"))?;
        drop(result);

        let (rows, has_more) = if paged {
            let has_more = collected.len() > limit;
            let rows = collected
                .iter()
                .take(limit)
                .map(row_values)
                .collect::<Vec<_>>();
            (rows, has_more)
        } else {
            let mut rows = collected
                .iter()
                .skip(offset)
                .take(limit + 1)
                .map(row_values)
                .collect::<Vec<_>>();
            let has_more = rows.len() > limit;
            rows.truncate(limit);
            (rows, has_more)
        };

        Ok(DbQueryResult {
            columns,
            rows,
            offset,
            limit,
            has_more,
            elapsed_ms: started.elapsed().as_millis() as u64,
        })
    }
}

async fn list_databases(conn: &mut Conn) -> Result<Vec<DbDatabase>, String> {
    let rows = conn
        .query::<Row, _>(
            "SELECT s.schema_name, COUNT(t.table_name) AS table_count \
             FROM information_schema.schemata s \
             LEFT JOIN information_schema.tables t \
               ON t.table_schema = s.schema_name AND t.table_type IN ('BASE TABLE','VIEW') \
             WHERE s.schema_name NOT IN ('information_schema','mysql','performance_schema','sys') \
             GROUP BY s.schema_name ORDER BY s.schema_name",
        )
        .await
        .map_err(|e| format!("[DB] 读取数据库列表失败: {e}"))?;
    Ok(rows
        .iter()
        .map(|row| DbDatabase {
            name: text(row, 0),
            table_count: count(row, 1),
        })
        .collect())
}

fn count(row: &Row, index: usize) -> usize {
    match row.as_ref(index) {
        Some(Value::UInt(value)) => *value as usize,
        Some(Value::Int(value)) => (*value).max(0) as usize,
        Some(Value::Bytes(bytes)) => std::str::from_utf8(bytes)
            .ok()
            .and_then(|text| text.parse().ok())
            .unwrap_or(0), // 某些代理把 COUNT 返回为文本
        _ => 0,
    }
}

async fn use_database(conn: &mut Conn, database: &str) -> Result<(), String> {
    let sql = format!("USE `{}`", database.replace('`', "``"));
    conn.query_drop(sql).await.map_err(|e| format!("[DB] 切换数据库 {database} 失败: {e}"))
}

// MySQL 的 schema 就是数据库，只加载当前选中的库，避免把服务器上所有库混在一起
async fn load_schema(conn: &mut Conn, db_name: &str) -> Result<DbSchema, String> {
    let tables = conn
        .exec::<Row, _, _>(
            "SELECT table_schema, table_name, table_type FROM information_schema.tables \
             WHERE table_schema = :schema AND table_type IN ('BASE TABLE','VIEW') \
             ORDER BY table_name",
            params! { "schema" => db_name },
        )
        .await
        .map_err(|e| format!("[DB] 读取表列表失败: {e}"))?;
    let columns = conn
        .exec::<Row, _, _>(
            "SELECT table_schema, table_name, column_name, column_type, is_nullable \
             FROM information_schema.columns \
             WHERE table_schema = :schema \
             ORDER BY table_name, ordinal_position",
            params! { "schema" => db_name },
        )
        .await
        .map_err(|e| format!("[DB] 读取列信息失败: {e}"))?;
    let pks = conn
        .exec::<Row, _, _>(
            "SELECT tc.table_schema, tc.table_name, kcu.column_name \
             FROM information_schema.table_constraints tc \
             JOIN information_schema.key_column_usage kcu \
               ON kcu.constraint_schema = tc.constraint_schema \
              AND kcu.constraint_name = tc.constraint_name \
              AND kcu.table_name = tc.table_name \
             WHERE tc.constraint_type = 'PRIMARY KEY' AND tc.table_schema = :schema",
            params! { "schema" => db_name },
        )
        .await
        .map_err(|e| format!("[DB] 读取主键信息失败: {e}"))?;

    let primary: HashSet<(String, String, String)> = pks
        .iter()
        .map(|row| (text(row, 0), text(row, 1), text(row, 2)))
        .collect();
    let mut grouped: HashMap<(String, String), Vec<DbColumn>> = HashMap::new();
    for row in &columns {
        let schema: String = text(row, 0);
        let table: String = text(row, 1);
        let name: String = text(row, 2);
        let data_type: String = text(row, 3);
        let nullable: String = text(row, 4);
        let primary_key = primary.contains(&(schema.clone(), table.clone(), name.clone()));
        grouped.entry((schema, table)).or_default().push(DbColumn {
            name,
            data_type,
            nullable: nullable == "YES",
            primary_key,
        });
    }

    let mut result = Vec::with_capacity(tables.len());
    for row in &tables {
        let schema: String = text(row, 0);
        let table: String = text(row, 1);
        let table_type: String = text(row, 2);
        let columns = grouped
            .remove(&(schema.clone(), table.clone()))
            .unwrap_or_default();
        result.push(DbTable {
            name: format!("{schema}.{table}"),
            schema,
            kind: if table_type == "VIEW" {
                "view".into()
            } else {
                "table".into()
            },
            columns,
        });
    }
    Ok(DbSchema { tables: result })
}

fn text(row: &Row, index: usize) -> String {
    row.get::<String, _>(index).unwrap_or_default()
}

fn row_values(row: &Row) -> Vec<JsonValue> {
    (0..row.len())
        .map(|index| {
            row.as_ref(index)
                .map(value_to_json)
                .unwrap_or(JsonValue::Null)
        })
        .collect()
}

fn value_to_json(value: &Value) -> JsonValue {
    match value {
        Value::NULL => JsonValue::Null,
        Value::Bytes(bytes) => match std::str::from_utf8(bytes) {
            Ok(text) => json!(text),
            Err(_) => json!(format!("<BLOB {} bytes>", bytes.len())),
        },
        Value::Int(v) => json!(v),
        Value::UInt(v) => json!(v),
        Value::Float(v) if v.is_finite() => json!(*v as f64),
        Value::Double(v) if v.is_finite() => json!(v),
        Value::Float(_) | Value::Double(_) => JsonValue::Null,
        Value::Date(year, month, day, hour, minute, second, micro) => {
            json!(format_date(*year, *month, *day, *hour, *minute, *second, *micro))
        }
        Value::Time(negative, days, hours, minutes, seconds, micro) => {
            json!(format_time(*negative, *days, *hours, *minutes, *seconds, *micro))
        }
    }
}

fn format_date(
    year: u16,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    second: u8,
    micro: u32,
) -> String {
    let date = format!("{year:04}-{month:02}-{day:02}");
    if hour == 0 && minute == 0 && second == 0 && micro == 0 {
        return date;
    }
    let mut text = format!("{date} {hour:02}:{minute:02}:{second:02}");
    if micro > 0 {
        text.push_str(&format!(".{micro:06}"));
    }
    text
}

fn format_time(negative: bool, days: u32, hours: u8, minutes: u8, seconds: u8, micro: u32) -> String {
    let total_hours = days as u64 * 24 + hours as u64;
    let sign = if negative { "-" } else { "" };
    let mut text = format!("{sign}{total_hours:02}:{minutes:02}:{seconds:02}");
    if micro > 0 {
        text.push_str(&format!(".{micro:06}"));
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_date_time_values() {
        assert_eq!(format_date(2026, 11, 17, 0, 0, 0, 0), "2026-11-17");
        assert_eq!(
            format_date(2026, 11, 17, 8, 5, 3, 12),
            "2026-11-17 08:05:03.000012"
        );
        assert_eq!(format_time(false, 1, 2, 3, 4, 0), "26:03:04");
        assert_eq!(format_time(true, 0, 1, 2, 3, 500), "-01:02:03.000500");
    }

    #[test]
    fn converts_common_values() {
        assert_eq!(value_to_json(&Value::NULL), JsonValue::Null);
        assert_eq!(value_to_json(&Value::Int(7)), json!(7));
        assert_eq!(value_to_json(&Value::UInt(8)), json!(8));
        assert_eq!(
            value_to_json(&Value::Bytes(b"hello".to_vec())),
            json!("hello")
        );
        assert_eq!(
            value_to_json(&Value::Bytes(vec![0xff, 0x00])),
            json!("<BLOB 2 bytes>")
        );
    }
}
