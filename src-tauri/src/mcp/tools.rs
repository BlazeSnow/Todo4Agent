use rusqlite::Connection;
use serde_json::{json, Value};

use super::{tool_error, tool_result};


fn arg_str(args: &Value, key: &str, lang: Lang) -> Result<String, String> {
    match args.get(key) {
        Some(Value::String(s)) if !s.trim().is_empty() => Ok(s.trim().to_string()),
        _ => Err(format!(
            "{}: {key} {}",
            t(lang, "参数错误", "Invalid argument"),
            t(lang, "必填且不能为空", "is required and cannot be empty")
        )),
    }
}

fn arg_i64(args: &Value, key: &str, lang: Lang) -> Result<i64, String> {
    match args.get(key) {
        Some(Value::Number(n)) => n.as_i64().ok_or_else(|| arg_int_err(key, lang)),
        _ => Err(arg_req_err(key, lang)),
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
        Some(_) => Err(format!(
            "{}: {key} {}",
            t(lang, "参数错误", "Invalid argument"),
            t(lang, "必须是字符串", "must be a string")
        )),
    }
}

fn arg_int_err(key: &str, lang: Lang) -> String {
    format!(
        "{}: {key} {}",
        t(lang, "参数错误", "Invalid argument"),
        t(lang, "必须是整数", "must be an integer")
    )
}

fn arg_req_err(key: &str, lang: Lang) -> String {
    format!(
        "{}: {key} {}",
        t(lang, "参数错误", "Invalid argument"),
        t(lang, "必填", "is required")
    )
}



use crate::db;
use crate::lang::{t, Lang};

/// 清单（分组）锁定提示：锁定后 Agent 无法编辑该清单，界面编辑不受影响
fn locked_err(name: &str, lang: Lang) -> String {
    match lang {
        Lang::Zh => format!("清单「{name}」已锁定，Agent 无法编辑（请让用户在界面侧边栏分组菜单解锁）"),
        Lang::En => format!(
            "List \"{name}\" is locked and cannot be edited by the Agent (ask the user to unlock it from the sidebar group menu)"
        ),
    }
}

/// 分组已锁定时输出错误并返回 true（调用方据此返回）
fn group_locked(conn: &Connection, user_id: i64, group_id: i64, id: &Value, lang: Lang) -> bool {
    if let Ok(Some((name, true))) = db::group_lock_info(conn, user_id, group_id) {
        tool_error(id, locked_err(&name, lang));
        return true;
    }
    false
}

/// 任务所在分组已锁定时输出错误并返回 true（调用方据此返回）
fn task_locked(conn: &Connection, user_id: i64, task_id: i64, id: &Value, lang: Lang) -> bool {
    if let Ok(Some((_, name, true))) = db::task_group_lock(conn, user_id, task_id) {
        tool_error(id, locked_err(&name, lang));
        return true;
    }
    false
}

pub(super) struct ToolDef {
    pub name: &'static str,
    pub description: &'static str,
    pub input_schema: Value,
}

impl ToolDef {
    const fn new(name: &'static str, description: &'static str, input_schema: Value) -> Self {
        Self {
            name,
            description,
            input_schema,
        }
    }
}

pub(super) fn tools(lang: Lang) -> Vec<ToolDef> {
    vec![
        ToolDef::new(
            "app_version",
            t(lang, "查询应用版本号", "Get the app version"),
            json!({ "type": "object", "properties": {} }),
        ),
        ToolDef::new(
            "app_release",
            t(
                lang,
                "查询应用发布页地址（GitHub Releases）",
                "Get the app release page URL (GitHub Releases)",
            ),
            json!({ "type": "object", "properties": {} }),
        ),
        ToolDef::new(
            "db_path",
            t(
                lang,
                "查询当前连接的数据库文件路径（本地 SQLite，可用环境变量 TODO4AGENT_DB 覆盖）",
                "Get the database file path in use (local SQLite; override with the TODO4AGENT_DB environment variable)",
            ),
            json!({ "type": "object", "properties": {} }),
        ),
        ToolDef::new(
            "group_list",
            t(lang, "列出所有任务分组", "List all task groups"),
            json!({ "type": "object", "properties": {} }),
        ),
        ToolDef::new(
            "group_create",
            t(
                lang,
                "创建任务分组；分组名不能重复",
                "Create a task group; group names must be unique",
            ),
            json!({
                "type": "object",
                "properties": {
                    "name": { "type": "string", "description": t(lang, "分组名（必填）", "Group name (required)") },
                    "description": { "type": "string", "description": t(lang, "分组描述（可选）：说明该清单的用途", "Group description (optional): what this list is for") }
                },
                "required": ["name"]
            }),
        ),
        ToolDef::new(
            "group_rename",
            t(
                lang,
                "重命名任务分组（可选同时更新分组描述）",
                "Rename a task group (optionally update its description)",
            ),
            json!({
                "type": "object",
                "properties": {
                    "id": { "type": "integer", "description": t(lang, "分组 id", "Group id") },
                    "name": { "type": "string", "description": t(lang, "新分组名（必填）", "New group name (required)") },
                    "description": { "type": "string", "description": t(lang, "分组描述（可选）：传入即更新，传空字符串清空", "Group description (optional): updated when provided, empty string clears it") }
                },
                "required": ["id", "name"]
            }),
        ),
        ToolDef::new(
            "group_delete",
            t(
                lang,
                "删除任务分组（组内任务含归档移入「无分组」；系统分组「无分组」不可删除）",
                "Delete a task group (its tasks, archived included, move to \"Ungrouped\"; the system group \"Ungrouped\" cannot be deleted)",
            ),
            json!({
                "type": "object",
                "properties": { "id": { "type": "integer", "description": t(lang, "分组 id（必填）", "Group id (required)") } },
                "required": ["id"]
            }),
        ),
        ToolDef::new(
            "task_list",
            t(
                lang,
                "列出任务；可按分组过滤，可选包含已归档",
                "List tasks; filterable by group, optionally including archived ones",
            ),
            json!({
                "type": "object",
                "properties": {
                    "group_id": { "type": "integer", "description": t(lang, "分组 id（可选，缺省返回全部）", "Group id (optional; defaults to all groups)") },
                    "include_archived": { "type": "boolean", "description": t(lang, "包含已归档任务（可选，默认 false 仅返回未归档）", "Include archived tasks (optional; defaults to false for unarchived only)") }
                }
            }),
        ),
        ToolDef::new(
            "task_create",
            t(
                lang,
                "创建任务（默认状态 pending）",
                "Create a task (status defaults to pending)",
            ),
            json!({
                "type": "object",
                "properties": {
                    "group_id": { "type": "integer", "description": t(lang, "所属分组 id（必填）", "Owning group id (required)") },
                    "title": { "type": "string", "description": t(lang, "任务标题（必填）", "Task title (required)") },
                    "description": { "type": "string", "description": t(lang, "详细说明（可选）", "Detailed description (optional)") },
                    "due_at": { "type": "string", "description": t(lang, "截止时间，ISO8601（可选）", "Due time, ISO8601 (optional)") }
                },
                "required": ["group_id", "title"]
            }),
        ),
        ToolDef::new(
            "task_update",
            t(
                lang,
                "更新任务字段（只修改传入的字段）",
                "Update task fields (only provided fields are changed)",
            ),
            json!({
                "type": "object",
                "properties": {
                    "id": { "type": "integer", "description": t(lang, "任务 id（必填）", "Task id (required)") },
                    "group_id": { "type": "integer", "description": t(lang, "移动到的分组 id", "Group id to move the task to") },
                    "title": { "type": "string", "description": t(lang, "新标题", "New title") },
                    "description": { "type": "string", "description": t(lang, "新说明", "New description") },
                    "status": { "type": "string", "enum": ["pending", "done"], "description": t(lang, "新状态", "New status") },
                    "due_at": { "type": ["string", "null"], "description": t(lang, "新截止时间；传 null 清空", "New due time; null clears it") }
                },
                "required": ["id"]
            }),
        ),
        ToolDef::new(
            "task_complete",
            t(
                lang,
                "完成 / 取消完成一个任务（切换 done 状态）",
                "Complete / uncomplete a task (toggles the done state)",
            ),
            json!({
                "type": "object",
                "properties": {
                    "id": { "type": "integer", "description": t(lang, "任务 id（必填）", "Task id (required)") },
                    "done": { "type": "boolean", "description": t(lang, "true 标记完成，false 恢复未完成（必填）", "true marks done, false restores pending (required)") }
                },
                "required": ["id", "done"]
            }),
        ),
        ToolDef::new(
            "task_archive",
            t(
                lang,
                "归档任务（从清单移入归档，界面「归档」页可查看与恢复）",
                "Archive a task (removed from its list; view and restore on the app's Archive page)",
            ),
            json!({
                "type": "object",
                "properties": { "id": { "type": "integer", "description": t(lang, "任务 id（必填）", "Task id (required)") } },
                "required": ["id"]
            }),
        ),
        ToolDef::new(
            "task_unarchive",
            t(
                lang,
                "取消归档（任务回到原清单）",
                "Unarchive a task (returns it to its original list)",
            ),
            json!({
                "type": "object",
                "properties": { "id": { "type": "integer", "description": t(lang, "任务 id（必填）", "Task id (required)") } },
                "required": ["id"]
            }),
        ),
        ToolDef::new(
            "task_delete",
            t(lang, "删除任务", "Delete a task"),
            json!({
                "type": "object",
                "properties": { "id": { "type": "integer", "description": t(lang, "任务 id（必填）", "Task id (required)") } },
                "required": ["id"]
            }),
        ),
        ToolDef::new(
            "task_export",
            t(
                lang,
                "导出任务清单与提示词为 JSON 文档（与界面导出同构）",
                "Export task lists and the prompt as a JSON document (same structure as the UI export)",
            ),
            json!({ "type": "object", "properties": {} }),
        ),
        ToolDef::new(
            "task_import",
            t(
                lang,
                "导入 JSON 文档（与 task_export 输出同构：同名分组并入、新分组新建，含 prompt 字段时提示词一并导入）",
                "Import a JSON document (same structure as task_export output: same-name groups merge, new groups are created; a prompt field also imports the prompt)",
            ),
            json!({
                "type": "object",
                "properties": {
                    "doc": {
                        "type": "object",
                        "description": t(
                            lang,
                            "任务清单文档（必填）：{version, exported_at, groups: [{name, tasks: [{title, description, status, due_at}]}]}",
                            "Task list document (required): {version, exported_at, groups: [{name, tasks: [{title, description, status, due_at}]}]}"
                        )
                    }
                },
                "required": ["doc"]
            }),
        ),
        ToolDef::new(
            "user_password",
            t(
                lang,
                "修改当前账号（启动凭据对应用户）的密码；成功后该用户的已登录会话全部失效，需同步更新客户端配置中的 TODO4AGENT_PASSWORD",
                "Change the password of the current account (the user whose credentials started this server); all its signed-in sessions are revoked on success, so update TODO4AGENT_PASSWORD in the client config accordingly",
            ),
            json!({
                "type": "object",
                "properties": {
                    "old_password": { "type": "string", "description": t(lang, "当前密码（必填）", "Current password (required)") },
                    "new_password": { "type": "string", "description": t(lang, "新密码，至少 4 位（必填）", "New password, at least 4 characters (required)") }
                },
                "required": ["old_password", "new_password"]
            }),
        ),
        ToolDef::new(
            "prompt_get",
            t(
                lang,
                "读取当前用户的 Agent 提示词（协作规范，类似 AGENTS.md）；默认为空，content 为空表示尚未设置",
                "Read the current user's Agent prompt (collaboration guidelines, like AGENTS.md); empty by default, an empty content means not set",
            ),
            json!({ "type": "object", "properties": {} }),
        ),
        ToolDef::new(
            "prompt_update",
            t(
                lang,
                "全量更新当前用户的 Agent 提示词；建议先 prompt_get 获取当前内容，按需修改后整体写回；传空字符串为清空",
                "Fully update the current user's Agent prompt; fetch it with prompt_get first, edit, then write the whole content back; an empty string clears it",
            ),
            json!({
                "type": "object",
                "properties": {
                    "content": { "type": "string", "description": t(lang, "新提示词全文；传空字符串清空（必填）", "Full new prompt text; an empty string clears it (required)") }
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
    let db_err = |e: rusqlite::Error| {
        tool_error(
            id,
            format!("{}: {e}", t(lang, "数据库错误", "Database error")),
        )
    };
    let e = |zh: &'static str, en: &'static str| t(lang, zh, en);

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
                "release_url": "https://github.com/BlazeSnow/Todo4Agent/releases"
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
                    tool_error(id, e("分组名已存在", "Group name already exists").into())
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
                            format!(
                                "{}: description {}",
                                t(lang, "参数错误", "Invalid argument"),
                                t(lang, "必须是字符串", "must be a string")
                            ),
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
                    return tool_error(
                        id,
                        e(
                            "系统分组「无分组」不可重命名",
                            "The system group \"Ungrouped\" cannot be renamed",
                        )
                        .into(),
                    );
                }
            }
            match db::rename_group(conn, user_id, gid, &name) {
                Ok(Some(_)) => {}
                Ok(None) => return tool_error(id, e("分组不存在", "Group not found").into()),
                Err(err) if db::is_unique_violation(&err) => {
                    tool_error(id, e("分组名已存在", "Group name already exists").into())
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
                Ok(None) => tool_error(id, e("分组不存在", "Group not found").into()),
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
                    return tool_error(
                        id,
                        e(
                            "系统分组「无分组」不可删除",
                            "The system group \"Ungrouped\" cannot be deleted",
                        )
                        .into(),
                    );
                }
            }
            match db::delete_group(conn, user_id, gid) {
                Ok(true) => tool_result(id, json!({ "ok": true }).to_string()),
                Ok(false) => tool_error(id, e("分组不存在", "Group not found").into()),
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
                        format!(
                            "{}: include_archived {}",
                            t(lang, "参数错误", "Invalid argument"),
                            t(lang, "必须是布尔值", "must be a boolean")
                        ),
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
                    tool_error(id, e("分组不存在", "Group not found").into())
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
                    _ => {
                        return tool_error(
                            id,
                            format!(
                                "{}: title {}",
                                t(lang, "参数错误", "Invalid argument"),
                                t(lang, "不能为空", "cannot be empty")
                            ),
                        )
                    }
                }
            }
            if let Some(v) = args.get("description") {
                match v.as_str() {
                    Some(s) => patch.description = Some(s.to_string()),
                    None => {
                        return tool_error(
                            id,
                            format!(
                                "{}: description {}",
                                t(lang, "参数错误", "Invalid argument"),
                                t(lang, "必须是字符串", "must be a string")
                            ),
                        )
                    }
                }
            }
            if let Some(v) = args.get("status") {
                match v.as_str() {
                    Some(s @ ("pending" | "done")) => patch.status = Some(s.to_string()),
                    _ => {
                        return tool_error(
                            id,
                            format!(
                                "{}: status {}",
                                t(lang, "参数错误", "Invalid argument"),
                                t(lang, "只能是 pending 或 done", "must be either pending or done")
                            ),
                        )
                    }
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
                Some(_) => {
                    return tool_error(
                        id,
                        format!(
                            "{}: due_at {}",
                            t(lang, "参数错误", "Invalid argument"),
                            t(lang, "必须是字符串或 null", "must be a string or null")
                        ),
                    )
                }
            }
            match db::update_task(conn, user_id, tid, &patch) {
                Ok(Some(task)) => tool_result(id, json!(task).to_string()),
                Ok(None) => tool_error(id, e("任务不存在", "Task not found").into()),
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
                _ => {
                    return tool_error(
                        id,
                        format!(
                            "{}: done {}",
                            t(lang, "参数错误", "Invalid argument"),
                            t(lang, "必填且必须是布尔值", "is required and must be a boolean")
                        ),
                    )
                }
            };
            let patch = db::TaskUpdate {
                status: Some(if done { "done" } else { "pending" }.to_string()),
                ..Default::default()
            };
            match db::update_task(conn, user_id, tid, &patch) {
                Ok(Some(task)) => tool_result(id, json!(task).to_string()),
                Ok(None) => tool_error(id, e("任务不存在", "Task not found").into()),
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
                Ok(false) => tool_error(
                    id,
                    e("任务不存在或已归档", "Task not found or already archived").into(),
                ),
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
                Ok(false) => tool_error(id, e("任务不在归档中", "Task is not in the archive").into()),
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
                Ok(false) => tool_error(id, e("任务不存在", "Task not found").into()),
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
                        match lang {
                            Lang::Zh => format!("参数错误: doc 必须是任务清单文档 JSON: {err}"),
                            Lang::En => format!("Invalid argument: doc must be a task list document JSON: {err}"),
                        },
                    )
                }
                None => return tool_error(id, arg_req_err("doc", lang)),
            };
            if doc.groups.is_empty() {
                return tool_error(id, e("导入内容为空", "Import content is empty").into());
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
                return tool_error(
                    id,
                    match lang {
                        Lang::Zh => format!(
                            "文档包含已锁定的清单：{}（请让用户在界面导入或先解锁）",
                            conflicts.join("、")
                        ),
                        Lang::En => format!(
                            "The document contains locked lists: {} (ask the user to import from the UI or unlock them first)",
                            conflicts.join(", ")
                        ),
                    },
                );
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
                        format!(
                            "{}: old_password {}",
                            t(lang, "参数错误", "Invalid argument"),
                            t(lang, "必填且必须是字符串", "is required and must be a string")
                        ),
                    )
                }
            };
            let new = match args.get("new_password").and_then(Value::as_str) {
                Some(v) => v.to_string(),
                None => {
                    return tool_error(
                        id,
                        format!(
                            "{}: new_password {}",
                            t(lang, "参数错误", "Invalid argument"),
                            t(lang, "必填且必须是字符串", "is required and must be a string")
                        ),
                    )
                }
            };
            if new.len() < 4 {
                return tool_error(
                    id,
                    format!(
                        "{}: new_password {}",
                        t(lang, "参数错误", "Invalid argument"),
                        t(lang, "至少 4 位", "must be at least 4 characters")
                    ),
                );
            }
            match db::change_user_password(conn, user_id, &old, &new) {
                Ok(true) => {
                    // 与界面改密一致：吊销该用户全部已登录会话
                    // （MCP 自身用环境变量凭据，当前连接不受影响）
                    let _ = db::delete_user_sessions(conn, user_id, None);
                    tool_result(
                        id,
                        json!({
                            "ok": true,
                            "note": t(
                                lang,
                                "密码已修改；请同步更新 MCP 客户端配置中的 TODO4AGENT_PASSWORD（当前连接不受影响，下次启动需用新密码）",
                                "Password changed; update TODO4AGENT_PASSWORD in the MCP client config accordingly (the current connection is unaffected; the next launch needs the new password)"
                            )
                        })
                        .to_string(),
                    )
                }
                Ok(false) => tool_error(id, e("原密码错误", "Current password is incorrect").into()),
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
                        format!(
                            "{}: content {}",
                            t(lang, "参数错误", "Invalid argument"),
                            t(lang, "必填且必须是字符串", "is required and must be a string")
                        ),
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

        _ => tool_error(
            id,
            format!("{}: {name}", t(lang, "未知工具", "Unknown tool")),
        ),
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
        let zh = tools(Lang::Zh);
        assert!(zh.iter().any(|t| t.description.contains("列出所有任务分组")));
        let en = tools(Lang::En);
        assert!(en.iter().any(|t| t.description.contains("List all task groups")));
        // 名称（协议契约）不随语言变化
        assert_eq!(
            zh.iter().map(|t| t.name).collect::<Vec<_>>(),
            en.iter().map(|t| t.name).collect::<Vec<_>>(),
        );
    }
}
