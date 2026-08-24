use rusqlite::{params, Connection, OptionalExtension, Result as SqlResult};

use super::*;

pub fn list_groups(conn: &Connection, user_id: i64) -> SqlResult<Vec<Group>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {GROUP_COLS} FROM groups WHERE deleted_at IS NULL AND user_id = ?1 ORDER BY sort_order, id"
    ))?;
    let rows = stmt.query_map(params![user_id], group_from_row)?;
    rows.collect()
}

pub fn create_group(conn: &Connection, user_id: i64, name: &str, description: &str) -> SqlResult<Group> {
    let created_at = now();
    conn.execute(
        "INSERT INTO groups (name, description, sort_order, created_at, user_id) VALUES (?1, ?2, 0, ?3, ?4)",
        params![name, description, created_at, user_id],
    )?;
    Ok(Group {
        id: conn.last_insert_rowid(),
        name: name.to_string(),
        description: description.to_string(),
        sort_order: 0,
        created_at,
        deleted_at: None,
        locked: false,
    })
}

/// 获取单个分组；不存在、已删除或不属于该用户返回 Ok(None)
pub fn get_group(conn: &Connection, user_id: i64, id: i64) -> SqlResult<Option<Group>> {
    conn.query_row(
        &format!("SELECT {GROUP_COLS} FROM groups WHERE id = ?1 AND user_id = ?2 AND deleted_at IS NULL"),
        params![id, user_id],
        group_from_row,
    )
    .optional()
}

/// 分组锁定状态：返回 (名称, 是否锁定)；不存在、已删除或不属于该用户返回 Ok(None)
pub fn group_lock_info(conn: &Connection, user_id: i64, group_id: i64) -> SqlResult<Option<(String, bool)>> {
    conn.query_row(
        "SELECT name, locked FROM groups WHERE id = ?1 AND user_id = ?2 AND deleted_at IS NULL",
        params![group_id, user_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )
    .optional()
}

/// 设置分组锁定；分组不存在、已删除或不属于该用户返回 Ok(false)
pub fn set_group_locked(conn: &Connection, user_id: i64, group_id: i64, locked: bool) -> SqlResult<bool> {
    let n = conn.execute(
        "UPDATE groups SET locked = ?1 WHERE id = ?2 AND user_id = ?3 AND deleted_at IS NULL",
        params![locked, group_id, user_id],
    )?;
    Ok(n > 0)
}

/// 更新分组描述；分组不存在、已删除或不属于该用户返回 Ok(false)
pub fn set_group_description(conn: &Connection, user_id: i64, group_id: i64, description: &str) -> SqlResult<bool> {
    let n = conn.execute(
        "UPDATE groups SET description = ?1 WHERE id = ?2 AND user_id = ?3 AND deleted_at IS NULL",
        params![description, group_id, user_id],
    )?;
    Ok(n > 0)
}

/// 该用户所有已锁定分组的名称（导入前冲突检查用）
pub fn locked_group_names(conn: &Connection, user_id: i64) -> SqlResult<Vec<String>> {
    let mut stmt =
        conn.prepare("SELECT name FROM groups WHERE user_id = ?1 AND locked = 1 AND deleted_at IS NULL")?;
    let rows = stmt.query_map(params![user_id], |row| row.get(0))?;
    rows.collect()
}

/// 分组是否属于该用户
pub fn group_owned_by(conn: &Connection, user_id: i64, group_id: i64) -> SqlResult<bool> {
    conn.query_row(
        "SELECT 1 FROM groups WHERE id = ?1 AND user_id = ?2",
        params![group_id, user_id],
        |_| Ok(()),
    )
    .optional()
    .map(|r| r.is_some())
}

/// 重命名分组；分组不存在、已删除、不属于该用户或为系统分组「无分组」返回 Ok(None)
pub fn rename_group(
    conn: &Connection,
    user_id: i64,
    id: i64,
    name: &str,
) -> SqlResult<Option<Group>> {
    let updated = conn.execute(
        "UPDATE groups SET name = ?1 WHERE id = ?2 AND user_id = ?3 AND deleted_at IS NULL
         AND name != ?4",
        params![name, id, user_id, NO_GROUP],
    )?;
    if updated == 0 {
        return Ok(None);
    }
    let row = conn.query_row(
        &format!("SELECT {GROUP_COLS} FROM groups WHERE id = ?1"),
        params![id],
        group_from_row,
    )?;
    Ok(Some(row))
}

/// 按给定顺序重排某用户的分组（group_ids 中分组的 sort_order 依次赋 0,1,2,...）
/// 调用方需持锁独占访问，故使用 unchecked_transaction
pub fn reorder_groups(conn: &Connection, user_id: i64, group_ids: &[i64]) -> SqlResult<()> {
    let tx = conn.unchecked_transaction()?;
    {
        let mut stmt = tx.prepare(
            "UPDATE groups SET sort_order = ?1 WHERE id = ?2 AND user_id = ?3 AND deleted_at IS NULL",
        )?;
        for (i, gid) in group_ids.iter().enumerate() {
            stmt.execute(params![i as i64, gid, user_id])?;
        }
    }
    tx.commit()
}

/// 软删除分组：组内全部任务（未归档 / 已归档 / 回收站中）移入系统分组「无分组」，
/// 任务本身不进回收站、不丢失；系统分组「无分组」不可删除。
/// 不存在、已删除、不属于该用户或为系统分组返回 Ok(false)
pub fn delete_group(conn: &Connection, user_id: i64, id: i64) -> SqlResult<bool> {
    let tx = conn.unchecked_transaction()?;
    let n = tx.execute(
        "UPDATE groups SET deleted_at = ?1 WHERE id = ?2 AND user_id = ?3 AND deleted_at IS NULL
         AND name != ?4",
        params![now(), id, user_id, NO_GROUP],
    )?;
    if n > 0 {
        let no_group = ensure_no_group(&tx, user_id)?;
        tx.execute(
            "UPDATE tasks SET group_id = ?1 WHERE group_id = ?2",
            params![no_group, id],
        )?;
    }
    tx.commit()?;
    Ok(n > 0)
}

/// 该用户系统分组「无分组」id；不存在时创建（不带描述）并返回。
/// 分组被删除时其任务（含归档）移入该分组；取消归档 / 从回收站恢复时
/// 原分组已不存在也回落到该分组
pub fn ensure_no_group(conn: &Connection, user_id: i64) -> SqlResult<i64> {
    let existing: Option<i64> = conn
        .query_row(
            "SELECT id FROM groups WHERE user_id = ?1 AND name = ?2 AND deleted_at IS NULL",
            params![user_id, NO_GROUP],
            |row| row.get(0),
        )
        .optional()?;
    if let Some(id) = existing {
        // 清理早期版本播种的默认描述（不匹配时为无操作）
        conn.execute(
            "UPDATE groups SET description = '' WHERE id = ?1 AND description = ?2",
            params![id, "分组删除后其任务的去处"],
        )?;
        return Ok(id);
    }
    Ok(create_group(conn, user_id, NO_GROUP, "")?.id)
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_conn;

    #[test]
    fn group_crud_and_unique() {
        let (c, admin) = test_conn();
        let g = create_group(&c, admin, "工作", "").unwrap();
        assert_eq!(g.id, 3); // 默认分组 id=1、系统分组「无分组」id=2
        let dup = create_group(&c, admin, "工作", "").unwrap_err();
        assert!(is_unique_violation(&dup));
        assert!(rename_group(&c, admin, g.id, "生活").unwrap().is_some());
        assert!(list_groups(&c, admin).unwrap().iter().any(|x| x.name == "生活"));
        assert!(delete_group(&c, admin, g.id).unwrap());
        assert!(rename_group(&c, admin, 999, "x").unwrap().is_none());
        assert!(!delete_group(&c, admin, g.id).unwrap());
    }

    #[test]
    fn delete_group_moves_tasks_to_no_group() {
        let (c, admin) = test_conn();
        let no_group = list_groups(&c, admin).unwrap().iter().find(|g| g.name == NO_GROUP).unwrap().id;
        let g = create_group(&c, admin, "临时", "").unwrap();
        create_task(&c, admin, g.id, "任务", "", None).unwrap();
        assert_eq!(list_tasks(&c, admin, None).unwrap().len(), 1);
        delete_group(&c, admin, g.id).unwrap();
        // 任务不随分组删除而删除，移入系统分组「无分组」
        assert!(list_tasks(&c, admin, Some(g.id)).unwrap().is_empty());
        assert_eq!(list_tasks(&c, admin, Some(no_group)).unwrap().len(), 1);
    }

    #[test]
    fn reorder_groups_order() {
        let (c, admin) = test_conn();
        let g1 = create_group(&c, admin, "甲", "").unwrap();
        let g2 = create_group(&c, admin, "乙", "").unwrap();
        let g3 = create_group(&c, admin, "丙", "").unwrap();

        reorder_groups(&c, admin, &[g3.id, g1.id, g2.id]).unwrap();
        let ids: Vec<i64> = list_groups(&c, admin).unwrap().iter().map(|g| g.id).collect();
        // 默认分组（快速清单）与系统分组（无分组）在最前，其后依次为 丙、甲、乙
        assert_eq!(ids, vec![1, 2, g3.id, g1.id, g2.id]);
    }

    #[test]
    fn group_name_scoped_per_user_and_trash() {
        let (c, admin) = test_conn();
        let other = create_user(&c, "bob", "pass1234").unwrap();

        // 其他用户可以创建与 admin 同名的分组（唯一性按用户生效）
        create_group(&c, other.id, DEFAULT_GROUP, "").unwrap();

        // 回收站中的分组不占用名字：删除后可再建同名分组
        let g = create_group(&c, admin, "项目", "").unwrap();
        delete_group(&c, admin, g.id).unwrap();
        let g2 = create_group(&c, admin, "项目", "").unwrap();
        assert_ne!(g.id, g2.id);

        // 同一用户的活动分组仍然不能重名
        let dup = create_group(&c, admin, "项目", "").unwrap_err();
        assert!(is_unique_violation(&dup));
    }

    #[test]
    fn group_description_roundtrip() {
        let (c, admin) = test_conn();
        let other = create_user(&c, "carol", "pass1234").unwrap();

        // 创建时带描述；默认分组无描述（空字符串）
        let g = create_group(&c, admin, "工作清单", "记录日常工作任务").unwrap();
        assert_eq!(g.description, "记录日常工作任务");
        let seeded = get_group(&c, admin, 1).unwrap().unwrap();
        assert_eq!(seeded.description, "");

        // 更新与清空
        assert!(set_group_description(&c, admin, g.id, "新描述").unwrap());
        assert_eq!(get_group(&c, admin, g.id).unwrap().unwrap().description, "新描述");
        assert!(set_group_description(&c, admin, g.id, "").unwrap());
        assert_eq!(get_group(&c, admin, g.id).unwrap().unwrap().description, "");

        // 软删除后不可更新；其他用户不可更新
        delete_group(&c, admin, g.id).unwrap();
        assert!(!set_group_description(&c, admin, g.id, "x").unwrap());
        let g2 = create_group(&c, admin, "另一个", "").unwrap();
        assert!(!set_group_description(&c, other.id, g2.id, "x").unwrap());
    }

    #[test]
    fn group_lock_roundtrip_and_isolation() {
        let (c, admin) = test_conn();
        let other = create_user(&c, "frank", "pass1234").unwrap();
        let g = create_group(&c, admin, "私人清单", "").unwrap();

        // 默认未锁定；锁定后 list/get/lock_info 均可见
        assert!(!g.locked);
        assert_eq!(group_lock_info(&c, admin, g.id).unwrap(), Some(("私人清单".into(), false)));
        set_group_locked(&c, admin, g.id, true).unwrap();
        let groups = list_groups(&c, admin).unwrap();
        assert!(groups.iter().any(|x| x.id == g.id && x.locked));
        assert_eq!(group_lock_info(&c, admin, g.id).unwrap(), Some(("私人清单".into(), true)));
        assert_eq!(get_group(&c, admin, g.id).unwrap().unwrap().locked, true);

        // 其他用户查不到（不存在语义），也改不了
        assert!(group_lock_info(&c, other.id, g.id).unwrap().is_none());
        assert!(!set_group_locked(&c, other.id, g.id, false).unwrap());

        // 解锁恢复；locked_group_names 只含锁定的
        set_group_locked(&c, admin, g.id, false).unwrap();
        assert!(locked_group_names(&c, admin).unwrap().is_empty());
        set_group_locked(&c, admin, g.id, true).unwrap();
        assert_eq!(locked_group_names(&c, admin).unwrap(), vec!["私人清单".to_string()]);

        // 任务所在分组的锁定信息（供 MCP 拦截）
        let t = create_task(&c, admin, g.id, "清单内任务", "", None).unwrap();
        let (_, name, locked) = task_group_lock(&c, admin, t.id).unwrap().unwrap();
        assert_eq!(name, "私人清单");
        assert!(locked);
        assert!(task_group_lock(&c, admin, 999).unwrap().is_none());
    }
}
