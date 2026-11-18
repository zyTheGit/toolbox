//! 数据库浏览器命令：打开 / 关闭会话，读取结构与执行只读查询。

use tauri::State;

use crate::core::db::mysql::MysqlSession;
use crate::core::db::postgres::PostgresSession;
use crate::core::db::sqlite::SqliteSession;
use crate::core::db::Session;
use crate::models::{
    DbHistoryEntryView, DbOpenInput, DbQueryResult, DbSchema, DbSessionInfo,
};
use crate::state::AppState;

type CmdResult<T> = Result<T, String>;

#[tauri::command]
pub async fn db_open(
    input: DbOpenInput,
    state: State<'_, AppState>,
) -> CmdResult<DbSessionInfo> {
    let kind = input.kind.trim().to_ascii_lowercase();
    let (session, connection) = match kind.as_str() {
        "sqlite" => {
            let path = input.sqlite_path.unwrap_or_default();
            let path = crate::core::config::expand_home(path.trim());
            if path.is_empty() {
                return Err("[PARAM] 请选择 SQLite 数据库文件".into());
            }
            let history_connection = path.clone();
            let opened = tauri::async_runtime::spawn_blocking(move || SqliteSession::open(&path))
                .await
                .map_err(|e| format!("[DB] 打开 SQLite 失败: {e}"))??;
            (Session::Sqlite(opened), history_connection)
        }
        "postgres" => {
            let connection = input.connection_string.unwrap_or_default();
            let session = PostgresSession::open(&connection).await?;
            (Session::Postgres(session), connection)
        }
        "mysql" => {
            let connection = input.connection_string.unwrap_or_default();
            let session = MysqlSession::open(&connection).await?;
            (Session::Mysql(session), connection)
        }
        other => return Err(format!("[PARAM] 不支持的数据库类型: {other}")),
    };

    if let Err(err) = state.secrets.record_db_connection(&kind, &connection) {
        eprintln!("[DB] 记录连接历史失败: {err}");
    }
    let id = state.db.add(session.clone());
    Ok(DbSessionInfo {
        id,
        kind: session.kind().into(),
        label: session.label(),
        schema: session.schema(),
        databases: session.databases(),
    })
}

#[tauri::command]
pub fn db_schema(id: String, state: State<'_, AppState>) -> CmdResult<DbSchema> {
    Ok(state.db.get(&id)?.schema())
}

#[tauri::command]
pub async fn db_select_database(
    id: String,
    database: String,
    state: State<'_, AppState>,
) -> CmdResult<DbSessionInfo> {
    let session = state.db.get(&id)?;
    let schema = session.select_database(&database).await?;
    Ok(DbSessionInfo {
        id,
        kind: session.kind().into(),
        label: session.label(),
        schema,
        databases: session.databases(),
    })
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
pub fn db_close_all(state: State<'_, AppState>) -> CmdResult<()> {
    state.db.close_all();
    Ok(())
}

#[tauri::command]
pub fn db_history_list(state: State<'_, AppState>) -> CmdResult<Vec<DbHistoryEntryView>> {
    Ok(state.secrets.list_db_connections())
}

#[tauri::command]
pub fn db_history_reveal(id: String, state: State<'_, AppState>) -> CmdResult<String> {
    state.secrets.reveal_db_connection(&id)
}

#[tauri::command]
pub fn db_history_rename(
    id: String,
    name: String,
    state: State<'_, AppState>,
) -> CmdResult<Vec<DbHistoryEntryView>> {
    state.secrets.rename_db_connection(&id, &name)?;
    Ok(state.secrets.list_db_connections())
}

#[tauri::command]
pub async fn db_pick_sqlite() -> CmdResult<Option<String>> {
    tauri::async_runtime::spawn_blocking(|| {
        crate::dialogs::pick_sqlite_file().map(|path| path.to_string_lossy().to_string())
    })
    .await
    .map_err(|e| format!("[TOOL] 打开文件选择器失败: {e}"))
}
