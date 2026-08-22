//! Todo4Agent 入口。
//! 三种模式：
//! - 默认：Tauri 桌面应用（后台启动 HTTP 服务，窗口加载 WebUI）
//! - `serve`：headless HTTP 服务（WebUI 于 3000 端口）
//! - `mcp`：MCP stdio 服务（供 Agent 连接操作任务清单）

mod api;
mod db;
mod mcp;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mode = args
        .iter()
        .skip(1)
        .find(|a| matches!(a.as_str(), "mcp" | "--mcp" | "serve" | "--serve"));

    match mode.map(String::as_str) {
        Some("mcp") | Some("--mcp") => {
            let conn = db::open(&db::db_path()).expect("打开数据库失败");
            mcp::serve(&conn);
            return;
        }
        Some("serve") | Some("--serve") => {
            api::serve_blocking();
            return;
        }
        _ => {}
    }

    // 桌面模式：先启动 HTTP 服务，再创建窗口加载 WebUI
    let port = api::spawn_server();
    let dev = std::env::var("TAURI_ENV_DEBUG").map(|v| v == "true").unwrap_or(false);
    let url = if dev {
        "http://localhost:3001".to_string()
    } else {
        format!("http://127.0.0.1:{port}")
    };

    tauri::Builder::default()
        .setup(move |app| {
            tauri::WebviewWindowBuilder::new(
                app,
                "main",
                tauri::WebviewUrl::External(url.parse().expect("非法窗口 URL")),
            )
            .title("Todo4Agent")
            .inner_size(1100.0, 720.0)
            .min_inner_size(800.0, 600.0)
            .build()?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Tauri 应用启动失败");
}