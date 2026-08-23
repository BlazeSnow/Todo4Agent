use rusqlite::Connection;
use serde_json::{json, Value};

use super::{tool_error, tool_result};


fn arg_str(args: &Value, key: &str) -> Result<String, String> {
    match args.get(key) {
        Some(Value::String(s)) if !s.trim().is_empty() => Ok(s.trim().to_string()),
        _ => Err(format!("参数错误: {key} 必填且不能为空")),
    }
}

fn arg_i64(args: &Value, key: &str) -> Result<i64, String> {
    match args.get(key) {
        Some(Value::Number(n)) => n.as_i64().ok_or_else(|| format!("参数错误: {key} 必须是整数")),
        _ => Err(format!("参数错误: {key} 必填")),
    }
}

fn arg_opt_i64(args: &Value, key: &str) -> Result<Option<i64>, String> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Number(n)) => n.as_i64().map(Some).ok_or_else(|| format!("参数错误: {key} 必须是整数")),
        Some(_) => Err(format!("参数错误: {key} 必须是整数")),
    }
}



use crate::db;

/// 任务清单写操作：任务清单锁定时被拒绝（列表/导出等读取不受影响，界面编辑不受影响）
const TASK_WRITE_TOOLS: &[&str] = &[
    "group_create",
    "group_rename",
    "group_delete",
    "task_create",
    "task_update",
    "task_complete",
    "task_delete",
    "task_import",
];

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

pub(super) fn tools() -> Vec<ToolDef> {
    vec![
        ToolDef::new(
            "app_version",
            "查询应用版本号",
            json!({ "type": "object", "properties": {} }),
        ),
        ToolDef::new(
            "app_release",
            "查询应用发布页地址（GitHub Releases）",
            json!({ "type": "object", "properties": {} }),
        ),
        ToolDef::new(
            "group_list",
            "列出所有任务分组",
            json!({ "type": "object", "properties": {} }),
        ),
        ToolDef::new(
            "group_create",
            "创建任务分组；分组名不能重复",
            json!({
                "type": "object",
                "properties": { "name": { "type": "string", "description": "分组名（必填）" } },
                "required": ["name"]
            }),
        ),
        ToolDef::new(
            "group_rename",
            "重命名任务分组",
            json!({
                "type": "object",
                "properties": {
                    "id": { "type": "integer", "description": "分组 id" },
                    "name": { "type": "string", "description": "新分组名（必填）" }
                },
                "required": ["id", "name"]
            }),
        ),
        ToolDef::new(
            "group_delete",
            "删除任务分组（其下任务一并移入回收站）",
            json!({
                "type": "object",
                "properties": { "id": { "type": "integer", "description": "分组 id（必填）" } },
                "required": ["id"]
            }),
        ),
        ToolDef::new(
            "task_list",
            "列出任务；可按分组过滤",
            json!({
                "type": "object",
                "properties": { "group_id": { "type": "integer", "description": "分组 id（可选，缺省返回全部）" } }
            }),
        ),
        ToolDef::new(
            "task_create",
            "创建任务（默认状态 pending）",
            json!({
                "type": "object",
                "properties": {
                    "group_id": { "type": "integer", "description": "所属分组 id（必填）" },
                    "title": { "type": "string", "description": "任务标题（必填）" },
                    "description": { "type": "string", "description": "详细说明（可选）" },
                    "due_at": { "type": "string", "description": "截止时间，ISO8601（可选）" }
                },
                "required": ["group_id", "title"]
            }),
        ),
        ToolDef::new(
            "task_update",
            "更新任务字段（只修改传入的字段）",
            json!({
                "type": "object",
                "properties": {
                    "id": { "type": "integer", "description": "任务 id（必填）" },
                    "group_id": { "type": "integer", "description": "移动到的分组 id" },
                    "title": { "type": "string", "description": "新标题" },
                    "description": { "type": "string", "description": "新说明" },
                    "status": { "type": "string", "enum": ["pending", "done"], "description": "新状态" },
                    "due_at": { "type": ["string", "null"], "description": "新截止时间；传 null 清空" }
                },
                "required": ["id"]
            }),
        ),
        ToolDef::new(
            "task_complete",
            "完成 / 取消完成一个任务（切换 done 状态）",
            json!({
                "type": "object",
                "properties": {
                    "id": { "type": "integer", "description": "任务 id（必填）" },
                    "done": { "type": "boolean", "description": "true 标记完成，false 恢复未完成（必填）" }
                },
                "required": ["id", "done"]
            }),
        ),
        ToolDef::new(
            "task_delete",
            "删除任务",
            json!({
                "type": "object",
                "properties": { "id": { "type": "integer", "description": "任务 id（必填）" } },
                "required": ["id"]
            }),
        ),
        ToolDef::new(
            "task_export",
            "导出任务清单与提示词为 JSON 文档（与界面导出同构）",
            json!({ "type": "object", "properties": {} }),
        ),
        ToolDef::new(
            "task_import",
            "导入 JSON 文档（与 task_export 输出同构：同名分组并入、新分组新建，含 prompt 字段时提示词一并导入）",
            json!({
                "type": "object",
                "properties": {
                    "doc": {
                        "type": "object",
                        "description": "任务清单文档（必填）：{version, exported_at, groups: [{name, tasks: [{title, description, status, due_at}]}]}"
                    }
                },
                "required": ["doc"]
            }),
        ),
        ToolDef::new(
            "user_password",
            "修改当前账号（启动凭据对应用户）的密码；成功后该用户的已登录会话全部失效，需同步更新客户端配置中的 TODO4AGENT_PASSWORD",
            json!({
                "type": "object",
                "properties": {
                    "old_password": { "type": "string", "description": "当前密码（必填）" },
                    "new_password": { "type": "string", "description": "新密码，至少 4 位（必填）" }
                },
                "required": ["old_password", "new_password"]
            }),
        ),
        ToolDef::new(
            "prompt_get",
            "读取当前用户的 Agent 提示词（协作规范，类似 AGENTS.md）；默认为空，content 为空表示尚未设置",
            json!({ "type": "object", "properties": {} }),
        ),
        ToolDef::new(
            "prompt_update",
            "全量更新当前用户的 Agent 提示词；建议先 prompt_get 获取当前内容，按需修改后整体写回；传空字符串为清空",
            json!({
                "type": "object",
                "properties": {
                    "content": { "type": "string", "description": "新提示词全文；传空字符串清空（必填）" }
                },
                "required": ["content"]
            }),
        ),
    ]
}

pub(super) fn call_tool(name: &str, args: &Value, conn: &Connection, user_id: i64, id: &Value) {
    let db_err = |e: rusqlite::Error| tool_error(id, format!("数据库错误: {e}"));

    // 任务清单锁定：拒绝写操作，读取类工具照常
    if TASK_WRITE_TOOLS.contains(&name) {
        match db::tasks_locked(conn, user_id) {
            Ok(true) => {
                return tool_error(
                    id,
                    "任务清单已锁定，Agent 无法编辑（列表、导出等读取不受影响；请让用户在界面右键菜单解锁）".into(),
                )
            }
            Ok(false) => {}
            Err(e) => return db_err(e),
        }
    }

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

        "group_list" => match db::list_groups(conn, user_id) {
            Ok(v) => tool_result(id, json!(v).to_string()),
            Err(e) => db_err(e),
        },

        "group_create" => {
            let name = match arg_str(args, "name") {
                Ok(v) => v,
                Err(m) => return tool_error(id, m),
            };
            match db::create_group(conn, user_id, &name) {
                Ok(g) => tool_result(id, json!(g).to_string()),
                Err(e) if db::is_unique_violation(&e) => tool_error(id, "分组名已存在".into()),
                Err(e) => db_err(e),
            }
        }

        "group_rename" => {
            let gid = match arg_i64(args, "id") {
                Ok(v) => v,
                Err(m) => return tool_error(id, m),
            };
            let name = match arg_str(args, "name") {
                Ok(v) => v,
                Err(m) => return tool_error(id, m),
            };
            match db::rename_group(conn, user_id, gid, &name) {
                Ok(Some(g)) => tool_result(id, json!(g).to_string()),
                Ok(None) => tool_error(id, "分组不存在".into()),
                Err(e) if db::is_unique_violation(&e) => tool_error(id, "分组名已存在".into()),
                Err(e) => db_err(e),
            }
        }

        "group_delete" => {
            let gid = match arg_i64(args, "id") {
                Ok(v) => v,
                Err(m) => return tool_error(id, m),
            };
            match db::delete_group(conn, user_id, gid) {
                Ok(true) => tool_result(id, json!({ "ok": true }).to_string()),
                Ok(false) => tool_error(id, "分组不存在".into()),
                Err(e) => db_err(e),
            }
        }

        "task_list" => {
            let gid = match arg_opt_i64(args, "group_id") {
                Ok(v) => v,
                Err(m) => return tool_error(id, m),
            };
            match db::list_tasks(conn, user_id, gid) {
                Ok(v) => tool_result(id, json!(v).to_string()),
                Err(e) => db_err(e),
            }
        }

        "task_create" => {
            let gid = match arg_i64(args, "group_id") {
                Ok(v) => v,
                Err(m) => return tool_error(id, m),
            };
            let title = match arg_str(args, "title") {
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
                Ok(t) => tool_result(id, json!(t).to_string()),
                // create_task 预检查分组归属，分组缺失时返回 QueryReturnedNoRows
                Err(rusqlite::Error::QueryReturnedNoRows) => {
                    tool_error(id, "分组不存在".into())
                }
                Err(e) => db_err(e),
            }
        }

        "task_update" => {
            let tid = match arg_i64(args, "id") {
                Ok(v) => v,
                Err(m) => return tool_error(id, m),
            };
            let mut patch = db::TaskUpdate::default();
            if let Some(v) = args.get("title") {
                match v.as_str() {
                    Some(s) if !s.trim().is_empty() => patch.title = Some(s.trim().to_string()),
                    _ => return tool_error(id, "参数错误: title 不能为空".into()),
                }
            }
            if let Some(v) = args.get("description") {
                match v.as_str() {
                    Some(s) => patch.description = Some(s.to_string()),
                    _ => return tool_error(id, "参数错误: description 必须是字符串".into()),
                }
            }
            if let Some(v) = args.get("status") {
                match v.as_str() {
                    Some(s @ ("pending" | "done")) => patch.status = Some(s.to_string()),
                    _ => return tool_error(id, "参数错误: status 只能是 pending 或 done".into()),
                }
            }
            if let Some(v) = args.get("group_id") {
                match v.as_i64() {
                    Some(n) => patch.group_id = Some(n),
                    None => return tool_error(id, "参数错误: group_id 必须是整数".into()),
                }
            }
            match args.get("due_at") {
                None => {}
                Some(Value::Null) => patch.due_at = Some(None),
                Some(Value::String(s)) => patch.due_at = Some(Some(s.clone())),
                Some(_) => return tool_error(id, "参数错误: due_at 必须是字符串或 null".into()),
            }
            match db::update_task(conn, user_id, tid, &patch) {
                Ok(Some(t)) => tool_result(id, json!(t).to_string()),
                Ok(None) => tool_error(id, "任务不存在".into()),
                Err(e) => db_err(e),
            }
        }

        "task_complete" => {
            let tid = match arg_i64(args, "id") {
                Ok(v) => v,
                Err(m) => return tool_error(id, m),
            };
            let done = match args.get("done") {
                Some(Value::Bool(b)) => *b,
                _ => return tool_error(id, "参数错误: done 必填且必须是布尔值".into()),
            };
            let patch = db::TaskUpdate {
                status: Some(if done { "done" } else { "pending" }.to_string()),
                ..Default::default()
            };
            match db::update_task(conn, user_id, tid, &patch) {
                Ok(Some(t)) => tool_result(id, json!(t).to_string()),
                Ok(None) => tool_error(id, "任务不存在".into()),
                Err(e) => db_err(e),
            }
        }

        "task_delete" => {
            let tid = match arg_i64(args, "id") {
                Ok(v) => v,
                Err(m) => return tool_error(id, m),
            };
            match db::delete_task(conn, user_id, tid) {
                Ok(true) => tool_result(id, json!({ "ok": true }).to_string()),
                Ok(false) => tool_error(id, "任务不存在".into()),
                Err(e) => db_err(e),
            }
        }

        "task_export" => match db::export_all(conn, user_id) {
            Ok(doc) => tool_result(id, serde_json::to_string(&doc).unwrap_or_else(|e| e.to_string())),
            Err(e) => db_err(e),
        },

        "task_import" => {
            let doc: db::ExportDoc = match args.get("doc").cloned().map(serde_json::from_value) {
                Some(Ok(d)) => d,
                Some(Err(e)) => {
                    return tool_error(id, format!("参数错误: doc 必须是任务清单文档 JSON: {e}"))
                }
                None => return tool_error(id, "参数错误: doc 必填".into()),
            };
            if doc.groups.is_empty() {
                return tool_error(id, "导入内容为空".into());
            }
            match db::import_doc(conn, user_id, &doc) {
                Ok(r) => tool_result(id, json!(r).to_string()),
                Err(e) => db_err(e),
            }
        }

        "user_password" => {
            // 密码不做 trim：与 HTTP 接口一致，按原样校验
            let old = match args.get("old_password").and_then(Value::as_str) {
                Some(v) => v.to_string(),
                None => return tool_error(id, "参数错误: old_password 必填且必须是字符串".into()),
            };
            let new = match args.get("new_password").and_then(Value::as_str) {
                Some(v) => v.to_string(),
                None => return tool_error(id, "参数错误: new_password 必填且必须是字符串".into()),
            };
            if new.len() < 4 {
                return tool_error(id, "参数错误: new_password 至少 4 位".into());
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
                            "note": "密码已修改；请同步更新 MCP 客户端配置中的 TODO4AGENT_PASSWORD（当前连接不受影响，下次启动需用新密码）"
                        })
                        .to_string(),
                    )
                }
                Ok(false) => tool_error(id, "原密码错误".into()),
                Err(e) => db_err(e),
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
            Err(e) => db_err(e),
        },

        "prompt_update" => {
            let content = match args.get("content").and_then(Value::as_str) {
                Some(s) => s.to_string(),
                None => return tool_error(id, "参数错误: content 必填且必须是字符串".into()),
            };
            match db::set_prompt(conn, user_id, &content) {
                Ok((is_default, updated_at)) => tool_result(
                    id,
                    json!({ "ok": true, "is_default": is_default, "updated_at": updated_at }).to_string(),
                ),
                Err(e) => db_err(e),
            }
        }

        _ => tool_error(id, format!("未知工具: {name}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_version_tool_defined() {
        assert!(tools().iter().any(|t| t.name == "app_version"));
        assert!(tools().iter().any(|t| t.name == "app_release"));
    }

    #[test]
    fn new_tools_defined() {
        let names: Vec<&str> = tools().iter().map(|t| t.name).collect();
        assert!(names.contains(&"group_delete"));
        assert!(names.contains(&"task_import"));
        assert!(names.contains(&"user_password"));
        assert!(names.contains(&"prompt_get"));
        assert!(names.contains(&"prompt_update"));
    }

    #[test]
    fn task_write_tools_are_known_tools() {
        let names: Vec<&str> = tools().iter().map(|t| t.name).collect();
        // 锁定拦截清单里的每个名字都必须是已定义工具
        for n in TASK_WRITE_TOOLS {
            assert!(names.contains(n), "TASK_WRITE_TOOLS 含未定义工具: {n}");
        }
        // 读取类工具不在拦截清单中
        for read in ["group_list", "task_list", "task_export", "app_version", "prompt_get"] {
            assert!(!TASK_WRITE_TOOLS.contains(&read));
        }
    }
}
