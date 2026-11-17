//! 数据库浏览器：连接、结构与只读查询结果模型。

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields, default)]
pub struct DbOpenInput {
    /// sqlite / postgres / mysql
    pub kind: String,
    pub sqlite_path: Option<String>,
    pub connection_string: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct DbColumn {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
    pub primary_key: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct DbTable {
    /// 展示用名称，PostgreSQL / MySQL 为 `schema.table`。
    pub name: String,
    pub schema: String,
    /// table / view
    pub kind: String,
    pub columns: Vec<DbColumn>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct DbSchema {
    pub tables: Vec<DbTable>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct DbDatabase {
    pub name: String,
    pub table_count: usize,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DbSessionInfo {
    pub id: String,
    pub kind: String,
    pub label: String,
    pub schema: DbSchema,
    /// MySQL 可切换的数据库；其它类型为空。
    pub databases: Vec<DbDatabase>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DbHistoryEntryView {
    pub id: String,
    pub kind: String,
    /// 自定义记录名称，默认为脱敏连接目标。
    pub name: String,
    /// 脱敏后的连接目标，不含密码。
    pub label: String,
    pub updated_at: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DbQueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<serde_json::Value>>,
    pub offset: usize,
    pub limit: usize,
    /// 当前页之后是否还有数据。
    pub has_more: bool,
    pub elapsed_ms: u64,
}
