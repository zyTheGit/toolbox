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

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DbSessionInfo {
    pub id: String,
    pub kind: String,
    pub label: String,
    pub schema: DbSchema,
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
