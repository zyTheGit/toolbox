//! 数据库浏览器命令：打开 / 关闭会话，读取结构与执行只读查询。

use tauri::State;

use crate::core::db::mysql::MysqlSession;
use crate::core::db::postgres::PostgresSession;
use crate::core::db::sqlite::SqliteSession;
use crate::core::db::Session;
use crate::models::{DbOpenInput, DbQueryResult, DbSchema, DbSessionInfo};
use crate::state::AppState;

type CmdResult<T> = Result<T, String>;

#[tauri::command]
pub async fn db_open(
    input: DbOpenInput,
    state: State<'_, AppState>,
) -> CmdResult<DbSessionInfo> {
    let session = match input.kind.trim().to_ascii_lowercase().as_str() {
        "sqlite" => {
            let path = input.sqlite_path.unwrap_or_default();
            let path = crate::core::config::expand_home(path.trim());
            if path.is_empty() {
                return Err("[PARAM] 请选择 SQLite 数据库文件".into());
            }
            let opened = tauri::async_runtime::spawn_blocking(move || SqliteSession::open(&path))
                .await
                .map_err(|e| format!("[DB] 打开 SQLite 失败: {e}"))??;
            Session::Sqlite(opened)
        }
        "postgres" => {
            let connection = input.connection_string.unwrap_or_default();
            Session::Postgres(PostgresSession::open(&connection).await?)
        }
        "mysql" => {
            let connection = input.connection_string.unwrap_or_default();
            Session::Mysql(MysqlSession::open(&connection).await?)
        }
        other => return Err(format!("[PARAM] 不支持的数据库类型: {other}")),
    };

    let id = state.db.add(session.clone());
    Ok(DbSessionInfo {
        id,
        kind: session.kind().into(),
        label: session.label().into(),
        schema: session.schema().clone(),
    })
}

#[tauri::command]
pub fn db_schema(id: String, state: State<'_, AppState>) -> CmdResult<DbSchema> {
    Ok(state.db.get(&id)?.schema().clone())
}

#[tauri::command]
pub async fn db_query(
    id: String,
    sql: String,
    offset: usize,
    limit: usize,
    state: State<'_, AppState>,
) -> CmdResult<DbQueryResult> {
    let offset = offset.min(1_000_000);
    let limit = limit.clamp(1, 1000);
    state.db.get(&id)?.query(&sql, offset, limit).await
}

#[tauri::command]
pub fn db_close(id: String, state: State<'_, AppState>) -> CmdResult<()> {
    state.db.remove(&id);
    Ok(())
}

#[tauri::command]
pub async fn db_pick_sqlite() -> CmdResult<Option<String>> {
    tauri::async_runtime::spawn_blocking(|| {
        rfd::FileDialog::new()
            .set_title("选择 SQLite 数据库")
            .add_filter("SQLite", &["db", "sqlite", "sqlite3", "db3"])
            .add_filter("所有文件", &["*"])
            .pick_file()
            .map(|path| path.to_string_lossy().to_string())
    })
    .await
    .map_err(|e| format!("[TOOL] 打开文件选择器失败: {e}"))
}
