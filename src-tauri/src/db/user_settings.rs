use rusqlite::{params, Connection, OptionalExtension, Result as SqlResult};

use super::*;

/// 任务清单锁定键：开启后 Agent 无法通过 MCP 修改任务数据（读取不受影响，
/// 界面编辑不受影响），用户可在「设置 → Agent 权限」切换
pub const KEY_TASKS_LOCKED: &str = "tasks_locked";

pub fn get_user_setting(conn: &Connection, user_id: i64, key: &str) -> SqlResult<Option<String>> {
    conn.query_row(
        "SELECT value FROM user_settings WHERE user_id = ?1 AND key = ?2",
        params![user_id, key],
        |row| row.get(0),
    )
    .optional()
}

pub fn set_user_setting(conn: &Connection, user_id: i64, key: &str, value: &str) -> SqlResult<()> {
    conn.execute(
        "INSERT INTO user_settings (user_id, key, value) VALUES (?1, ?2, ?3)
         ON CONFLICT(user_id, key) DO UPDATE SET value = excluded.value",
        params![user_id, key, value],
    )?;
    Ok(())
}

/// 任务清单是否已锁定（对该用户）
pub fn tasks_locked(conn: &Connection, user_id: i64) -> SqlResult<bool> {
    Ok(
        get_user_setting(conn, user_id, KEY_TASKS_LOCKED)?
            .as_deref()
            .is_some_and(|v| v != "0"),
    )
}

pub fn set_tasks_locked(conn: &Connection, user_id: i64, locked: bool) -> SqlResult<()> {
    set_user_setting(conn, user_id, KEY_TASKS_LOCKED, if locked { "1" } else { "0" })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_conn;

    #[test]
    fn tasks_lock_default_off_and_toggle() {
        let (c, admin) = test_conn();
        // 默认未锁定
        assert!(!tasks_locked(&c, admin).unwrap());
        // 开启后可读回；非法值按未锁定处理
        set_tasks_locked(&c, admin, true).unwrap();
        assert!(tasks_locked(&c, admin).unwrap());
        set_user_setting(&c, admin, KEY_TASKS_LOCKED, "0").unwrap();
        assert!(!tasks_locked(&c, admin).unwrap());
        set_user_setting(&c, admin, KEY_TASKS_LOCKED, "bogus").unwrap();
        assert!(tasks_locked(&c, admin).unwrap()); // 非 "0" 均视为开
        set_tasks_locked(&c, admin, false).unwrap();
        assert!(!tasks_locked(&c, admin).unwrap());
    }

    #[test]
    fn user_settings_isolated_per_user() {
        let (c, admin) = test_conn();
        let other = create_user(&c, "erin", "pass1234").unwrap();
        set_tasks_locked(&c, admin, true).unwrap();
        // 其他用户不受影响
        assert!(!tasks_locked(&c, other.id).unwrap());
        set_tasks_locked(&c, other.id, true).unwrap();
        set_tasks_locked(&c, admin, false).unwrap();
        assert!(tasks_locked(&c, other.id).unwrap());
        assert!(!tasks_locked(&c, admin).unwrap());
    }
}
