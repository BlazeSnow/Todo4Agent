use rusqlite::{Connection, Result as SqlResult};

use super::*;

pub fn export_all(conn: &Connection, user_id: i64) -> SqlResult<ExportDoc> {
    let groups = list_groups(conn, user_id)?;
    let mut out = Vec::with_capacity(groups.len());
    for g in &groups {
        let tasks = list_tasks(conn, user_id, Some(g.id))?;
        out.push(ExportGroup {
            name: g.name.clone(),
            description: g.description.clone(),
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
    // 提示词一并导出；默认空状态导出为 null（导入端跳过）
    let prompt = get_custom_prompt(conn, user_id).map(|p| p.map(|(content, _)| content))?;
    Ok(ExportDoc {
        version: 1,
        exported_at: now(),
        prompt,
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
    /// 是否导入/更新了提示词（文档含 prompt 字段时）
    pub prompt_imported: bool,
}

/// 导入导出文档（仅导入到该用户）：同名分组并入（任务追加），新分组新建；任务全部新增；
/// 文档含 prompt 字段时提示词一并导入（空白视为清空），不含则保持现状
pub fn import_doc(conn: &Connection, user_id: i64, doc: &ExportDoc) -> SqlResult<ImportResult> {
    let mut result = ImportResult {
        groups_created: 0,
        groups_merged: 0,
        tasks_imported: 0,
        tasks_skipped: 0,
        prompt_imported: false,
    };

    if let Some(prompt) = &doc.prompt {
        set_prompt(conn, user_id, prompt)?;
        result.prompt_imported = true;
    }

    // 先收集现有分组名（当前用户未删除的分组；同名即并入）
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
                // 并入时文档带非空描述则覆盖现有描述（与任务追加语义一致）
                let desc = g.description.trim();
                if !desc.is_empty() {
                    set_group_description(conn, user_id, *id, desc)?;
                }
                result.groups_merged += 1;
                *id
            }
            None => {
                let group = create_group(conn, user_id, name, g.description.trim())?;
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
        let gid = list_groups(&c, admin).unwrap()[0].id;
        create_task(&c, admin, gid, "原有任务", "", None).unwrap();

        let doc = ExportDoc {
            version: 1,
            exported_at: "2026-08-22T00:00:00Z".to_string(),
            prompt: None,
            groups: vec![
                ExportGroup {
                    name: DEFAULT_GROUP.to_string(), // 同名 → 并入快速清单
                    description: "导入的描述".to_string(),
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
                    description: String::new(),
                    tasks: vec![ExportTask {
                        title: "新组任务".to_string(),
                        description: String::new(),
                        status: "pending".to_string(),
                        due_at: None,
                    }],
                },
            ],
        };

        let r = import_doc(&c, admin, &doc).unwrap();
        assert_eq!(r.groups_created, 1);
        assert_eq!(r.groups_merged, 1);
        assert_eq!(r.tasks_imported, 3);
        assert_eq!(r.tasks_skipped, 1);
        assert!(!r.prompt_imported); // 文档不含 prompt 字段

        // 快速清单：原有 + 2 个导入任务，且导入任务保持 done 状态
        let tasks = list_tasks(&c, admin, Some(gid)).unwrap();
        assert_eq!(tasks.len(), 3);
        assert!(tasks.iter().any(|t| t.title == "导入任务1" && t.status == "done"));

        // 并入的分组：文档带非空描述 → 覆盖
        let merged = get_group(&c, admin, gid).unwrap().unwrap();
        assert_eq!(merged.description, "导入的描述");

        // 新分组：文档描述为空 → 保持空
        let groups = list_groups(&c, admin).unwrap();
        let new_g = groups.iter().find(|g| g.name == "新分组").unwrap();
        assert_eq!(list_tasks(&c, admin, Some(new_g.id)).unwrap().len(), 1);
        assert_eq!(new_g.description, "");
    }

    #[test]
    fn export_shape() {
        let (c, admin) = test_conn();
        let gid = list_groups(&c, admin).unwrap()[0].id;
        create_task(&c, admin, gid, "A", "B", None).unwrap();
        let doc = export_all(&c, admin).unwrap();
        assert_eq!(doc.version, 1);
        // 默认分组与系统分组「无分组」
        assert_eq!(doc.groups.len(), 2);
        assert_eq!(doc.groups[0].name, DEFAULT_GROUP);
        assert_eq!(doc.groups[1].name, NO_GROUP);
        assert_eq!(doc.groups[0].tasks.len(), 1);
        assert_eq!(doc.groups[0].tasks[0].title, "A");
        // 未设置提示词 → 导出为 None
        assert!(doc.prompt.is_none());
    }

    #[test]
    fn prompt_exported_and_imported() {
        let (c, admin) = test_conn();
        set_prompt(&c, admin, "规范V1").unwrap();
        let doc = export_all(&c, admin).unwrap();
        assert_eq!(doc.prompt.as_deref(), Some("规范V1"));

        // 导入到新用户：提示词随文档一并迁移
        let other = create_user(&c, "dave", "pass1234").unwrap();
        let r = import_doc(&c, other.id, &doc).unwrap();
        assert!(r.prompt_imported);
        assert_eq!(get_custom_prompt(&c, other.id).unwrap().unwrap().0, "规范V1");

        // 旧版文档（无 prompt 字段）：提示词保持现状
        let mut legacy = doc.clone();
        legacy.prompt = None;
        set_prompt(&c, other.id, "本地修改").unwrap();
        let r = import_doc(&c, other.id, &legacy).unwrap();
        assert!(!r.prompt_imported);
        assert_eq!(get_custom_prompt(&c, other.id).unwrap().unwrap().0, "本地修改");

        // prompt 为空白 → 导入即清空
        let mut clearing = doc.clone();
        clearing.prompt = Some("   ".to_string());
        let r = import_doc(&c, other.id, &clearing).unwrap();
        assert!(r.prompt_imported);
        assert!(get_custom_prompt(&c, other.id).unwrap().is_none());
    }
}
