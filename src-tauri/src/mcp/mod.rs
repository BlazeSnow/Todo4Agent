//! MCP（Model Context Protocol）stdio Server。
//! 与桌面端/HTTP 服务共享同一个 SQLite 数据库，供 Agent 调用任务清单工具。
//! 协议交互为换行分隔的 JSON-RPC 2.0 消息（stdin 读入，stdout 输出）。

pub mod tools;

use tools::call_tool;

use rusqlite::Connection;
use serde_json::{json, Value};
use std::io::{BufRead, Write};

use crate::db;
use crate::lang::{t, Lang};

pub(super) fn send(v: &Value) {
    let mut out = std::io::stdout().lock();
    let _ = writeln!(out, "{v}");
    let _ = out.flush();
}

pub(super) fn respond(id: &Value, result: Value) {
    send(&json!({ "jsonrpc": "2.0", "id": id, "result": result }));
}

pub(super) fn respond_error(id: &Value, code: i64, message: &str) {
    send(&json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": code, "message": message }
    }));
}

pub(super) fn tool_result(id: &Value, text: String) {
    respond(
        id,
        json!({ "content": [{ "type": "text", "text": text }] }),
    );
}

pub(super) fn tool_error(id: &Value, text: String) {
    respond(
        id,
        json!({ "content": [{ "type": "text", "text": text }], "isError": true }),
    );
}

fn handle(msg: &Value, conn: &Connection, user_id: i64, lang: &mut Lang) {
    let id = msg.get("id").cloned().unwrap_or(Value::Null);
    let method = msg
        .get("method")
        .and_then(Value::as_str)
        .unwrap_or_default();

    match method {
        // 协议版本以客户端为准；locale 记录为会话语言（工具描述与消息按其返回）
        "initialize" => {
            let proto = msg
                .get("protocolVersion")
                .and_then(Value::as_str)
                .unwrap_or("2025-03-26")
                .to_string();
            if let Some(l) = Lang::parse_tag(msg.pointer("/params/locale").and_then(Value::as_str)) {
                *lang = l;
            }
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
            let list: Vec<Value> = tools::tools(*lang)
                .iter()
                .map(|tool| {
                    json!({
                        "name": tool.name,
                        "description": tool.description,
                        "inputSchema": tool.input_schema
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
            call_tool(&name, &args, conn, user_id, &id, *lang);
        }
        "" => respond_error(&id, -32601, "Method not found"),
        _ => respond_error(&id, -32601, "Method not found"),
    }
}

/// 解析并校验 MCP 身份：TODO4AGENT_USERNAME / TODO4AGENT_PASSWORD 环境变量
/// 指定真实用户凭据，缺失任一或校验失败均报错（应用启动即播种初始 admin 用户）
fn resolve_mcp_user(
    conn: &Connection,
    username: Option<&str>,
    password: Option<&str>,
    lang: Lang,
) -> Result<i64, String> {
    match (username, password) {
        (Some(u), Some(p)) => match db::verify_user(conn, u, p) {
            Ok(Some(user)) => Ok(user.id),
            Ok(None) => Err(format!("{}：{u}", t(lang, "用户名或密码错误", "Invalid username or password"))),
            Err(e) => Err(format!("{}：{e}", t(lang, "验证用户失败", "Failed to verify user"))),
        },
        _ => Err(t(
            lang,
            "MCP 需要设置 TODO4AGENT_USERNAME 与 TODO4AGENT_PASSWORD 环境变量（运行 todo4agent help 查看接入说明）",
            "MCP requires the TODO4AGENT_USERNAME and TODO4AGENT_PASSWORD environment variables (run todo4agent help for setup instructions)",
        )
        .to_string()),
    }
}

/// 主循环：逐行读取 stdin 的 JSON-RPC 消息并应答。
/// 身份由环境变量 TODO4AGENT_USERNAME / TODO4AGENT_PASSWORD 指定并验证；
/// 验证失败时向 stderr 输出原因并以非零码退出（MCP 客户端会显示启动失败）。
/// 会话语言：TODO4AGENT_LANG 环境变量（zh / en）优先，
/// 否则取 initialize 请求的 locale，未提供时默认中文。
pub fn serve(conn: &Connection) {
    let env_user = std::env::var("TODO4AGENT_USERNAME").ok();
    let env_pass = std::env::var("TODO4AGENT_PASSWORD").ok();
    let mut lang = Lang::from_env().unwrap_or_default();
    let user_id = match resolve_mcp_user(conn, env_user.as_deref(), env_pass.as_deref(), lang) {
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
        handle(&msg, conn, user_id, &mut lang);
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
    fn defaults_and_credentials() {
        let c = test_conn();
        // 初始 admin 用户自动创建，默认密码可登录
        let uid = resolve_mcp_user(&c, Some("admin"), Some("admin123"), Lang::Zh).unwrap();
        assert!(uid > 0);
        // 缺任一凭据拒绝
        assert!(resolve_mcp_user(&c, None, None, Lang::Zh).is_err());
        assert!(resolve_mcp_user(&c, Some("x"), None, Lang::Zh).is_err());
        // 错误密码
        assert!(resolve_mcp_user(&c, Some("admin"), Some("wrong"), Lang::Zh).is_err());
    }

    #[test]
    fn credentials_verify_and_bind() {
        let c = test_conn();
        db::create_user(&c, "alice", "pass1234").unwrap();
        let uid = resolve_mcp_user(&c, Some("alice"), Some("pass1234"), Lang::Zh).unwrap();
        assert!(uid > 0);
        assert!(resolve_mcp_user(&c, Some("alice"), Some("wrong"), Lang::Zh).is_err());
        assert!(resolve_mcp_user(&c, Some("nobody"), Some("pass1234"), Lang::Zh).is_err());
        assert!(resolve_mcp_user(&c, None, None, Lang::Zh).is_err());
        assert!(resolve_mcp_user(&c, Some("alice"), None, Lang::Zh).is_err());
    }

    #[test]
    fn credentials_error_language() {
        let c = test_conn();
        let zh = resolve_mcp_user(&c, Some("admin"), Some("wrong"), Lang::Zh).unwrap_err();
        assert!(zh.contains("用户名或密码错误"));
        let en = resolve_mcp_user(&c, Some("admin"), Some("wrong"), Lang::En).unwrap_err();
        assert!(en.contains("Invalid username or password"));
    }
}
