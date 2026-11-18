//! PostgreSQL 只读会话：连接后固定为只读事务并限制语句超时。
//! 结果统一走 simple_query 的文本行，避免为每种类型单独做解码。

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Instant;

use serde_json::{json, Value};
use tokio_postgres::{Client, SimpleQueryMessage};
use tokio_postgres_rustls::MakeRustlsConnect;

use super::sql;
use crate::models::{DbColumn, DbQueryResult, DbSchema, DbTable};

#[derive(Clone)]
pub struct PostgresSession {
    label: String,
    schema: DbSchema,
    client: Arc<Client>,
}

impl PostgresSession {
    pub async fn open(connection_string: &str) -> Result<Self, String> {
        let config: tokio_postgres::Config = connection_string
            .trim()
            .parse()
            .map_err(|e| format!("[PARAM] PostgreSQL 连接串无效: {e}"))?;
        let label = config
            .get_dbname()
            .map(|name| name.to_string())
            .unwrap_or_else(|| "postgres".into());

        let connector = MakeRustlsConnect::new(tls_config()?);
        let (client, connection) = config
            .connect(connector)
            .await
            .map_err(|e| format!("[DB] 连接 PostgreSQL 失败: {e}"))?;
        tokio::spawn(async move {
            let _ = connection.await;
        });
        client
            .batch_execute("SET default_transaction_read_only = on; SET statement_timeout = 15000")
            .await
            .map_err(|e| format!("[DB] 初始化只读会话失败: {e}"))?;

        let schema = load_schema(&client).await?;
        Ok(Self {
            label,
            schema,
            client: Arc::new(client),
        })
    }

    pub fn kind(&self) -> &'static str {
        "postgres"
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn schema(&self) -> &DbSchema {
        &self.schema
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
        sql::ensure_read_only("PostgreSQL", &keyword)?;
        let paged = sql::wrappable(&keyword);
        let text = if paged {
            sql::wrap_page(&statement, limit, offset)
        } else {
            statement
        };

        let messages = self.run_read_only(&text).await?;

        let mut columns: Vec<String> = Vec::new();
        let mut all: Vec<Vec<Value>> = Vec::new();
        for message in messages {
            if let SimpleQueryMessage::Row(row) = message {
                if columns.is_empty() {
                    columns = row
                        .columns()
                        .iter()
                        .map(|column| column.name().to_string())
                        .collect();
                }
                let values = (0..row.len())
                    .map(|index| match row.get(index) {
                        Some(text) => json!(text),
                        None => Value::Null,
                    })
                    .collect();
                all.push(values);
            }
        }

        let (rows, has_more) = if paged {
            let has_more = all.len() > limit;
            all.truncate(limit);
            (all, has_more)
        } else {
            let mut rows: Vec<_> = all.into_iter().skip(offset).take(limit + 1).collect();
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

    /// 每条查询包在显式只读事务里执行：即使 GUC 被 `set_config` 之类的函数改掉，
    /// 当前事务仍然是 READ ONLY，写语句依旧被服务器拒绝。
    async fn run_read_only(&self, text: &str) -> Result<Vec<SimpleQueryMessage>, String> {
        self.client
            .batch_execute("BEGIN READ ONLY")
            .await
            .map_err(|e| format!("[DB] 开启只读事务失败: {e}"))?;
        let outcome = self.client.simple_query(text).await;
        if let Err(err) = self.client.batch_execute("ROLLBACK").await {
            // 连接已断开时回滚同样会失败，这里只记录，不影响原始错误
            eprintln!("[DB] 回滚只读事务失败: {err}");
        }
        outcome.map_err(|e| format!("[DB] 查询失败: {e}"))
    }
}

async fn load_schema(client: &Client) -> Result<DbSchema, String> {
    let tables = client
        .query(
            "SELECT table_schema, table_name, table_type FROM information_schema.tables \
             WHERE table_schema NOT IN ('pg_catalog','information_schema') \
             AND table_type IN ('BASE TABLE','VIEW') ORDER BY table_schema, table_name",
            &[],
        )
        .await
        .map_err(|e| format!("[DB] 读取表列表失败: {e}"))?;
    let columns = client
        .query(
            "SELECT table_schema, table_name, column_name, data_type, is_nullable \
             FROM information_schema.columns \
             WHERE table_schema NOT IN ('pg_catalog','information_schema') \
             ORDER BY table_schema, table_name, ordinal_position",
            &[],
        )
        .await
        .map_err(|e| format!("[DB] 读取列信息失败: {e}"))?;
    let pks = client
        .query(
            "SELECT tc.table_schema, tc.table_name, kcu.column_name \
             FROM information_schema.table_constraints tc \
             JOIN information_schema.key_column_usage kcu \
               ON kcu.constraint_name = tc.constraint_name \
              AND kcu.table_schema = tc.table_schema \
             WHERE tc.constraint_type = 'PRIMARY KEY'",
            &[],
        )
        .await
        .map_err(|e| format!("[DB] 读取主键信息失败: {e}"))?;

    let primary: HashSet<(String, String, String)> = pks
        .iter()
        .map(|row| (row.get(0), row.get(1), row.get(2)))
        .collect();
    let mut grouped: HashMap<(String, String), Vec<DbColumn>> = HashMap::new();
    for row in columns {
        let schema: String = row.get(0);
        let table: String = row.get(1);
        let name: String = row.get(2);
        let data_type: String = row.get(3);
        let nullable: String = row.get(4);
        let primary_key = primary.contains(&(schema.clone(), table.clone(), name.clone()));
        grouped.entry((schema, table)).or_default().push(DbColumn {
            name,
            data_type,
            nullable: nullable == "YES",
            primary_key,
        });
    }

    let mut result = Vec::with_capacity(tables.len());
    for row in tables {
        let schema: String = row.get(0);
        let table: String = row.get(1);
        let table_type: String = row.get(2);
        let columns = grouped.remove(&(schema.clone(), table.clone())).unwrap_or_default();
        result.push(DbTable {
            name: format!("{schema}.{table}"),
            schema,
            kind: if table_type == "VIEW" { "view".into() } else { "table".into() },
            columns,
        });
    }
    Ok(DbSchema { tables: result })
}

fn tls_config() -> Result<rustls::ClientConfig, String> {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let mut roots = rustls::RootCertStore::empty();
    let native = rustls_native_certs::load_native_certs();
    for cert in native.certs {
        let _ = roots.add(cert);
    }
    if roots.is_empty() {
        roots
            .roots
            .extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    }
    Ok(rustls::ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth())
}
