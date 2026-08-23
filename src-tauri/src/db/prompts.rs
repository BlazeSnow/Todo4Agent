use rusqlite::{params, Connection, OptionalExtension, Result as SqlResult};

use super::*;

/// 读取用户自定义提示词；未设置返回 None（默认提示词为空，由用户自行填写）
pub fn get_custom_prompt(conn: &Connection, user_id: i64) -> SqlResult<Option<(String, String)>> {
    conn.query_row(
        "SELECT content, updated_at FROM prompts WHERE user_id = ?1",
        params![user_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )
    .optional()
}

/// 全量保存提示词。空白内容视为清空：删除自定义行、回到默认空提示词。
/// 返回 (is_default, updated_at)：清空后为 (true, None)
pub fn set_prompt(conn: &Connection, user_id: i64, content: &str) -> SqlResult<(bool, Option<String>)> {
    if content.trim().is_empty() {
        conn.execute("DELETE FROM prompts WHERE user_id = ?1", params![user_id])?;
        return Ok((true, None));
    }
    let updated_at = now();
    conn.execute(
        "INSERT INTO prompts (user_id, content, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(user_id) DO UPDATE SET content = excluded.content, updated_at = excluded.updated_at",
        params![user_id, content, updated_at],
    )?;
    Ok((false, Some(updated_at)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_conn;

    #[test]
    fn prompt_default_empty_then_custom_then_clear() {
        let (c, admin) = test_conn();
        // 默认：无自定义（提示词为空，由用户自行填写）
        assert!(get_custom_prompt(&c, admin).unwrap().is_none());

        // 保存自定义后可读回，时间戳非空
        let (is_default, ts) = set_prompt(&c, admin, "我的规范").unwrap();
        assert!(!is_default);
        assert!(ts.is_some());
        let (content, updated_at) = get_custom_prompt(&c, admin).unwrap().unwrap();
        assert_eq!(content, "我的规范");
        assert_eq!(Some(updated_at), ts);

        // 空内容 / 纯空白 = 清空，回到默认状态
        assert_eq!(set_prompt(&c, admin, "").unwrap(), (true, None));
        assert!(get_custom_prompt(&c, admin).unwrap().is_none());
        set_prompt(&c, admin, "再次自定义").unwrap();
        assert_eq!(set_prompt(&c, admin, "   ").unwrap(), (true, None));
        assert!(get_custom_prompt(&c, admin).unwrap().is_none());
    }

    #[test]
    fn prompt_isolated_per_user() {
        let (c, admin) = test_conn();
        let other = create_user(&c, "carol", "pass1234").unwrap();
        set_prompt(&c, admin, "admin 的规范").unwrap();
        // 其他用户不受影响
        assert!(get_custom_prompt(&c, other.id).unwrap().is_none());
        set_prompt(&c, other.id, "carol 的规范").unwrap();
        assert_eq!(get_custom_prompt(&c, admin).unwrap().unwrap().0, "admin 的规范");
        assert_eq!(get_custom_prompt(&c, other.id).unwrap().unwrap().0, "carol 的规范");
        // 清空 admin 不影响 carol
        set_prompt(&c, admin, "").unwrap();
        assert!(get_custom_prompt(&c, admin).unwrap().is_none());
        assert_eq!(get_custom_prompt(&c, other.id).unwrap().unwrap().0, "carol 的规范");
    }
}
