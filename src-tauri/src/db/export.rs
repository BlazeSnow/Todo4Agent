use rusqlite::{Connection, Result as SqlResult};

use super::*;

pub fn export_all(conn: &Connection, user_id: Option<i64>) -> SqlResult<ExportDoc> {
    let groups = list_groups(conn, user_id)?;
    let mut out = Vec::with_capacity(groups.len());
    for g in &groups {
        let tasks = list_tasks(conn, user_id, Some(g.id))?;
        out.push(ExportGroup {
            name: g.name.clone(),
            tasks: tasks
                .into_iter()
                .map(|t| ExportTask {
                    title: t.title,
                    description: t.description,
                    status: t.status,
                    due_at: t.due_at,
                })
                .collect(),
        });
    }
    Ok(ExportDoc {
        version: 1,
        exported_at: now(),
        groups: out,
    })
}

// ---------- 导入 ----------

/// 导入结果统计
#[derive(Debug, Clone, Serialize)]
pub struct ImportResult {
    pub groups_created: usize,
    pub groups_merged: usize,
    pub tasks_imported: usize,
    pub tasks_skipped: usize,
}

/// 导入导出文档（仅导入到该用户）：同名分组并入（任务追加），新分组新建；任务全部新增
pub fn import_doc(conn: &Connection, user_id: Option<i64>, doc: &ExportDoc) -> SqlResult<ImportResult> {
    let mut result = ImportResult {
        groups_created: 0,
        groups_merged: 0,
        tasks_imported: 0,
        tasks_skipped: 0,
    };

    // 先收集现有分组名（含回收站中同名的也视为占用，避免 UNIQUE 冲突）
    let groups = list_groups(conn, user_id)?;
    let mut name_map: std::collections::HashMap<String, i64> = groups
        .iter()
        .map(|g| (g.name.clone(), g.id))
        .collect();

    for g in &doc.groups {
        let name = g.name.trim();
        if name.is_empty() {
            continue;
        }
        let group_id = match name_map.get(name) {
            Some(id) => {
                result.groups_merged += 1;
                *id
            }
            None => {
                let group = create_group(conn, user_id, name)?;
                result.groups_created += 1;
                name_map.insert(name.to_string(), group.id);
                group.id
            }
        };

        for t in &g.tasks {
            let title = t.title.trim();
            if title.is_empty() {
                result.tasks_skipped += 1;
                continue;
            }
            // 已完成任务导入后保持完成状态
            let task =
                create_task(conn, user_id, group_id, title, t.description.trim(), t.due_at.as_deref())?;
            if t.status == "done" {
                let _ = update_task(
                    conn,
                    user_id,
                    task.id,
                    &TaskUpdate {
                        status: Some("done".to_string()),
                        ..Default::default()
                    },
                );
            }
            result.tasks_imported += 1;
        }
    }
    Ok(result)
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_conn;

    #[test]
    fn import_doc_merge() {
        let (c, admin) = test_conn();
        // 预置数据：快速清单已存在（默认播种）
        let gid = list_groups(&c, Some(admin)).unwrap()[0].id;
        create_task(&c, Some(admin), gid, "原有任务", "", None).unwrap();

        let doc = ExportDoc {
            version: 1,
            exported_at: "2026-08-22T00:00:00Z".to_string(),
            groups: vec![
                ExportGroup {
                    name: DEFAULT_GROUP.to_string(), // 同名 → 并入快速清单
                    tasks: vec![
                        ExportTask {
                            title: "导入任务1".to_string(),
                            description: "说明".to_string(),
                            status: "done".to_string(),
                            due_at: Some("2026-09-01T00:00:00Z".to_string()),
                        },
                        ExportTask {
                            title: "导入任务2".to_string(),
                            description: String::new(),
                            status: "pending".to_string(),
                            due_at: None,
                        },
                        ExportTask {
                            title: "  ".to_string(), // 空标题 → 跳过
                            description: String::new(),
                            status: "pending".to_string(),
                            due_at: None,
                        },
                    ],
                },
                ExportGroup {
                    name: "新分组".to_string(), // 新名字 → 新建
                    tasks: vec![ExportTask {
                        title: "新组任务".to_string(),
                        description: String::new(),
                        status: "pending".to_string(),
                        due_at: None,
                    }],
                },
            ],
        };

        let r = import_doc(&c, Some(admin), &doc).unwrap();
        assert_eq!(r.groups_created, 1);
        assert_eq!(r.groups_merged, 1);
        assert_eq!(r.tasks_imported, 3);
        assert_eq!(r.tasks_skipped, 1);

        // 快速清单：原有 + 2 个导入任务，且导入任务保持 done 状态
        let tasks = list_tasks(&c, Some(admin), Some(gid)).unwrap();
        assert_eq!(tasks.len(), 3);
        assert!(tasks.iter().any(|t| t.title == "导入任务1" && t.status == "done"));

        // 新分组
        let groups = list_groups(&c, Some(admin)).unwrap();
        let new_g = groups.iter().find(|g| g.name == "新分组").unwrap();
        assert_eq!(list_tasks(&c, Some(admin), Some(new_g.id)).unwrap().len(), 1);
    }

    #[test]
    fn export_shape() {
        let (c, admin) = test_conn();
        let gid = list_groups(&c, Some(admin)).unwrap()[0].id;
        create_task(&c, Some(admin), gid, "A", "B", None).unwrap();
        let doc = export_all(&c, Some(admin)).unwrap();
        assert_eq!(doc.version, 1);
        assert_eq!(doc.groups.len(), 1);
        assert_eq!(doc.groups[0].name, DEFAULT_GROUP);
        assert_eq!(doc.groups[0].tasks.len(), 1);
        assert_eq!(doc.groups[0].tasks[0].title, "A");
    }
}
