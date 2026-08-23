//! Todo4Agent 入口。
//! 运行模式（由命令行参数决定，`todo4agent help` 查看完整说明）：
//! - 默认：Tauri 桌面应用（后台启动 HTTP 服务，窗口加载 WebUI）
//! - `serve`：headless HTTP 服务（WebUI 于 3000 端口）
//! - `mcp`：MCP stdio 服务（供 Agent 连接操作任务清单）
//! - `help` / `version`：查看帮助 / 版本号

// Windows 发布版走 GUI 子系统，避免启动时弹出黑色控制台窗口；
// debug 构建保留控制台，便于查看后端日志。
#![cfg_attr(all(not(debug_assertions), windows), windows_subsystem = "windows")]

use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, WindowEvent,
};

mod api;
mod auth;
mod db;
mod mcp;

/// 命令行运行模式：由首个被识别的参数决定
#[derive(Debug)]
enum RunMode {
    /// Tauri 桌面应用（默认，无参数或仅非选项参数）
    Desktop,
    Serve,
    Mcp,
    Help,
    Version,
    /// 以 - 开头但未被识别的参数：用法错误，避免拼错参数时误入桌面模式
    UnknownFlag(String),
}

fn parse_mode<I: IntoIterator<Item = String>>(args: I) -> RunMode {
    for a in args {
        match a.as_str() {
            "mcp" | "--mcp" => return RunMode::Mcp,
            "serve" | "--serve" => return RunMode::Serve,
            "help" | "--help" | "-h" => return RunMode::Help,
            "version" | "--version" | "-V" => return RunMode::Version,
            other if other.starts_with('-') => {
                return RunMode::UnknownFlag(other.to_string())
            }
            // 非选项参数（如拖放的文件路径）忽略，维持默认桌面模式
            _ => {}
        }
    }
    RunMode::Desktop
}

fn main() {
    match parse_mode(std::env::args().skip(1)) {
        RunMode::Mcp => {
            let conn = db::open(&db::db_path()).expect("打开数据库失败");
            mcp::serve(&conn);
        }
        RunMode::Serve => api::serve_blocking(),
        RunMode::Help => print_help(),
        RunMode::Version => println!("todo4agent {}", env!("CARGO_PKG_VERSION")),
        RunMode::UnknownFlag(arg) => {
            eprintln!("未知参数: {arg}\n");
            print_help();
            std::process::exit(2);
        }
        RunMode::Desktop => run_desktop(),
    }
}

/// 输出命令行帮助：运行模式、MCP 客户端配置示例、初始账号与数据库位置。
/// 供用户与 Agent 首次运行时自助了解接入方式。
fn print_help() {
    println!(
        "Todo4Agent v{} — 为 Agent 设计的 MCP 任务清单",
        env!("CARGO_PKG_VERSION")
    );
    // 大段文本作为参数传入而非格式串，避免 JSON 花括号被当作格式占位符
    println!(
        "{}",
        r#"
用法:
  todo4agent              启动桌面应用（后台同时提供 WebUI 服务，默认 3000 端口）
  todo4agent serve        无界面启动 WebUI / HTTP API（默认 3000 端口，占用时顺延）
  todo4agent mcp          启动 MCP Server（stdio，供 Agent 客户端连接）
  todo4agent help         显示本帮助
  todo4agent version      显示版本号

MCP 接入（客户端配置示例，ZCode / Claude Desktop 通用格式）:
{
  "mcpServers": {
    "todo4agent": {
      "command": "todo4agent",
      "args": ["mcp"],
      "env": {
        "TODO4AGENT_USERNAME": "你的用户名",
        "TODO4AGENT_PASSWORD": "你的密码"
      }
    }
  }
}
凭据必填：缺失或校验失败将以非零码退出；首次运行数据库会自动创建初始账号。
可用工具（分组/任务增删改查、导入导出等）见软件内「Agent 接入」页。

首次使用:
  初始账号 admin / admin123，登录后请尽快在「设置 → 用户」修改密码。
  WebUI 默认 http://127.0.0.1:3000（监听 0.0.0.0，可在设置中改为仅本机）。
"#
    );
    println!(
        "数据:\n  任务数据保存在本地 SQLite，当前路径:\n    {}\n  可用环境变量 TODO4AGENT_DB 指定其他位置。",
        db::db_path().display()
    );
    println!("\n文档: https://github.com/BlazeSnow/Todo4Agent");
}

fn run_desktop() {
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

            // 系统托盘：关闭窗口后应用驻留后台，通过托盘菜单显示/退出
            let show_item = MenuItem::with_id(app, "show", "显示主界面", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_item, &quit_item])?;

            let mut tray = TrayIconBuilder::with_id("main-tray")
                .tooltip("Todo4Agent")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => show_main_window(app),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        show_main_window(tray.app_handle());
                    }
                });
            if let Some(icon) = app.default_window_icon() {
                tray = tray.icon(icon.clone());
            }
            tray.build(app)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                // 关闭按钮隐藏到托盘（后台运行），退出请使用托盘菜单
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .run(tauri::generate_context!())
        .expect("Tauri 应用启动失败");
}

/// 显示并聚焦主窗口（托盘左键点击 / 托盘菜单"显示主界面"）
fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn parses_modes() {
        assert!(matches!(parse_mode(args(&[])), RunMode::Desktop));
        assert!(matches!(parse_mode(args(&["mcp"])), RunMode::Mcp));
        assert!(matches!(parse_mode(args(&["--mcp"])), RunMode::Mcp));
        assert!(matches!(parse_mode(args(&["serve"])), RunMode::Serve));
        assert!(matches!(parse_mode(args(&["--serve"])), RunMode::Serve));
        assert!(matches!(parse_mode(args(&["help"])), RunMode::Help));
        assert!(matches!(parse_mode(args(&["--help"])), RunMode::Help));
        assert!(matches!(parse_mode(args(&["-h"])), RunMode::Help));
        assert!(matches!(parse_mode(args(&["version"])), RunMode::Version));
        assert!(matches!(parse_mode(args(&["--version"])), RunMode::Version));
        assert!(matches!(parse_mode(args(&["-V"])), RunMode::Version));
        // 非选项参数（如文件路径）不改变默认桌面模式
        assert!(matches!(parse_mode(args(&["file.json"])), RunMode::Desktop));
        // 首个被识别的参数生效
        assert!(matches!(parse_mode(args(&["x.json", "mcp"])), RunMode::Mcp));
    }

    #[test]
    fn unknown_flag_is_usage_error() {
        match parse_mode(args(&["--bogus"])) {
            RunMode::UnknownFlag(a) => assert_eq!(a, "--bogus"),
            other => panic!("期望 UnknownFlag，实际 {other:?}"),
        }
        // 未知选项在模式参数之前时同样报错
        assert!(matches!(
            parse_mode(args(&["--bogus", "mcp"])),
            RunMode::UnknownFlag(_)
        ));
    }
}
