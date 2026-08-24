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

/// 从回收站恢复任务（原分组已被删除时回落到系统分组「无分组」）；
/// 不存在、未删除或不属于该用户返回 Ok(false)
pub fn restore_task(conn: &Connection, user_id: i64, id: i64) -> SqlResult<bool> {
    let tx = conn.unchecked_transaction()?;
    let group: Option<(i64, Option<String>)> = tx
        .query_row(
            "SELECT t.group_id, g.deleted_at FROM tasks t
             JOIN groups g ON g.id = t.group_id
             WHERE t.id = ?1 AND t.deleted_at IS NOT NULL AND g.user_id = ?2",
            params![id, user_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    let Some((group_id, group_deleted_at)) = group else {
        return Ok(false);
    };
    let target = if group_deleted_at.is_some() {
        ensure_no_group(&tx, user_id)?
    } else {
        group_id
    };
    let n = tx.execute(
        "UPDATE tasks SET deleted_at = NULL, updated_at = ?1, group_id = ?2
         WHERE id = ?3 AND deleted_at IS NOT NULL",
        params![now(), target, id],
    )?;
    tx.commit()?;
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

/// 从回收站恢复分组（仅限该用户的回收站项）。
/// 新版删除流程下组内任务已在删除时移入「无分组」，恢复的是空分组；
/// 旧版数据中随组删除的任务仍随恢复还原。原名被该用户的现有分组占用时
/// 自动重命名为「原名 (2)」「原名 (3)」……
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

/// 彻底删除分组（物理删除，不可恢复；仅限该用户回收站中的分组）。
/// 新版删除流程下任务已随删除移入「无分组」，此处仅清理旧版数据遗留；
/// 系统分组「无分组」不可清理
pub fn purge_group(conn: &Connection, user_id: i64, id: i64) -> SqlResult<bool> {
    let tx = conn.unchecked_transaction()?;
    let name: Option<String> = tx
        .query_row(
            "SELECT name FROM groups WHERE id = ?1 AND user_id = ?2",
            params![id, user_id],
            |row| row.get(0),
        )
        .optional()?;
    let Some(name) = name else {
        return Ok(false);
    };
    if name == NO_GROUP {
        return Ok(false);
    }
    // 兼容旧版数据：仍存活（未删除）的任务移入「无分组」后，再清理组内已删除任务
    let no_group = ensure_no_group(&tx, user_id)?;
    tx.execute(
        "UPDATE tasks SET group_id = ?1 WHERE group_id = ?2 AND deleted_at IS NULL",
        params![no_group, id],
    )?;
    tx.execute("DELETE FROM tasks WHERE group_id = ?1", params![id])?;
    let n = tx.execute(
        "DELETE FROM groups WHERE id = ?1 AND user_id = ?2",
        params![id, user_id],
    )?;
    tx.commit()?;
    Ok(n > 0)
}

/// 清空回收站：彻底删除该用户所有已删除的分组与任务。
/// 兼容旧版数据：回收站分组下仍存活的任务移入「无分组」，不随分组清理
pub fn empty_trash(conn: &Connection, user_id: i64) -> SqlResult<()> {
    let tx = conn.unchecked_transaction()?;
    let no_group = ensure_no_group(&tx, user_id)?;
    tx.execute(
        "UPDATE tasks SET group_id = ?1
         WHERE deleted_at IS NULL
           AND group_id IN (SELECT id FROM groups WHERE user_id = ?2 AND deleted_at IS NOT NULL)",
        params![no_group, user_id],
    )?;
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

        // 分组软删：组内任务移入「无分组」，不进回收站；恢复得到空分组
        let no_group = list_groups(&c, admin).unwrap().iter().find(|g| g.name == NO_GROUP).unwrap().id;
        let g = create_group(&c, admin, "回收组", "").unwrap();
        create_task(&c, admin, g.id, "组内任务", "", None).unwrap();
        assert!(delete_group(&c, admin, g.id).unwrap());
        let (groups, tasks) = list_trash(&c, admin).unwrap();
        assert_eq!(groups.len(), 1);
        assert!(tasks.is_empty());
        assert_eq!(list_tasks(&c, admin, Some(no_group)).unwrap().len(), 1);
        assert!(restore_group(&c, admin, g.id).unwrap().is_some());
        assert_eq!(list_groups(&c, admin).unwrap().len(), 3);
        assert!(list_tasks(&c, admin, Some(g.id)).unwrap().is_empty());
        assert_eq!(list_tasks(&c, admin, Some(no_group)).unwrap().len(), 1);

        // 系统分组「无分组」不可删除、不可彻底删除
        assert!(!delete_group(&c, admin, no_group).unwrap());
        assert!(!purge_group(&c, admin, no_group).unwrap());

        // 彻底删除分组不影响已移入「无分组」的任务
        let g2 = create_group(&c, admin, "再删组", "").unwrap();
        delete_group(&c, admin, g2.id).unwrap();
        assert!(purge_group(&c, admin, g2.id).unwrap());
        assert!(list_trash(&c, admin).unwrap().0.iter().all(|x| x.id != g2.id));
        assert_eq!(list_tasks(&c, admin, Some(no_group)).unwrap().len(), 1);

        // 清空回收站
        delete_group(&c, admin, g.id).unwrap();
        empty_trash(&c, admin).unwrap();
        let (groups, tasks) = list_trash(&c, admin).unwrap();
        assert!(groups.is_empty());
        assert!(tasks.is_empty());
        assert_eq!(list_tasks(&c, admin, Some(no_group)).unwrap().len(), 1);
    }

    #[test]
    fn tasks_move_to_no_group_on_group_deletion() {
        let (c, admin) = test_conn();
        let no_group = list_groups(&c, admin).unwrap().iter().find(|g| g.name == NO_GROUP).unwrap().id;
        let g = create_group(&c, admin, "待删组", "").unwrap();
        let active = create_task(&c, admin, g.id, "进行中", "", None).unwrap();
        let archived = create_task(&c, admin, g.id, "已完成", "", None).unwrap();
        archive_task(&c, admin, archived.id).unwrap();

        // 删除分组：未归档与已归档任务都移入「无分组」，不进回收站、不丢失
        assert!(delete_group(&c, admin, g.id).unwrap());
        let (_, trash_tasks) = list_trash(&c, admin).unwrap();
        assert!(trash_tasks.is_empty());
        let active_list = list_tasks(&c, admin, Some(no_group)).unwrap();
        assert!(active_list.iter().any(|t| t.id == active.id));
        let arc = list_archived(&c, admin).unwrap();
        assert_eq!(arc.len(), 1);
        assert_eq!(arc[0].id, archived.id);
        assert_eq!(arc[0].group_id, no_group);

        // 取消归档：任务已在「无分组」存活，直接回到该分组
        assert!(unarchive_task(&c, admin, archived.id).unwrap());
        assert!(list_tasks(&c, admin, Some(no_group)).unwrap().iter().any(|t| t.id == archived.id));

        // 彻底删除分组与清空回收站均不影响「无分组」中的任务
        archive_task(&c, admin, archived.id).unwrap();
        assert!(purge_group(&c, admin, g.id).unwrap());
        let g2 = create_group(&c, admin, "清空组", "").unwrap();
        delete_group(&c, admin, g2.id).unwrap();
        empty_trash(&c, admin).unwrap();
        assert_eq!(list_archived(&c, admin).unwrap().len(), 1);
        assert_eq!(list_tasks(&c, admin, Some(no_group)).unwrap().len(), 1);

        // 系统分组不可改名
        assert!(rename_group(&c, admin, no_group, "改名").unwrap().is_none());
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
        // 组内任务在删除时已移入「无分组」，恢复得到空分组
        assert!(list_tasks(&c, admin, Some(restored.id)).unwrap().is_empty());
        let no_group = groups.iter().find(|x| x.name == NO_GROUP).unwrap().id;
        assert!(list_tasks(&c, admin, Some(no_group)).unwrap().iter().any(|t| t.title == "任务A"));

        // 无冲突时原名恢复
        let g2 = create_group(&c, admin, "空组", "").unwrap();
        delete_group(&c, admin, g2.id).unwrap();
        assert_eq!(restore_group(&c, admin, g2.id).unwrap().unwrap(), None);

        // 不存在 / 非回收站项
        assert!(restore_group(&c, admin, 999).unwrap().is_none());
        assert!(restore_group(&c, admin, restored.id).unwrap().is_none());
    }
}
