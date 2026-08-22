//! MCP（Model Context Protocol）stdio Server。
//! 与桌面端/HTTP 服务共享同一个 SQLite 数据库，供 Agent 调用任务清单工具。
//! 协议交互为换行分隔的 JSON-RPC 2.0 消息（stdin 读入，stdout 输出）。

use rusqlite::Connection;
use serde_json::{json, Value};
use std::io::{BufRead, Write};

use crate::db;

struct ToolDef {
    name: &'static str,
    description: &'static str,
    input_schema: Value,
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

fn tools() -> Vec<ToolDef> {
    vec![
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
            "导出全部任务清单为 JSON 文档（与界面导出同构）",
            json!({ "type": "object", "properties": {} }),
        ),
    ]
}

fn send(v: &Value) {
    let mut out = std::io::stdout().lock();
    let _ = writeln!(out, "{v}");
    let _ = out.flush();
}

fn respond(id: &Value, result: Value) {
    send(&json!({ "jsonrpc": "2.0", "id": id, "result": result }));
}

fn respond_error(id: &Value, code: i64, message: &str) {
    send(&json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": code, "message": message }
    }));
}

fn tool_result(id: &Value, text: String) {
    respond(
        id,
        json!({ "content": [{ "type": "text", "text": text }] }),
    );
}

fn tool_error(id: &Value, text: String) {
    respond(
        id,
        json!({ "content": [{ "type": "text", "text": text }], "isError": true }),
    );
}

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

fn call_tool(name: &str, args: &Value, conn: &Connection, user_id: Option<i64>, id: &Value) {
    let db_err = |e: rusqlite::Error| tool_error(id, format!("数据库错误: {e}"));

    match name {
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
                Err(e) if db::is_unique_violation(&e) => tool_error(id, "分组不存在".into()),
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

        _ => tool_error(id, format!("未知工具: {name}")),
    }
}

fn handle(msg: &Value, conn: &Connection, user_id: Option<i64>) {
    let id = msg.get("id").cloned().unwrap_or(Value::Null);
    let method = msg
        .get("method")
        .and_then(Value::as_str)
        .unwrap_or_default();

    match method {
        // 协议版本以客户端为准
        "initialize" => {
            let proto = msg
                .get("protocolVersion")
                .and_then(Value::as_str)
                .unwrap_or("2025-03-26")
                .to_string();
            respond(
                &id,
                json!({
                    "protocolVersion": proto,
                    "capabilities": { "tools": { "listChanged": false } },
                    "serverInfo": { "name": "todo4agent", "version": env!("CARGO_PKG_VERSION") }
                }),
            );
        }
        // 通知类消息无需响应
        "notifications/initialized" | "notifications/cancelled" => {}
        "ping" => respond(&id, json!({})),
        "tools/list" => {
            let list: Vec<Value> = tools()
                .iter()
                .map(|t| {
                    json!({
                        "name": t.name,
                        "description": t.description,
                        "inputSchema": t.input_schema
                    })
                })
                .collect();
            respond(&id, json!({ "tools": list }));
        }
        "tools/call" => {
            let name = msg
                .pointer("/params/name")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            let args = msg
                .pointer("/params/arguments")
                .cloned()
                .unwrap_or_else(|| json!({}));
            call_tool(&name, &args, conn, user_id, &id);
        }
        "" => respond_error(&id, -32601, "Method not found"),
        _ => respond_error(&id, -32601, "Method not found"),
    }
}

/// 解析 MCP 身份：
/// - 提供 TODO4AGENT_USERNAME / TODO4AGENT_PASSWORD 环境变量时校验凭据并绑定该用户；
/// - 未提供凭据时：本地模式（尚无用户）可用，多用户模式拒绝启动。
fn resolve_mcp_user(
    conn: &Connection,
    username: Option<&str>,
    password: Option<&str>,
) -> Result<Option<i64>, String> {
    let users_exist = db::user_count(conn).unwrap_or(0) > 0;
    match (username, password) {
        (Some(u), Some(p)) => match db::verify_user(conn, u, p) {
            Ok(Some(user)) => Ok(Some(user.id)),
            Ok(None) => Err(format!("用户名或密码错误：{u}")),
            Err(e) => Err(format!("验证用户失败：{e}")),
        },
        _ => {
            if users_exist {
                Err("多用户模式下 MCP 需要设置 TODO4AGENT_USERNAME 与 TODO4AGENT_PASSWORD 环境变量".to_string())
            } else {
                // 本地模式：无用户数据空间
                Ok(None)
            }
        }
    }
}

/// 主循环：逐行读取 stdin 的 JSON-RPC 消息并应答。
/// 身份由环境变量 TODO4AGENT_USERNAME / TODO4AGENT_PASSWORD 指定并验证；
/// 验证失败时向 stderr 输出原因并以非零码退出（MCP 客户端会显示启动失败）。
pub fn serve(conn: &Connection) {
    let env_user = std::env::var("TODO4AGENT_USERNAME").ok();
    let env_pass = std::env::var("TODO4AGENT_PASSWORD").ok();
    let user_id = match resolve_mcp_user(conn, env_user.as_deref(), env_pass.as_deref()) {
        Ok(uid) => uid,
        Err(msg) => {
            eprintln!("todo4agent-mcp: {msg}");
            std::process::exit(1);
        }
    };

    let stdin = std::io::stdin();
    let mut line = String::new();
    loop {
        line.clear();
        match stdin.lock().read_line(&mut line) {
            Ok(0) => break, // EOF（客户端断开）
            Ok(_) => {}
            Err(e) => {
                eprintln!("stdin 读取失败: {e}");
                break;
            }
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let msg: Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(_) => {
                send(&json!({
                    "jsonrpc": "2.0",
                    "id": Value::Null,
                    "error": { "code": -32700, "message": "Parse error" }
                }));
                continue;
            }
        };
        handle(&msg, conn, user_id);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use std::path::Path;

    fn test_conn() -> rusqlite::Connection {
        db::open(Path::new(":memory:")).expect("open memory db")
    }

    #[test]
    fn local_mode_no_credentials_ok() {
        let c = test_conn();
        assert_eq!(resolve_mcp_user(&c, None, None).unwrap(), None);
        assert_eq!(resolve_mcp_user(&c, Some("x"), None).unwrap(), None);
    }

    #[test]
    fn credentials_verify_and_bind() {
        let c = test_conn();
        db::create_user(&c, "alice", "pass1234").unwrap();
        let uid = resolve_mcp_user(&c, Some("alice"), Some("pass1234")).unwrap();
        assert!(uid.is_some());
        // 错误密码 / 不存在用户
        assert!(resolve_mcp_user(&c, Some("alice"), Some("wrong")).is_err());
        assert!(resolve_mcp_user(&c, Some("nobody"), Some("pass1234")).is_err());
        // 多用户模式缺凭据拒绝
        assert!(resolve_mcp_user(&c, None, None).is_err());
        assert!(resolve_mcp_user(&c, Some("alice"), None).is_err());
    }
}
