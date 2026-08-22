//! 本地认证基础组件：密码哈希（SHA-256 加盐）、随机 token、会话表。
//! 多用户模型：users 表由 db 模块管理；本模块维护 token → user_id 的内存会话映射，
//! 并通过 db::sessions 表持久化 —— 应用重启后已登录用户无需重新登录。

use rand::RngCore;
use rusqlite::Connection;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Debug, Default)]
pub struct Sessions {
    /// token -> user_id
    tokens: Mutex<HashMap<String, i64>>,
}

impl Sessions {
    /// 启动时从数据库恢复已持久化的会话
    pub fn load_from_db(&self, conn: &Connection) {
        if let Ok(list) = crate::db::load_sessions(conn) {
            let mut tokens = self.tokens.lock().unwrap();
            tokens.clear();
            tokens.extend(list);
        }
    }

    /// 为用户签发新 token（持久化到数据库）
    pub fn issue(&self, conn: &Connection, user_id: i64) -> String {
        let token = random_hex(24);
        let _ = crate::db::save_session(conn, &token, user_id);
        self.tokens.lock().unwrap().insert(token.clone(), user_id);
        token
    }

    pub fn user_id(&self, token: &str) -> Option<i64> {
        self.tokens.lock().unwrap().get(token).copied()
    }

    /// 撤销 token（登出，同时从数据库删除）
    pub fn revoke(&self, conn: &Connection, token: &str) {
        let _ = crate::db::delete_session(conn, token);
        self.tokens.lock().unwrap().remove(token);
    }
}

pub fn hash_password(password: &str, salt: &str) -> String {
    let mut h = Sha256::new();
    h.update(salt.as_bytes());
    h.update(password.as_bytes());
    format_hex(&h.finalize())
}

pub fn new_salt() -> String {
    random_hex(16)
}

pub fn random_hex(bytes: usize) -> String {
    let mut buf = vec![0u8; bytes];
    rand::rng().fill_bytes(&mut buf);
    format_hex(&buf)
}

fn format_hex(data: &[u8]) -> String {
    data.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_and_verify() {
        let salt = new_salt();
        let h1 = hash_password("secret123", &salt);
        let h2 = hash_password("secret123", &salt);
        assert_eq!(h1, h2);
        assert_ne!(h1, hash_password("secret124", &salt));
        assert_ne!(h1, hash_password("secret123", &new_salt()));
    }

    #[test]
    fn session_issue_revoke_and_reload() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE sessions (token TEXT PRIMARY KEY, user_id INTEGER NOT NULL);",
        )
        .unwrap();

        let s = Sessions::default();
        let t1 = s.issue(&conn, 1);
        let t2 = s.issue(&conn, 2);
        assert_eq!(s.user_id(&t1), Some(1));
        assert_eq!(s.user_id(&t2), Some(2));
        assert_eq!(s.user_id("bad"), None);

        // 模拟重启：新实例从数据库恢复会话
        let s2 = Sessions::default();
        s2.load_from_db(&conn);
        assert_eq!(s2.user_id(&t1), Some(1));
        assert_eq!(s2.user_id(&t2), Some(2));

        // 登出：撤销并同步删除
        s2.revoke(&conn, &t1);
        assert_eq!(s2.user_id(&t1), None);
        let s3 = Sessions::default();
        s3.load_from_db(&conn);
        assert_eq!(s3.user_id(&t1), None);
        assert_eq!(s3.user_id(&t2), Some(2));
    }

    #[test]
    fn random_hex_shape() {
        let a = random_hex(16);
        let b = random_hex(16);
        assert_eq!(a.len(), 32);
        assert_ne!(a, b);
    }
}