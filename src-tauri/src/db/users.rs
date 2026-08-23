use rusqlite::{params, Connection, OptionalExtension, Result as SqlResult};

use super::*;

use crate::auth;

pub fn user_count(conn: &Connection) -> SqlResult<u32> {
    conn.query_row("SELECT COUNT(*) FROM users", [], |row| row.get(0))
}

pub fn list_users(conn: &Connection) -> SqlResult<Vec<User>> {
    let mut stmt = conn.prepare("SELECT id, username, created_at FROM users ORDER BY id")?;
    let rows = stmt.query_map([], |row| {
        Ok(User {
            id: row.get(0)?,
            username: row.get(1)?,
            created_at: row.get(2)?,
        })
    })?;
    rows.collect()
}

/// 创建用户；用户名重复返回 Err（UNIQUE 约束）
pub fn create_user(conn: &Connection, username: &str, password: &str) -> SqlResult<User> {
    let stored = auth::new_password_hash(password);
    let created_at = now();
    conn.execute(
        "INSERT INTO users (username, salt, password_hash, created_at) VALUES (?1, ?2, ?3, ?4)",
        params![username, auth::salt_of_hash(&stored), stored, created_at],
    )?;
    let id = conn.last_insert_rowid();
    // 首个用户接管本地模式遗留的无主数据
    if user_count(conn)? == 1 {
        conn.execute(
            "UPDATE groups SET user_id = ?1 WHERE user_id IS NULL",
            params![id],
        )?;
    }
    Ok(User {
        id,
        username: username.to_string(),
        created_at,
    })
}

/// 校验用户名密码；成功返回用户。旧格式（单轮 SHA-256）哈希在登录成功时透明升级为 PBKDF2
pub fn verify_user(conn: &Connection, username: &str, password: &str) -> SqlResult<Option<User>> {
    let row = conn
        .query_row(
            "SELECT id, username, salt, password_hash, created_at FROM users WHERE username = ?1",
            params![username],
            |row| {
                Ok((
                    User {
                        id: row.get(0)?,
                        username: row.get(1)?,
                        created_at: row.get(4)?,
                    },
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            },
        )
        .optional()?;
    let Some((user, salt, hash)) = row else {
        return Ok(None);
    };
    if !auth::verify_password(password, &salt, &hash) {
        return Ok(None);
    }
    if auth::is_legacy_password_hash(&hash) {
        let stored = auth::new_password_hash(password);
        conn.execute(
            "UPDATE users SET salt = ?1, password_hash = ?2 WHERE id = ?3",
            params![auth::salt_of_hash(&stored), stored, user.id],
        )?;
    }
    Ok(Some(user))
}

/// 修改用户密码
pub fn change_user_password(
    conn: &Connection,
    user_id: i64,
    old_password: &str,
    new_password: &str,
) -> SqlResult<bool> {
    let row = conn
        .query_row(
            "SELECT salt, password_hash FROM users WHERE id = ?1",
            params![user_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()?;
    let Some((salt, hash)) = row else { return Ok(false) };
    if !auth::verify_password(old_password, &salt, &hash) {
        return Ok(false);
    }
    let stored = auth::new_password_hash(new_password);
    conn.execute(
        "UPDATE users SET salt = ?1, password_hash = ?2, default_password = 0 WHERE id = ?3",
        params![auth::salt_of_hash(&stored), stored, user_id],
    )?;
    Ok(true)
}

/// 播种默认分组（不存在才插入，可重复调用）

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth;
    use crate::db::test_conn;

    #[test]
    fn legacy_password_upgraded_on_login() {
        let (c, _admin) = test_conn();
        // 手工插入旧格式（单轮 SHA-256）用户，模拟历史存量数据
        let salt = auth::new_salt();
        let legacy = auth::hash_password("oldpass", &salt);
        c.execute(
            "INSERT INTO users (username, salt, password_hash, created_at)
             VALUES ('legacy', ?1, ?2, '2026-01-01T00:00:00Z')",
            params![salt, legacy],
        )
        .unwrap();

        let u = verify_user(&c, "legacy", "oldpass").unwrap().unwrap();
        // 登录成功后透明升级为 PBKDF2 格式
        let stored: String = c
            .query_row(
                "SELECT password_hash FROM users WHERE id = ?1",
                params![u.id],
                |r| r.get(0),
            )
            .unwrap();
        assert!(stored.starts_with("pbkdf2$"));
        // 新格式下旧密码仍可登录，错误密码被拒
        assert!(verify_user(&c, "legacy", "oldpass").unwrap().is_some());
        assert!(verify_user(&c, "legacy", "wrong").unwrap().is_none());
    }

    #[test]
    fn multi_user_isolation() {
        let (c, admin) = test_conn();
        // 初始用户 admin 拥有默认分组与数据
        let gid = list_groups(&c, admin).unwrap()[0].id;
        create_task(&c, admin, gid, "admin任务", "", None).unwrap();

        // 新注册用户：独立数据空间
        let u1 = create_user(&c, "alice", "pass1234").unwrap();
        assert_eq!(list_groups(&c, u1.id).unwrap().len(), 0);
        assert_eq!(list_tasks(&c, u1.id, None).unwrap().len(), 0);

        // admin 的分组与新用户互不可见/不可操作
        let g = &list_groups(&c, admin).unwrap()[0];
        assert!(rename_group(&c, u1.id, g.id, "抢注").unwrap().is_none());
        assert_eq!(list_tasks(&c, u1.id, Some(g.id)).unwrap().len(), 0);
        let t1 = list_tasks(&c, admin, None).unwrap()[0].clone();
        assert!(!delete_task(&c, u1.id, t1.id).unwrap());

        // 密码校验与默认密码标记
        assert!(verify_user(&c, "admin", "admin123").unwrap().is_some());
        assert!(verify_user(&c, "alice", "pass1234").unwrap().is_some());
        assert!(verify_user(&c, "alice", "wrong").unwrap().is_none());
        assert!(has_default_password_user(&c).unwrap()); // admin 仍是默认密码

        // 改密后清除默认标记
        assert!(change_user_password(&c, admin, "admin123", "newpass9").unwrap());
        assert!(verify_user(&c, "admin", "newpass9").unwrap().is_some());
        assert!(!has_default_password_user(&c).unwrap());
    }
}
