use rusqlite::Connection;
use serde_json::{json, Value};

use super::{tool_error, tool_result};
use crate::db;
use crate::lang::{tr, tr_a, Lang};

fn arg_str(args: &Value, key: &str, lang: Lang) -> Result<String, String> {
    match args.get(key) {
        Some(Value::String(s)) if !s.trim().is_empty() => Ok(s.trim().to_string()),
        _ => Err(tr_a(lang, "arg-error-required-nonempty", &[("key", key)])),
    }
}

fn arg_i64(args: &Value, key: &str, lang: Lang) -> Result<i64, String> {
    match args.get(key) {
        Some(Value::Number(n)) => n.as_i64().ok_or_else(|| arg_int_err(key, lang)),
        _ => Err(tr_a(lang, "arg-error-required", &[("key", key)])),
    }
}

fn arg_opt_i64(args: &Value, key: &str, lang: Lang) -> Result<Option<i64>, String> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Number(n)) => n.as_i64().map(Some).ok_or_else(|| arg_int_err(key, lang)),
        Some(_) => Err(arg_int_err(key, lang)),
    }
}

/// 可选字符串参数：缺省 / null / 空白均视为未提供（返回 None），其余返回 trim 后的值
fn arg_opt_str(args: &Value, key: &str, lang: Lang) -> Result<Option<String>, String> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => {
            let t = s.trim();
            Ok(if t.is_empty() { None } else { Some(t.to_string()) })
        }
        Some(_) => Err(tr_a(lang, "arg-error-string", &[("key", key)])),
    }
}

fn arg_int_err(key: &str, lang: Lang) -> String {
    tr_a(lang, "arg-error-int", &[("key", key)])
}

/// 分组已锁定时输出错误并返回 true（调用方据此返回）
fn group_locked(conn: &Connection, user_id: i64, group_id: i64, id: &Value, lang: Lang) -> bool {
    if let Ok(Some((name, true))) = db::group_lock_info(conn, user_id, group_id) {
        tool_error(id, lang.locked_err(&name));
        return true;
    }
    false
}

/// 任务所在分组已锁定时输出错误并返回 true（调用方据此返回）
fn task_locked(conn: &Connection, user_id: i64, task_id: i64, id: &Value, lang: Lang) -> bool {
    if let Ok(Some((_, name, true))) = db::task_group_lock(conn, user_id, task_id) {
        tool_error(id, lang.locked_err(&name));
        return true;
    }
    false
}

pub(super) struct ToolDef {
    pub name: &'static str,
    pub description: String,
    pub input_schema: Value,
}

impl ToolDef {
    fn new(name: &'static str, description: String, input_schema: Value) -> Self {
        Self {
            name,
            description,
            input_schema,
        }
    }
}

pub(super) fn tools(lang: Lang) -> Vec<ToolDef> {
    let d = |key: &str| tr(lang, key);
    vec![
        ToolDef::new(
            "app_version",
            d("tool-app-version"),
            json!({ "type": "object", "properties": {} }),
        ),
        ToolDef::new(
            "app_release",
            d("tool-app-release"),
            json!({ "type": "object", "properties": {} }),
        ),
        ToolDef::new(
            "db_path",
            d("tool-db-path"),
            json!({ "type": "object", "properties": {} }),
        ),
        ToolDef::new(
            "group_list",
            d("tool-group-list"),
            json!({ "type": "object", "properties": {} }),
        ),
        ToolDef::new(
            "group_create",
            d("tool-group-create"),
            json!({
                "type": "object",
                "properties": {
                    "name": { "type": "string", "description": d("tp-name-required") },
                    "description": { "type": "string", "description": d("tp-desc-purpose") }
                },
                "required": ["name"]
            }),
        ),
        ToolDef::new(
            "group_rename",
            d("tool-group-rename"),
            json!({
                "type": "object",
                "properties": {
                    "id": { "type": "integer", "description": d("tp-group-id") },
                    "name": { "type": "string", "description": d("tp-name-new") },
                    "description": { "type": "string", "description": d("tp-desc-update") }
                },
                "required": ["id", "name"]
            }),
        ),
        ToolDef::new(
            "group_delete",
            d("tool-group-delete"),
            json!({
                "type": "object",
                "properties": { "id": { "type": "integer", "description": d("tp-group-id-required") } },
                "required": ["id"]
            }),
        ),
        ToolDef::new(
            "task_list",
            d("tool-task-list"),
            json!({
                "type": "object",
                "properties": {
                    "group_id": { "type": "integer", "description": d("tp-group-id-optional-all") },
                    "include_archived": { "type": "boolean", "description": d("tp-include-archived") }
                }
            }),
        ),
        ToolDef::new(
            "task_create",
            d("tool-task-create"),
            json!({
                "type": "object",
                "properties": {
                    "group_id": { "type": "integer", "description": d("tp-owning-group-required") },
                    "title": { "type": "string", "description": d("tp-title-required") },
                    "description": { "type": "string", "description": d("tp-desc-optional") },
                    "due_at": { "type": "string", "description": d("tp-due-optional") }
                },
                "required": ["group_id", "title"]
            }),
        ),
        ToolDef::new(
            "task_update",
            d("tool-task-update"),
            json!({
                "type": "object",
                "properties": {
                    "id": { "type": "integer", "description": d("tp-task-id-required") },
                    "group_id": { "type": "integer", "description": d("tp-move-group-id") },
                    "title": { "type": "string", "description": d("tp-new-title") },
                    "description": { "type": "string", "description": d("tp-new-desc") },
                    "status": { "type": "string", "enum": ["pending", "done"], "description": d("tp-new-status") },
                    "due_at": { "type": ["string", "null"], "description": d("tp-new-due") }
                },
                "required": ["id"]
            }),
        ),
        ToolDef::new(
            "task_complete",
            d("tool-task-complete"),
            json!({
                "type": "object",
                "properties": {
                    "id": { "type": "integer", "description": d("tp-task-id-required") },
                    "done": { "type": "boolean", "description": d("tp-done") }
                },
                "required": ["id", "done"]
            }),
        ),
        ToolDef::new(
            "task_archive",
            d("tool-task-archive"),
            json!({
                "type": "object",
                "properties": { "id": { "type": "integer", "description": d("tp-task-id-required") } },
                "required": ["id"]
            }),
        ),
        ToolDef::new(
            "task_unarchive",
            d("tool-task-unarchive"),
            json!({
                "type": "object",
                "properties": { "id": { "type": "integer", "description": d("tp-task-id-required") } },
                "required": ["id"]
            }),
        ),
        ToolDef::new(
            "task_delete",
            d("tool-task-delete"),
            json!({
                "type": "object",
                "properties": { "id": { "type": "integer", "description": d("tp-task-id-required") } },
                "required": ["id"]
            }),
        ),
        ToolDef::new(
            "task_export",
            d("tool-task-export"),
            json!({ "type": "object", "properties": {} }),
        ),
        ToolDef::new(
            "task_import",
            d("tool-task-import"),
            json!({
                "type": "object",
                "properties": {
                    "doc": { "type": "object", "description": d("tp-doc") }
                },
                "required": ["doc"]
            }),
        ),
        ToolDef::new(
            "user_password",
            d("tool-user-password"),
            json!({
                "type": "object",
                "properties": {
                    "old_password": { "type": "string", "description": d("tp-old-password") },
                    "new_password": { "type": "string", "description": d("tp-new-password") }
                },
                "required": ["old_password", "new_password"]
            }),
        ),
        ToolDef::new(
            "prompt_get",
            d("tool-prompt-get"),
            json!({ "type": "object", "properties": {} }),
        ),
        ToolDef::new(
            "prompt_update",
            d("tool-prompt-update"),
            json!({
                "type": "object",
                "properties": {
                    "content": { "type": "string", "description": d("tp-content") }
                },
                "required": ["content"]
            }),
        ),
    ]
}

pub(super) fn call_tool(
    name: &str,
    args: &Value,
    conn: &Connection,
    user_id: i64,
    id: &Value,
    lang: Lang,
) {
    let db_err =
        |e: rusqlite::Error| tool_error(id, tr_a(lang, "db-error", &[("err", &e.to_string())]));
    let e = |key: &str| tr(lang, key);

    match name {
        "app_version" => tool_result(
            id,
            json!({ "name": "todo4agent", "version": env!("CARGO_PKG_VERSION") }).to_string(),
        ),

        "app_release" => tool_result(
            id,
            json!({
                "name": "todo4agent",
                "version": env!("CARGO_PKG_VERSION"),
                "release_url": "https://github.com/Todo4Agent/Todo4Agent/releases"
            })
            .to_string(),
        ),

        // 本 MCP 进程实际打开的数据库文件（含 TODO4AGENT_DB 环境变量覆盖）
        "db_path" => tool_result(id, json!({ "path": db::db_path() }).to_string()),

        "group_list" => match db::list_groups(conn, user_id) {
            Ok(v) => tool_result(id, json!(v).to_string()),
            Err(err) => db_err(err),
        },

        "group_create" => {
            let name = match arg_str(args, "name", lang) {
                Ok(v) => v,
                Err(m) => return tool_error(id, m),
            };
            let description = match arg_opt_str(args, "description", lang) {
                Ok(v) => v.unwrap_or_default(),
                Err(m) => return tool_error(id, m),
            };
            match db::create_group(conn, user_id, &name, &description) {
                Ok(g) => tool_result(id, json!(g).to_string()),
                Err(err) if db::is_unique_violation(&err) => {
                    tool_error(id, e("group-name-taken"))
                }
                Err(err) => db_err(err),
            }
        }

        "group_rename" => {
            let gid = match arg_i64(args, "id", lang) {
                Ok(v) => v,
                Err(m) => return tool_error(id, m),
            };
            let name = match arg_str(args, "name", lang) {
                Ok(v) => v,
                Err(m) => return tool_error(id, m),
            };
            // 先解析描述参数，避免改名成功后才发现参数非法
            let description = match args.get("description") {
                None => None,
                Some(v) => match v.as_str() {
                    Some(s) => Some(s.trim().to_string()),
                    None => {
                        return tool_error(
                            id,
                            tr_a(lang, "arg-error-string", &[("key", "description")]),
                        )
                    }
                },
            };
            if group_locked(conn, user_id, gid, id, lang) {
                return;
            }
            // 系统分组不可改名，先行拦截给出可读错误（db 层同样兜底）
            if let Ok(Some(g)) = db::get_group(conn, user_id, gid) {
                if g.name == db::NO_GROUP {
                    return tool_error(id, e("no-group-rename"));
                }
            }
            match db::rename_group(conn, user_id, gid, &name) {
                Ok(Some(_)) => {}
                Ok(None) => return tool_error(id, e("group-not-found")),
                Err(err) if db::is_unique_violation(&err) => {
                    tool_error(id, e("group-name-taken"))
                }
                Err(err) => return db_err(err),
            }
            if let Some(d) = description {
                match db::set_group_description(conn, user_id, gid, &d) {
                    Ok(_) => {}
                    Err(err) => return db_err(err),
                }
            }
            match db::get_group(conn, user_id, gid) {
                Ok(Some(g)) => tool_result(id, json!(g).to_string()),
                Ok(None) => tool_error(id, e("group-not-found")),
                Err(err) => db_err(err),
            }
        }

        "group_delete" => {
            let gid = match arg_i64(args, "id", lang) {
                Ok(v) => v,
                Err(m) => return tool_error(id, m),
            };
            if group_locked(conn, user_id, gid, id, lang) {
                return;
            }
            // 系统分组不可删除，先行拦截给出可读错误（db 层同样兜底）
            if let Ok(Some(g)) = db::get_group(conn, user_id, gid) {
                if g.name == db::NO_GROUP {
                    return tool_error(id, e("no-group-delete"));
                }
            }
            match db::delete_group(conn, user_id, gid) {
                Ok(true) => tool_result(id, json!({ "ok": true }).to_string()),
                Ok(false) => tool_error(id, e("group-not-found")),
                Err(err) => db_err(err),
            }
        }

        "task_list" => {
            let gid = match arg_opt_i64(args, "group_id", lang) {
                Ok(v) => v,
                Err(m) => return tool_error(id, m),
            };
            let include_archived = match args.get("include_archived") {
                None | Some(Value::Null) | Some(Value::Bool(false)) => false,
                Some(Value::Bool(true)) => true,
                Some(_) => {
                    return tool_error(
                        id,
                        tr_a(lang, "arg-error-bool", &[("key", "include_archived")]),
                    )
                }
            };
            let result = match db::list_tasks(conn, user_id, gid) {
                Ok(mut v) => {
                    // 包含归档时追加归档任务（按归档时间倒序）
                    if include_archived {
                        match db::list_archived(conn, user_id) {
                            Ok(a) => v.extend(a),
                            Err(err) => return db_err(err),
                        }
                    }
                    Ok(v)
                }
                Err(err) => Err(err),
            };
            match result {
                Ok(v) => tool_result(id, json!(v).to_string()),
                Err(err) => db_err(err),
            }
        }

        "task_create" => {
            let gid = match arg_i64(args, "group_id", lang) {
                Ok(v) => v,
                Err(m) => return tool_error(id, m),
            };
            if group_locked(conn, user_id, gid, id, lang) {
                return;
            }
            let title = match arg_str(args, "title", lang) {
                Ok(v) => v,
                Err(m) => return tool_error(id, m),
            };
            let description = args
                .get("description")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let due_at = args.get("due_at").and_then(Value::as_str).map(String::from);
            match db::create_task(conn, user_id, gid, &title, &description, due_at.as_deref()) {
                Ok(task) => tool_result(id, json!(task).to_string()),
                // create_task 预检查分组归属，分组缺失时返回 QueryReturnedNoRows
                Err(rusqlite::Error::QueryReturnedNoRows) => {
                    tool_error(id, e("group-not-found"))
                }
                Err(err) => db_err(err),
            }
        }

        "task_update" => {
            let tid = match arg_i64(args, "id", lang) {
                Ok(v) => v,
                Err(m) => return tool_error(id, m),
            };
            if task_locked(conn, user_id, tid, id, lang) {
                return;
            }
            // 移入锁定清单同样拒绝
            if let Some(target) = args.get("group_id").and_then(Value::as_i64) {
                if group_locked(conn, user_id, target, id, lang) {
                    return;
                }
            }
            let mut patch = db::TaskUpdate::default();
            if let Some(v) = args.get("title") {
                match v.as_str() {
                    Some(s) if !s.trim().is_empty() => patch.title = Some(s.trim().to_string()),
                    _ => return tool_error(id, tr(lang, "arg-error-title-empty")),
                }
            }
            if let Some(v) = args.get("description") {
                match v.as_str() {
                    Some(s) => patch.description = Some(s.to_string()),
                    None => {
                        return tool_error(
                            id,
                            tr_a(lang, "arg-error-string", &[("key", "description")]),
                        )
                    }
                }
            }
            if let Some(v) = args.get("status") {
                match v.as_str() {
                    Some(s @ ("pending" | "done")) => patch.status = Some(s.to_string()),
                    _ => return tool_error(id, tr(lang, "arg-error-status")),
                }
            }
            if let Some(v) = args.get("group_id") {
                match v.as_i64() {
                    Some(n) => patch.group_id = Some(n),
                    None => return tool_error(id, arg_int_err("group_id", lang)),
                }
            }
            match args.get("due_at") {
                None => {}
                Some(Value::Null) => patch.due_at = Some(None),
                Some(Value::String(s)) => patch.due_at = Some(Some(s.clone())),
                Some(_) => return tool_error(id, tr(lang, "arg-error-due")),
            }
            match db::update_task(conn, user_id, tid, &patch) {
                Ok(Some(task)) => tool_result(id, json!(task).to_string()),
                Ok(None) => tool_error(id, e("task-not-found")),
                Err(err) => db_err(err),
            }
        }

        "task_complete" => {
            let tid = match arg_i64(args, "id", lang) {
                Ok(v) => v,
                Err(m) => return tool_error(id, m),
            };
            if task_locked(conn, user_id, tid, id, lang) {
                return;
            }
            let done = match args.get("done") {
                Some(Value::Bool(b)) => *b,
                _ => return tool_error(id, tr(lang, "arg-error-done")),
            };
            let patch = db::TaskUpdate {
                status: Some(if done { "done" } else { "pending" }.to_string()),
                ..Default::default()
            };
            match db::update_task(conn, user_id, tid, &patch) {
                Ok(Some(task)) => tool_result(id, json!(task).to_string()),
                Ok(None) => tool_error(id, e("task-not-found")),
                Err(err) => db_err(err),
            }
        }

        "task_archive" => {
            let tid = match arg_i64(args, "id", lang) {
                Ok(v) => v,
                Err(m) => return tool_error(id, m),
            };
            if task_locked(conn, user_id, tid, id, lang) {
                return;
            }
            match db::archive_task(conn, user_id, tid) {
                Ok(true) => tool_result(id, json!({ "ok": true }).to_string()),
                Ok(false) => tool_error(id, e("task-not-found-or-archived")),
                Err(err) => db_err(err),
            }
        }

        "task_unarchive" => {
            let tid = match arg_i64(args, "id", lang) {
                Ok(v) => v,
                Err(m) => return tool_error(id, m),
            };
            if task_locked(conn, user_id, tid, id, lang) {
                return;
            }
            match db::unarchive_task(conn, user_id, tid) {
                Ok(true) => tool_result(id, json!({ "ok": true }).to_string()),
                Ok(false) => tool_error(id, e("task-not-archived")),
                Err(err) => db_err(err),
            }
        }

        "task_delete" => {
            let tid = match arg_i64(args, "id", lang) {
                Ok(v) => v,
                Err(m) => return tool_error(id, m),
            };
            if task_locked(conn, user_id, tid, id, lang) {
                return;
            }
            match db::delete_task(conn, user_id, tid) {
                Ok(true) => tool_result(id, json!({ "ok": true }).to_string()),
                Ok(false) => tool_error(id, e("task-not-found")),
                Err(err) => db_err(err),
            }
        }

        "task_export" => match db::export_all(conn, user_id) {
            Ok(doc) => tool_result(id, serde_json::to_string(&doc).unwrap_or_else(|err| err.to_string())),
            Err(err) => db_err(err),
        },

        "task_import" => {
            let doc: db::ExportDoc = match args.get("doc").cloned().map(serde_json::from_value) {
                Some(Ok(d)) => d,
                Some(Err(err)) => {
                    return tool_error(
                        id,
                        tr_a(lang, "import-doc-invalid", &[("err", &err.to_string())]),
                    )
                }
                None => return tool_error(id, tr_a(lang, "arg-error-required", &[("key", "doc")])),
            };
            if doc.groups.is_empty() {
                return tool_error(id, e("import-empty"));
            }
            // 文档包含已锁定清单时整体拒绝（用户可在界面导入或先解锁）
            let locked_names = match db::locked_group_names(conn, user_id) {
                Ok(v) => v,
                Err(err) => return db_err(err),
            };
            let conflicts: Vec<String> = doc
                .groups
                .iter()
                .map(|g| g.name.trim())
                .filter(|n| !n.is_empty() && locked_names.iter().any(|l| l == n))
                .map(String::from)
                .collect();
            if !conflicts.is_empty() {
                return tool_error(id, lang.import_locked(&conflicts));
            }
            match db::import_doc(conn, user_id, &doc) {
                Ok(r) => tool_result(id, json!(r).to_string()),
                Err(err) => db_err(err),
            }
        }

        "user_password" => {
            // 密码不做 trim：与 HTTP 接口一致，按原样校验
            let old = match args.get("old_password").and_then(Value::as_str) {
                Some(v) => v.to_string(),
                None => {
                    return tool_error(
                        id,
                        tr_a(lang, "arg-error-required-string", &[("key", "old_password")]),
                    )
                }
            };
            let new = match args.get("new_password").and_then(Value::as_str) {
                Some(v) => v.to_string(),
                None => {
                    return tool_error(
                        id,
                        tr_a(lang, "arg-error-required-string", &[("key", "new_password")]),
                    )
                }
            };
            if new.len() < 4 {
                return tool_error(id, tr(lang, "arg-error-new-password-short"));
            }
            match db::change_user_password(conn, user_id, &old, &new) {
                Ok(true) => {
                    // 与界面改密一致：吊销该用户全部已登录会话
                    // （MCP 自身用环境变量凭据，当前连接不受影响）
                    let _ = db::delete_user_sessions(conn, user_id, None);
                    tool_result(
                        id,
                        json!({ "ok": true, "note": tr(lang, "password-changed-note") }).to_string(),
                    )
                }
                Ok(false) => tool_error(id, e("wrong-password")),
                Err(err) => db_err(err),
            }
        }

        "prompt_get" => match db::get_custom_prompt(conn, user_id) {
            Ok(Some((content, updated_at))) => tool_result(
                id,
                json!({ "content": content, "is_default": false, "updated_at": updated_at }).to_string(),
            ),
            Ok(None) => tool_result(
                id,
                json!({ "content": "", "is_default": true, "updated_at": null }).to_string(),
            ),
            Err(err) => db_err(err),
        },

        "prompt_update" => {
            let content = match args.get("content").and_then(Value::as_str) {
                Some(s) => s.to_string(),
                None => {
                    return tool_error(
                        id,
                        tr_a(lang, "arg-error-required-string", &[("key", "content")]),
                    )
                }
            };
            match db::set_prompt(conn, user_id, &content) {
                Ok((is_default, updated_at)) => tool_result(
                    id,
                    json!({ "ok": true, "is_default": is_default, "updated_at": updated_at }).to_string(),
                ),
                Err(err) => db_err(err),
            }
        }

        _ => tool_error(id, tr_a(lang, "unknown-tool", &[("name", name)])),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_version_tool_defined() {
        assert!(tools(Lang::Zh).iter().any(|t| t.name == "app_version"));
        assert!(tools(Lang::Zh).iter().any(|t| t.name == "app_release"));
        assert!(tools(Lang::Zh).iter().any(|t| t.name == "db_path"));
    }

    #[test]
    fn new_tools_defined() {
        let names: Vec<&str> = tools(Lang::Zh).iter().map(|t| t.name).collect();
        assert!(names.contains(&"group_delete"));
        assert!(names.contains(&"task_import"));
        assert!(names.contains(&"task_archive"));
        assert!(names.contains(&"task_unarchive"));
        assert!(names.contains(&"user_password"));
        assert!(names.contains(&"prompt_get"));
        assert!(names.contains(&"prompt_update"));
    }

    #[test]
    fn tool_descriptions_follow_language() {
        let zh: Vec<String> = tools(Lang::Zh).iter().map(|t| t.description.clone()).collect();
        assert!(zh.iter().any(|d| d.contains("列出所有任务分组")));
        let en: Vec<String> = tools(Lang::En).iter().map(|t| t.description.clone()).collect();
        assert!(en.iter().any(|d| d.contains("List all task groups")));
        // 名称（协议契约）不随语言变化
        assert_eq!(
            tools(Lang::Zh).iter().map(|t| t.name).collect::<Vec<_>>(),
            tools(Lang::En).iter().map(|t| t.name).collect::<Vec<_>>(),
        );
    }
}
