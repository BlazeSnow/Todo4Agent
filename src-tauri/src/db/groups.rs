use rusqlite::{params, Connection, OptionalExtension, Result as SqlResult};

use super::*;

pub fn list_groups(conn: &Connection, user_id: Option<i64>) -> SqlResult<Vec<Group>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {GROUP_COLS} FROM groups WHERE deleted_at IS NULL AND user_id IS ?1 ORDER BY sort_order, id"
    ))?;
    let rows = stmt.query_map(params![user_id], group_from_row)?;
    rows.collect()
}

pub fn create_group(conn: &Connection, user_id: Option<i64>, name: &str) -> SqlResult<Group> {
    let created_at = now();
    conn.execute(
        "INSERT INTO groups (name, sort_order, created_at, user_id) VALUES (?1, 0, ?2, ?3)",
        params![name, created_at, user_id],
    )?;
    Ok(Group {
        id: conn.last_insert_rowid(),
        name: name.to_string(),
        sort_order: 0,
        created_at,
        deleted_at: None,
    })
}

/// 分组是否属于该用户
pub fn group_owned_by(conn: &Connection, user_id: Option<i64>, group_id: i64) -> SqlResult<bool> {
    conn.query_row(
        "SELECT 1 FROM groups WHERE id = ?1 AND user_id IS ?2",
        params![group_id, user_id],
        |_| Ok(()),
    )
    .optional()
    .map(|r| r.is_some())
}

/// 重命名分组；分组不存在、已删除或不属于该用户返回 Ok(None)
pub fn rename_group(
    conn: &Connection,
    user_id: Option<i64>,
    id: i64,
    name: &str,
) -> SqlResult<Option<Group>> {
    let updated = conn.execute(
        "UPDATE groups SET name = ?1 WHERE id = ?2 AND user_id IS ?3 AND deleted_at IS NULL",
        params![name, id, user_id],
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
pub fn reorder_groups(conn: &Connection, user_id: Option<i64>, group_ids: &[i64]) -> SqlResult<()> {
    let tx = conn.unchecked_transaction()?;
    {
        let mut stmt = tx.prepare(
            "UPDATE groups SET sort_order = ?1 WHERE id = ?2 AND user_id IS ?3 AND deleted_at IS NULL",
        )?;
        for (i, gid) in group_ids.iter().enumerate() {
            stmt.execute(params![i as i64, gid, user_id])?;
        }
    }
    tx.commit()
}

/// 软删除分组（连同其下任务一并进入回收站）；不存在、已删除或不属于该用户返回 Ok(false)
pub fn delete_group(conn: &Connection, user_id: Option<i64>, id: i64) -> SqlResult<bool> {
    let tx = conn.unchecked_transaction()?;
    let n = tx.execute(
        "UPDATE groups SET deleted_at = ?1 WHERE id = ?2 AND user_id IS ?3 AND deleted_at IS NULL",
        params![now(), id, user_id],
    )?;
    if n > 0 {
        tx.execute(
            "UPDATE tasks SET deleted_at = ?1 WHERE group_id = ?2 AND deleted_at IS NULL",
            params![now(), id],
        )?;
    }
    tx.commit()?;
    Ok(n > 0)
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_conn;

    #[test]
    fn group_crud_and_unique() {
        let (c, admin) = test_conn();
        let g = create_group(&c, Some(admin), "工作").unwrap();
        assert_eq!(g.id, 2); // 默认分组占 id=1
        let dup = create_group(&c, Some(admin), "工作").unwrap_err();
        assert!(is_unique_violation(&dup));
        assert!(rename_group(&c, Some(admin), g.id, "生活").unwrap().is_some());
        assert!(list_groups(&c, Some(admin)).unwrap().iter().any(|x| x.name == "生活"));
        assert!(delete_group(&c, Some(admin), g.id).unwrap());
        assert!(rename_group(&c, Some(admin), 999, "x").unwrap().is_none());
        assert!(!delete_group(&c, Some(admin), g.id).unwrap());
    }

    #[test]
    fn cascade_delete_group() {
        let (c, admin) = test_conn();
        let g = create_group(&c, Some(admin), "临时").unwrap();
        create_task(&c, Some(admin), g.id, "任务", "", None).unwrap();
        assert_eq!(list_tasks(&c, Some(admin), None).unwrap().len(), 1);
        delete_group(&c, Some(admin), g.id).unwrap();
        assert_eq!(list_tasks(&c, Some(admin), None).unwrap().len(), 0);
    }

    #[test]
    fn reorder_groups_order() {
        let (c, admin) = test_conn();
        let g1 = create_group(&c, Some(admin), "甲").unwrap();
        let g2 = create_group(&c, Some(admin), "乙").unwrap();
        let g3 = create_group(&c, Some(admin), "丙").unwrap();

        reorder_groups(&c, Some(admin), &[g3.id, g1.id, g2.id]).unwrap();
        let ids: Vec<i64> = list_groups(&c, Some(admin)).unwrap().iter().map(|g| g.id).collect();
        // 默认分组（快速清单）在最前，其后依次为 丙、甲、乙
        assert_eq!(ids, vec![1, g3.id, g1.id, g2.id]);
    }

    #[test]
    fn group_name_scoped_per_user_and_trash() {
        let (c, admin) = test_conn();
        let other = create_user(&c, "bob", "pass1234").unwrap();

        // 其他用户可以创建与 admin 同名的分组（唯一性按用户生效）
        create_group(&c, Some(other.id), DEFAULT_GROUP).unwrap();

        // 回收站中的分组不占用名字：删除后可再建同名分组
        let g = create_group(&c, Some(admin), "项目").unwrap();
        delete_group(&c, Some(admin), g.id).unwrap();
        let g2 = create_group(&c, Some(admin), "项目").unwrap();
        assert_ne!(g.id, g2.id);

        // 同一用户的活动分组仍然不能重名
        let dup = create_group(&c, Some(admin), "项目").unwrap_err();
        assert!(is_unique_violation(&dup));
    }
}
