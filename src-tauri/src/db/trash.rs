use rusqlite::{params, Connection, Result as SqlResult};

use super::*;

pub fn list_trash(conn: &Connection, user_id: i64) -> SqlResult<(Vec<Group>, Vec<Task>)> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {GROUP_COLS} FROM groups WHERE deleted_at IS NOT NULL AND user_id = ?1 ORDER BY deleted_at DESC, id DESC"
    ))?;
    let groups: Vec<Group> =
        stmt.query_map(params![user_id], group_from_row)?.collect::<SqlResult<_>>()?;

    let mut stmt = conn.prepare(&format!(
        "SELECT {TASK_COLS} FROM tasks WHERE deleted_at IS NOT NULL
         AND group_id IN (SELECT id FROM groups WHERE user_id = ?1)
         ORDER BY deleted_at DESC, id DESC"
    ))?;
    let tasks: Vec<Task> =
        stmt.query_map(params![user_id], task_from_row)?.collect::<SqlResult<_>>()?;
    Ok((groups, tasks))
}

/// 从回收站恢复任务；不存在、未删除或不属于该用户返回 Ok(false)
pub fn restore_task(conn: &Connection, user_id: i64, id: i64) -> SqlResult<bool> {
    let n = conn.execute(
        "UPDATE tasks SET deleted_at = NULL, updated_at = ?1
         WHERE id = ?2 AND deleted_at IS NOT NULL
           AND group_id IN (SELECT id FROM groups WHERE user_id = ?3)",
        params![now(), id, user_id],
    )?;
    Ok(n > 0)
}

/// 彻底删除任务（物理删除）；不存在或不属于该用户返回 Ok(false)
pub fn purge_task(conn: &Connection, user_id: i64, id: i64) -> SqlResult<bool> {
    let n = conn.execute(
        "DELETE FROM tasks WHERE id = ?1
         AND group_id IN (SELECT id FROM groups WHERE user_id = ?2)",
        params![id, user_id],
    )?;
    Ok(n > 0)
}

/// 从回收站恢复分组及其下任务（仅限该用户的回收站项）。
/// 原名被该用户的现有分组占用时自动重命名为「原名 (2)」「原名 (3)」……
/// 返回值：Ok(None) = 不在回收站；Ok(Some(None)) = 原名恢复；
/// Ok(Some(Some(new))) = 恢复成功并重命名为 new
pub fn restore_group(
    conn: &Connection,
    user_id: i64,
    id: i64,
) -> SqlResult<Option<Option<String>>> {
    let tx = conn.unchecked_transaction()?;
    let original: Option<String> = tx
        .query_row(
            "SELECT name FROM groups WHERE id = ?1 AND user_id = ?2 AND deleted_at IS NOT NULL",
            params![id, user_id],
            |row| row.get(0),
        )
        .optional()?;
    let Some(original) = original else {
        return Ok(None);
    };

    let mut final_name = original.clone();
    let mut suffix = 2u32;
    loop {
        let taken: bool = tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM groups
                  WHERE user_id = ?1 AND name = ?2 AND deleted_at IS NULL)",
                params![user_id, final_name],
                |row| row.get::<_, i64>(0),
            )
            .map(|n| n != 0)?;
        if !taken {
            break;
        }
        // 防御性上限：同名分组不可能多到用尽 999 个后缀
        if suffix > 1000 {
            return Err(rusqlite::Error::InvalidQuery);
        }
        final_name = format!("{original} ({suffix})");
        suffix += 1;
    }

    let n = tx.execute(
        "UPDATE groups SET deleted_at = NULL, name = ?3
          WHERE id = ?1 AND user_id = ?2 AND deleted_at IS NOT NULL",
        params![id, user_id, final_name],
    )?;
    if n > 0 {
        tx.execute(
            "UPDATE tasks SET deleted_at = NULL WHERE group_id = ?1",
            params![id],
        )?;
    }
    tx.commit()?;
    Ok(Some(if final_name == original {
        None
    } else {
        Some(final_name)
    }))
}

/// 彻底删除分组及其下任务（物理删除，不可恢复；仅限该用户的分组）
pub fn purge_group(conn: &Connection, user_id: i64, id: i64) -> SqlResult<bool> {
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "DELETE FROM tasks WHERE group_id = ?1 AND group_id IN (SELECT id FROM groups WHERE user_id = ?2)",
        params![id, user_id],
    )?;
    let n = tx.execute(
        "DELETE FROM groups WHERE id = ?1 AND user_id = ?2",
        params![id, user_id],
    )?;
    tx.commit()?;
    Ok(n > 0)
}

/// 清空回收站：彻底删除该用户所有已删除的分组与任务
pub fn empty_trash(conn: &Connection, user_id: i64) -> SqlResult<()> {
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "DELETE FROM tasks WHERE deleted_at IS NOT NULL
         AND group_id IN (SELECT id FROM groups WHERE user_id = ?1)",
        params![user_id],
    )?;
    tx.execute(
        "DELETE FROM groups WHERE deleted_at IS NOT NULL AND user_id = ?1",
        params![user_id],
    )?;
    tx.commit()
}

/// 全部数据导出为 JSON 文档（仅该用户的数据）

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_conn;

    #[test]
    fn trash_flow() {
        let (c, admin) = test_conn();
        let gid = list_groups(&c, admin).unwrap()[0].id;
        let t = create_task(&c, admin, gid, "待删任务", "", None).unwrap();

        // 删除任务 → 回收站可见、列表与导出不可见
        assert!(delete_task(&c, admin, t.id).unwrap());
        assert!(list_tasks(&c, admin, None).unwrap().is_empty());
        let (_, tasks) = list_trash(&c, admin).unwrap();
        assert_eq!(tasks.len(), 1);
        assert!(tasks[0].deleted_at.is_some());
        let doc = export_all(&c, admin).unwrap();
        assert!(doc.groups[0].tasks.is_empty());

        // 恢复 → 回到列表
        assert!(restore_task(&c, admin, t.id).unwrap());
        assert_eq!(list_tasks(&c, admin, None).unwrap().len(), 1);
        assert!(list_trash(&c, admin).unwrap().1.is_empty());

        // 再删 → 彻底删除
        delete_task(&c, admin, t.id).unwrap();
        assert!(purge_task(&c, admin, t.id).unwrap());
        assert!(list_trash(&c, admin).unwrap().1.is_empty());

        // 分组软删级联 + 恢复
        let g = create_group(&c, admin, "回收组", "").unwrap();
        create_task(&c, admin, g.id, "组内任务", "", None).unwrap();
        assert!(delete_group(&c, admin, g.id).unwrap());
        let (groups, tasks) = list_trash(&c, admin).unwrap();
        assert_eq!(groups.len(), 1);
        assert_eq!(tasks.len(), 1);
        assert!(restore_group(&c, admin, g.id).unwrap().is_some());
        assert_eq!(list_groups(&c, admin).unwrap().len(), 2);
        assert_eq!(list_tasks(&c, admin, Some(g.id)).unwrap().len(), 1);

        // 分组在回收站时其任务再删除会怎样：恢复后任务仍在
        let g2 = create_group(&c, admin, "再删组", "").unwrap();
        delete_group(&c, admin, g2.id).unwrap();
        assert!(purge_group(&c, admin, g2.id).unwrap());
        assert!(list_trash(&c, admin).unwrap().0.iter().all(|x| x.id != g2.id));

        // 清空回收站
        delete_group(&c, admin, g.id).unwrap();
        empty_trash(&c, admin).unwrap();
        let (groups, tasks) = list_trash(&c, admin).unwrap();
        assert!(groups.is_empty());
        assert!(tasks.is_empty());
    }

    #[test]
    fn restore_group_name_conflict_renames() {
        let (c, admin) = test_conn();
        let g = create_group(&c, admin, "项目", "").unwrap();
        create_task(&c, admin, g.id, "任务A", "", None).unwrap();
        delete_group(&c, admin, g.id).unwrap();

        // 原名被新分组占用 → 恢复时自动重命名
        create_group(&c, admin, "项目", "").unwrap();
        let out = restore_group(&c, admin, g.id).unwrap().unwrap();
        assert_eq!(out, Some("项目 (2)".to_string()));

        let groups = list_groups(&c, admin).unwrap();
        assert!(groups.iter().any(|x| x.name == "项目"));
        let restored = groups.iter().find(|x| x.name == "项目 (2)").unwrap();
        // 组内任务随分组一起恢复
        assert_eq!(list_tasks(&c, admin, Some(restored.id)).unwrap().len(), 1);

        // 无冲突时原名恢复
        let g2 = create_group(&c, admin, "空组", "").unwrap();
        delete_group(&c, admin, g2.id).unwrap();
        assert_eq!(restore_group(&c, admin, g2.id).unwrap().unwrap(), None);

        // 不存在 / 非回收站项
        assert!(restore_group(&c, admin, 999).unwrap().is_none());
        assert!(restore_group(&c, admin, restored.id).unwrap().is_none());
    }
}
