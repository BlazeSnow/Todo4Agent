use rusqlite::{params, Connection, OptionalExtension, Result as SqlResult};

use super::*;

pub fn list_tasks(
    conn: &Connection,
    user_id: i64,
    group_id: Option<i64>,
) -> SqlResult<Vec<Task>> {
    // 默认按手动排序序号，再按 id（创建先后）稳定排序；不含已删除与已归档任务；仅本用户分组下的任务
    let sql = match group_id {
        Some(_) => format!(
            "SELECT {TASK_COLS} FROM tasks WHERE group_id = ?1 AND deleted_at IS NULL AND archived_at IS NULL
             AND group_id IN (SELECT id FROM groups WHERE user_id = ?2)
             ORDER BY sort_order, id"
        ),
        None => format!(
            "SELECT {TASK_COLS} FROM tasks WHERE deleted_at IS NULL AND archived_at IS NULL
             AND group_id IN (SELECT id FROM groups WHERE user_id = ?1)
             ORDER BY sort_order, id"
        ),
    };
    let mut stmt = conn.prepare(&sql)?;
    let rows = match group_id {
        Some(gid) => stmt.query_map(params![gid, user_id], task_from_row)?,
        None => stmt.query_map(params![user_id], task_from_row)?,
    };
    rows.collect()
}

// ---------- 归档 ----------

/// 归档列表：该用户已归档且未删除的任务，按归档时间倒序（时间线展示）
pub fn list_archived(conn: &Connection, user_id: i64) -> SqlResult<Vec<Task>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {TASK_COLS} FROM tasks WHERE archived_at IS NOT NULL AND deleted_at IS NULL
         AND group_id IN (SELECT id FROM groups WHERE user_id = ?1)
         ORDER BY archived_at DESC, id DESC"
    ))?;
    let rows = stmt.query_map(params![user_id], task_from_row)?;
    rows.collect()
}

/// 归档任务（从清单移入归档）；不存在、已归档、已删除或不属于该用户返回 Ok(false)
pub fn archive_task(conn: &Connection, user_id: i64, id: i64) -> SqlResult<bool> {
    let n = conn.execute(
        "UPDATE tasks SET archived_at = ?1, updated_at = ?1
         WHERE id = ?2 AND deleted_at IS NULL AND archived_at IS NULL
         AND group_id IN (SELECT id FROM groups WHERE user_id = ?3)",
        params![now(), id, user_id],
    )?;
    Ok(n > 0)
}

/// 取消归档（回到原清单；原分组已被删除时回落到系统分组「无分组」，避免任务落入侧边栏不可见的位置）；
/// 不存在、未归档、已删除或不属于该用户返回 Ok(false)
pub fn unarchive_task(conn: &Connection, user_id: i64, id: i64) -> SqlResult<bool> {
    let tx = conn.unchecked_transaction()?;
    let group: Option<(i64, Option<String>)> = tx
        .query_row(
            "SELECT t.group_id, g.deleted_at FROM tasks t
             JOIN groups g ON g.id = t.group_id
             WHERE t.id = ?1 AND t.deleted_at IS NULL AND t.archived_at IS NOT NULL
               AND g.user_id = ?2",
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
        "UPDATE tasks SET archived_at = NULL, updated_at = ?1, group_id = ?2
         WHERE id = ?3 AND archived_at IS NOT NULL AND deleted_at IS NULL",
        params![now(), target, id],
    )?;
    tx.commit()?;
    Ok(n > 0)
}

pub fn create_task(
    conn: &Connection,
    user_id: i64,
    group_id: i64,
    title: &str,
    description: &str,
    due_at: Option<&str>,
) -> SqlResult<Task> {
    // 分组必须属于当前用户
    if !group_owned_by(conn, user_id, group_id)? {
        return Err(rusqlite::Error::QueryReturnedNoRows);
    }
    let ts = now();
    conn.execute(
        "INSERT INTO tasks (group_id, title, description, status, due_at, created_at, updated_at)
         VALUES (?1, ?2, ?3, 'pending', ?4, ?5, ?5)",
        params![group_id, title, description, due_at, ts],
    )?;
    Ok(Task {
        id: conn.last_insert_rowid(),
        group_id,
        title: title.to_string(),
        description: description.to_string(),
        status: "pending".to_string(),
        due_at: due_at.map(String::from),
        created_at: ts.clone(),
        updated_at: ts,
        sort_order: 0,
        deleted_at: None,
        archived_at: None,
    })
}

/// 按给定顺序重排某分组内的任务（task_ids 中任务的 sort_order 依次赋 0,1,2,...）
/// 调用方需持锁独占访问（如 api 层 Mutex<Connection>），故使用 unchecked_transaction
pub fn reorder_tasks(
    conn: &Connection,
    user_id: i64,
    group_id: i64,
    task_ids: &[i64],
) -> SqlResult<()> {
    if !group_owned_by(conn, user_id, group_id)? {
        return Err(rusqlite::Error::QueryReturnedNoRows);
    }
    let tx = conn.unchecked_transaction()?;
    {
        let mut stmt = tx.prepare(
            "UPDATE tasks SET sort_order = ?1, updated_at = ?2 WHERE id = ?3 AND group_id = ?4",
        )?;
        for (i, tid) in task_ids.iter().enumerate() {
            stmt.execute(params![i as i64, now(), tid, group_id])?;
        }
    }
    tx.commit()
}

/// 局部更新任务；任务不存在、不属于该用户返回 Ok(None)
pub fn update_task(
    conn: &Connection,
    user_id: i64,
    id: i64,
    patch: &TaskUpdate,
) -> SqlResult<Option<Task>> {
    let mut sets: Vec<&str> = Vec::new();
    let mut vals: Vec<rusqlite::types::Value> = Vec::new();

    if let Some(v) = &patch.title {
        sets.push("title = ?");
        vals.push(v.clone().into());
    }
    if let Some(v) = &patch.description {
        sets.push("description = ?");
        vals.push(v.clone().into());
    }
    if let Some(v) = &patch.status {
        sets.push("status = ?");
        vals.push(v.clone().into());
    }
    if let Some(v) = &patch.group_id {
        sets.push("group_id = ?");
        vals.push((*v).into());
    }
    match &patch.due_at {
        Some(Some(v)) => {
            sets.push("due_at = ?");
            vals.push(v.clone().into());
        }
        Some(None) => sets.push("due_at = NULL"),
        None => {}
    }
    sets.push("updated_at = ?");
    vals.push(now().into());

    // 全部使用匿名占位符：SET 依次绑定 vals，最后绑定 id 与 user_id
    // （WHERE 中的 user_id 与 id 交叉引用，故显式保持顺序）
    let sql = format!(
        "UPDATE tasks SET {} WHERE id = ? RETURNING {TASK_COLS}",
        sets.join(", ")
    );
    // 先按 id 更新并检查归属：归属检查通过子查询
    let user_ok = conn.query_row(
        "SELECT 1 FROM tasks WHERE id = ?1 AND deleted_at IS NULL
         AND group_id IN (SELECT id FROM groups WHERE user_id = ?2)",
        params![id, user_id],
        |_| Ok(()),
    ).optional()?.is_some();
    if !user_ok {
        return Ok(None);
    }
    vals.push(id.into());
    let row = conn
        .query_row(&sql, rusqlite::params_from_iter(vals.iter()), task_from_row)
        .optional()?;
    Ok(row)
}

/// 软删除任务（进入回收站）；不存在、已删除或不属于该用户返回 Ok(false)
pub fn delete_task(conn: &Connection, user_id: i64, id: i64) -> SqlResult<bool> {
    let n = conn.execute(
        "UPDATE tasks SET deleted_at = ?1, updated_at = ?1 WHERE id = ?2 AND deleted_at IS NULL
         AND group_id IN (SELECT id FROM groups WHERE user_id = ?3)",
        params![now(), id, user_id],
    )?;
    Ok(n > 0)
}

/// 任务所属分组的锁定信息：返回 (group_id, 分组名, 是否锁定)；
/// 任务不存在、已删除或不属于该用户返回 Ok(None)（清单锁定的 MCP 拦截用）
pub fn task_group_lock(
    conn: &Connection,
    user_id: i64,
    task_id: i64,
) -> SqlResult<Option<(i64, String, bool)>> {
    conn.query_row(
        "SELECT g.id, g.name, g.locked FROM tasks t
         JOIN groups g ON g.id = t.group_id
         WHERE t.id = ?1 AND t.deleted_at IS NULL AND g.user_id = ?2",
        params![task_id, user_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )
    .optional()
}

// ---------- 回收站 ----------

/// 回收站内容：该用户已删除的分组与任务（按删除时间倒序）

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_conn;

    #[test]
    fn task_crud() {
        let (c, admin) = test_conn();
        let gid = list_groups(&c, admin).unwrap()[0].id;
        let t = create_task(&c, admin, gid, "写文档", "详细说明", Some("2026-09-01T00:00:00Z")).unwrap();
        assert_eq!(t.status, "pending");
        assert_eq!(list_tasks(&c, admin, Some(gid)).unwrap().len(), 1);

        let t2 = update_task(
            &c,
            admin,
            t.id,
            &TaskUpdate {
                title: Some("改标题".into()),
                status: Some("done".into()),
                ..Default::default()
            },
        )
        .unwrap()
        .unwrap();
        assert_eq!(t2.title, "改标题");
        assert_eq!(t2.status, "done");

        let cleared = update_task(
            &c,
            admin,
            t.id,
            &TaskUpdate {
                due_at: Some(None),
                ..Default::default()
            },
        )
        .unwrap()
        .unwrap();
        assert!(cleared.due_at.is_none());

        assert!(update_task(&c, admin, 999, &TaskUpdate::default()).unwrap().is_none());
        assert!(delete_task(&c, admin, t.id).unwrap());
        assert!(!delete_task(&c, admin, t.id).unwrap());
    }

    #[test]
    fn reorder_tasks_order() {
        let (c, admin) = test_conn();
        let gid = list_groups(&c, admin).unwrap()[0].id;
        let t1 = create_task(&c, admin, gid, "A", "", None).unwrap();
        let t2 = create_task(&c, admin, gid, "B", "", None).unwrap();
        let t3 = create_task(&c, admin, gid, "C", "", None).unwrap();
        assert_eq!(list_tasks(&c, admin, Some(gid)).unwrap().len(), 3);

        reorder_tasks(&c, admin, gid, &[t3.id, t1.id, t2.id]).unwrap();
        let ids: Vec<i64> = list_tasks(&c, admin, Some(gid))
            .unwrap()
            .iter()
            .map(|t| t.id)
            .collect();
        assert_eq!(ids, vec![t3.id, t1.id, t2.id]);

        // 不影响其他分组
        reorder_tasks(&c, admin, gid, &[t2.id, t3.id, t1.id]).unwrap();
        let ids: Vec<i64> = list_tasks(&c, admin, Some(gid))
            .unwrap()
            .iter()
            .map(|t| t.id)
            .collect();
        assert_eq!(ids, vec![t2.id, t3.id, t1.id]);
    }

    #[test]
    fn archive_roundtrip() {
        let (c, admin) = test_conn();
        let other = create_user(&c, "erin", "pass1234").unwrap();
        let gid = list_groups(&c, admin).unwrap()[0].id;
        let t1 = create_task(&c, admin, gid, "完成的任务", "", None).unwrap();
        let t2 = create_task(&c, admin, gid, "另一个", "", None).unwrap();

        // 归档后从清单消失、进入归档列表
        assert!(archive_task(&c, admin, t1.id).unwrap());
        let active = list_tasks(&c, admin, Some(gid)).unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].id, t2.id);
        let archived = list_archived(&c, admin).unwrap();
        assert_eq!(archived.len(), 1);
        assert_eq!(archived[0].id, t1.id);
        assert!(archived[0].archived_at.is_some());
        // 重复归档 / 他人归档均无效
        assert!(!archive_task(&c, admin, t1.id).unwrap());
        assert!(!archive_task(&c, other.id, t2.id).unwrap());

        // 取消归档回到清单
        assert!(unarchive_task(&c, admin, t1.id).unwrap());
        assert!(list_archived(&c, admin).unwrap().is_empty());
        assert_eq!(list_tasks(&c, admin, Some(gid)).unwrap().len(), 2);
        assert!(!unarchive_task(&c, admin, t1.id).unwrap());

        // 归档后删除进回收站（不再出现在归档），恢复后回到归档态
        archive_task(&c, admin, t1.id).unwrap();
        assert!(delete_task(&c, admin, t1.id).unwrap());
        assert!(list_archived(&c, admin).unwrap().is_empty());
        restore_task(&c, admin, t1.id).unwrap();
        let archived = list_archived(&c, admin).unwrap();
        assert_eq!(archived.len(), 1);
        assert_eq!(archived[0].id, t1.id);
    }
}
