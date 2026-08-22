//! 本地认证基础组件：密码哈希（SHA-256 加盐）、随机 token、会话表。
//! 多用户模型：users 表由 db 模块管理；本模块维护 token → user_id 的内存会话映射。
//! 未创建任何用户前软件处于“本地模式”（无登录要求）；创建首个用户后进入多用户模式。

use rand::RngCore;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Debug, Default)]
pub struct Sessions {
    /// token -> user_id
    tokens: Mutex<HashMap<String, i64>>,
}

impl Sessions {
    /// 为用户签发新 token
    pub fn issue(&self, user_id: i64) -> String {
        let token = random_hex(24);
        self.tokens.lock().unwrap().insert(token.clone(), user_id);
        token
    }

    pub fn user_id(&self, token: &str) -> Option<i64> {
        self.tokens.lock().unwrap().get(token).copied()
    }

    /// 撤销 token（登出）
    pub fn revoke(&self, token: &str) {
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
    fn session_issue_revoke() {
        let s = Sessions::default();
        let t1 = s.issue(1);
        let t2 = s.issue(2);
        assert_eq!(s.user_id(&t1), Some(1));
        assert_eq!(s.user_id(&t2), Some(2));
        assert_eq!(s.user_id("bad"), None);
        s.revoke(&t1);
        assert_eq!(s.user_id(&t1), None);
        assert_eq!(s.user_id(&t2), Some(2));
    }

    #[test]
    fn random_hex_shape() {
        let a = random_hex(16);
        let b = random_hex(16);
        assert_eq!(a.len(), 32);
        assert_ne!(a, b);
    }
}