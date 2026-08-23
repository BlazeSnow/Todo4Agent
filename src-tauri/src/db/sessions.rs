use rusqlite::{params, Connection, OptionalExtension, Result as SqlResult};

use crate::auth;

/// 为用户签发新会话 token（持久化到 sessions 表）
pub fn issue_session(conn: &Connection, user_id: i64) -> SqlResult<String> {
    let token = auth::random_hex(24);
    save_session(conn, &token, user_id)?;
    Ok(token)
}

/// 校验会话 token，不存在返回 None。
/// 直接查库而非内存缓存：桌面 / serve / mcp 多进程共享同一数据库文件，
/// 任一进程的签发或吊销（如 MCP 改密后吊销会话）对其他进程立即生效
pub fn session_user_id(conn: &Connection, token: &str) -> SqlResult<Option<i64>> {
    conn.query_row(
        "SELECT user_id FROM sessions WHERE token = ?1",
        params![token],
        |row| row.get(0),
    )
    .optional()
}

pub fn save_session(conn: &Connection, token: &str, user_id: i64) -> SqlResult<()> {
    conn.execute(
        "INSERT INTO sessions (token, user_id) VALUES (?1, ?2)
         ON CONFLICT(token) DO UPDATE SET user_id = excluded.user_id",
        params![token, user_id],
    )?;
    Ok(())
}

pub fn delete_session(conn: &Connection, token: &str) -> SqlResult<()> {
    conn.execute("DELETE FROM sessions WHERE token = ?1", params![token])?;
    Ok(())
}

/// 删除某用户全部会话；keep 指定的 token 保留（改密后保留当前登录）
pub fn delete_user_sessions(conn: &Connection, user_id: i64, keep: Option<&str>) -> SqlResult<()> {
    match keep {
        Some(t) => conn.execute(
            "DELETE FROM sessions WHERE user_id = ?1 AND token != ?2",
            params![user_id, t],
        ),
        None => conn.execute("DELETE FROM sessions WHERE user_id = ?1", params![user_id]),
    }?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sessions_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE sessions (token TEXT PRIMARY KEY, user_id INTEGER NOT NULL);",
        )
        .unwrap();
        conn
    }

    #[test]
    fn issue_lookup_revoke() {
        let c = sessions_conn();
        let t1 = issue_session(&c, 1).unwrap();
        let t2 = issue_session(&c, 2).unwrap();
        assert_eq!(session_user_id(&c, &t1).unwrap(), Some(1));
        assert_eq!(session_user_id(&c, &t2).unwrap(), Some(2));
        assert_eq!(session_user_id(&c, "bad").unwrap(), None);

        // 登出撤销后立即失效（无需重启）
        delete_session(&c, &t1).unwrap();
        assert_eq!(session_user_id(&c, &t1).unwrap(), None);
        assert_eq!(session_user_id(&c, &t2).unwrap(), Some(2));
    }

    #[test]
    fn delete_user_sessions_keeps_current() {
        let c = sessions_conn();
        let t1 = issue_session(&c, 1).unwrap();
        let t2 = issue_session(&c, 1).unwrap();
        let t3 = issue_session(&c, 2).unwrap();

        // 改密后：用户 1 仅保留当前 token，用户 2 不受影响
        delete_user_sessions(&c, 1, Some(&t1)).unwrap();
        assert_eq!(session_user_id(&c, &t1).unwrap(), Some(1));
        assert_eq!(session_user_id(&c, &t2).unwrap(), None);
        assert_eq!(session_user_id(&c, &t3).unwrap(), Some(2));

        // keep=None 时全部吊销
        delete_user_sessions(&c, 1, None).unwrap();
        assert_eq!(session_user_id(&c, &t1).unwrap(), None);
    }
}
