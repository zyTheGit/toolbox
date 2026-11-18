//! 会话池：限制并发会话数量、回收空闲会话，并统一关闭底层连接。

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::Mutex;

use super::Session;

/// 同时保留的会话上限；打开第 9 个时淘汰最久未使用的会话。
pub const MAX_SESSIONS: usize = 8;
/// 会话空闲多久后自动回收。
pub const IDLE_TIMEOUT: Duration = Duration::from_secs(30 * 60);
/// 后台回收线程的检查间隔。
const REAP_INTERVAL: Duration = Duration::from_secs(60);

struct SessionEntry {
    session: Session,
    opened_at: Instant,
    last_used_at: Instant,
}

impl SessionEntry {
    fn new(session: Session, now: Instant) -> Self {
        Self {
            session,
            opened_at: now,
            last_used_at: now,
        }
    }

    /// 空闲时间是否已达到回收阈值。
    fn is_idle(&self, now: Instant) -> bool {
        now.saturating_duration_since(self.last_used_at) >= IDLE_TIMEOUT
    }
}

/// 数据库会话池。所有策略都在内部完成，命令层只负责打开 / 取用 / 关闭。
pub struct DbManager {
    sessions: Mutex<HashMap<String, SessionEntry>>,
}

impl DbManager {
    pub fn new() -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
        }
    }

    /// 启动后台回收线程：每 60 秒清理一次空闲会话，会话池释放后线程自行退出。
    pub fn start_reaper(self: &Arc<Self>) {
        let weak = Arc::downgrade(self);
        std::thread::spawn(move || loop {
            std::thread::sleep(REAP_INTERVAL);
            let Some(manager) = weak.upgrade() else {
                break;
            };
            manager.sweep_idle();
        });
    }

    /// 记录新会话并返回 id；超限淘汰最久未使用的会话，空闲超时的会话一并回收。
    pub fn add(&self, session: Session) -> String {
        let now = Instant::now();
        let id = uuid::Uuid::new_v4().to_string();
        let retired = {
            let mut sessions = self.sessions.lock();
            let mut retired = take_idle(&mut sessions, now);
            while sessions.len() >= MAX_SESSIONS {
                let Some(oldest) = sessions
                    .iter()
                    .min_by_key(|(_, entry)| (entry.last_used_at, entry.opened_at))
                    .map(|(id, _)| id.clone())
                else {
                    break;
                };
                if let Some(entry) = sessions.remove(&oldest) {
                    retired.push(entry.session);
                }
            }
            sessions.insert(id.clone(), SessionEntry::new(session, now));
            retired
        };
        retire(retired);
        id
    }

    /// 取会话并刷新最后使用时间；不存在的会话提示重新连接。
    pub fn get(&self, id: &str) -> Result<Session, String> {
        let now = Instant::now();
        let (found, retired) = {
            let mut sessions = self.sessions.lock();
            let retired = take_idle(&mut sessions, now);
            let found = sessions.get_mut(id).map(|entry| {
                entry.last_used_at = now;
                entry.session.clone()
            });
            (found, retired)
        };
        retire(retired);
        found.ok_or_else(|| "[PARAM] 数据库会话不存在，请重新连接".to_string())
    }

    pub fn remove(&self, id: &str) {
        let removed = self.sessions.lock().remove(id);
        if let Some(entry) = removed {
            retire(vec![entry.session]);
        }
    }

    /// 关闭全部会话，用于页面卸载或需要彻底释放连接的场景。
    pub fn close_all(&self) {
        let retired = {
            let mut sessions = self.sessions.lock();
            sessions
                .drain()
                .map(|(_, entry)| entry.session)
                .collect::<Vec<_>>()
        };
        retire(retired);
    }

    /// 立即回收一次空闲会话，返回回收数量。
    pub fn sweep_idle(&self) -> usize {
        let now = Instant::now();
        let mut sessions = self.sessions.lock();
        let retired = take_idle(&mut sessions, now);
        drop(sessions);
        let count = retired.len();
        retire(retired);
        count
    }
}

/// 摘出所有空闲超时的会话。
fn take_idle(sessions: &mut HashMap<String, SessionEntry>, now: Instant) -> Vec<Session> {
    let idle: Vec<String> = sessions
        .iter()
        .filter(|(_, entry)| entry.is_idle(now))
        .map(|(id, _)| id.clone())
        .collect();
    idle.into_iter()
        .filter_map(|id| sessions.remove(&id).map(|entry| entry.session))
        .collect()
}

/// 释放会话：MySQL 显式断开，SQLite / PostgreSQL 随 Drop 释放连接。
fn retire(sessions: Vec<Session>) {
    for session in sessions {
        if let Session::Mysql(mysql) = session {
            tauri::async_runtime::spawn(async move { mysql.shutdown().await });
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::core::db::sqlite::temp_session;

    fn add_sqlite(manager: &DbManager, tag: &str, dirs: &mut Vec<PathBuf>) -> String {
        let (dir, session) = temp_session(tag);
        dirs.push(dir);
        manager.add(Session::Sqlite(session))
    }

    fn cleanup(manager: DbManager, dirs: Vec<PathBuf>) {
        drop(manager);
        for dir in dirs {
            std::fs::remove_dir_all(dir).ok();
        }
    }

    #[test]
    fn ninth_session_evicts_least_recently_used() {
        let manager = DbManager::new();
        let mut dirs = Vec::new();
        let ids = (0..MAX_SESSIONS)
            .map(|index| add_sqlite(&manager, &format!("evict-{index}"), &mut dirs))
            .collect::<Vec<_>>();
        // 先访问第 1 个会话，让它变成最近使用；其余按打开顺序退化
        manager.get(&ids[0]).unwrap();

        let ninth = add_sqlite(&manager, "evict-8", &mut dirs);
        assert_eq!(manager.sessions.lock().len(), MAX_SESSIONS);
        assert!(
            manager.get(&ids[1]).is_err(),
            "最久未使用的会话应被淘汰"
        );
        assert!(manager.get(&ids[0]).is_ok());
        assert!(manager.get(&ninth).is_ok());
        manager.close_all();
        assert_eq!(manager.sessions.lock().len(), 0);
        cleanup(manager, dirs);
    }

    #[test]
    fn idle_sessions_are_reaped_after_ttl() {
        let manager = DbManager::new();
        let mut dirs = Vec::new();
        let id = add_sqlite(&manager, "idle", &mut dirs);
        {
            let mut sessions = manager.sessions.lock();
            let entry = sessions.get_mut(&id).unwrap();
            // 把最后使用时间拨回到 TTL 之前
            entry.last_used_at = Instant::now() - IDLE_TIMEOUT - Duration::from_secs(1);
        }
        assert_eq!(manager.sweep_idle(), 1);
        assert_eq!(manager.sessions.lock().len(), 0);
        assert!(manager.get(&id).is_err());
        cleanup(manager, dirs);
    }

    #[test]
    fn recently_used_sessions_survive_sweep() {
        let manager = DbManager::new();
        let mut dirs = Vec::new();
        let id = add_sqlite(&manager, "fresh", &mut dirs);
        assert_eq!(manager.sweep_idle(), 0);
        assert!(manager.get(&id).is_ok());
        manager.remove(&id);
        assert!(manager.get(&id).is_err());
        cleanup(manager, dirs);
    }

    #[test]
    fn close_all_closes_every_session() {
        let manager = DbManager::new();
        let mut dirs = Vec::new();
        let first = add_sqlite(&manager, "all-a", &mut dirs);
        let second = add_sqlite(&manager, "all-b", &mut dirs);
        assert_eq!(manager.sessions.lock().len(), 2);
        manager.close_all();
        assert_eq!(manager.sessions.lock().len(), 0);
        assert!(manager.get(&first).is_err());
        assert!(manager.get(&second).is_err());
        cleanup(manager, dirs);
    }
}
