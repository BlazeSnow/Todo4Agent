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

// 加载 locales/ 下的 Fluent 语言包（编译期内嵌为静态 LOCALES，
// 缺失键回落中文）；查询入口见 lang 模块
fluent_i18n::i18n!("locales", fallback = "zh-CN");

mod api;
mod auth;
mod db;
mod lang;
mod mcp;

/// 命令行运行模式
#[derive(Debug, PartialEq)]
enum RunMode {
    /// Tauri 桌面应用（默认，无参数或仅非选项参数）
    Desktop,
    Serve,
    Mcp,
    Help,
    Version,
}

/// 解析后的命令行选项
#[derive(Debug, PartialEq)]
struct Cli {
    /// 运行模式：首个被识别的模式参数生效
    mode: RunMode,
    /// `--port` 指定的端口（1024-65535）：本次运行有效，优先于设置页保存的端口
    port: Option<u16>,
}

/// 解析命令行参数。Err 为用法错误（附加人类可读说明，调用方以退出码 2 结束）
fn parse_args<I: IntoIterator<Item = String>>(args: I) -> Result<Cli, String> {
    let mut mode = RunMode::Desktop;
    let mut port = None;
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            // 模式参数：首个生效，后续忽略
            "mcp" | "--mcp" if mode == RunMode::Desktop => mode = RunMode::Mcp,
            "serve" | "--serve" if mode == RunMode::Desktop => mode = RunMode::Serve,
            "help" | "--help" | "-h" if mode == RunMode::Desktop => mode = RunMode::Help,
            "version" | "--version" | "-V" if mode == RunMode::Desktop => mode = RunMode::Version,
            "--port" => {
                let val = it.next().ok_or("--port 需要端口号，例如 --port 8080")?;
                port = Some(parse_port(&val)?);
            }
            other => {
                if let Some(v) = other.strip_prefix("--port=") {
                    port = Some(parse_port(v)?);
                } else if other.starts_with('-') {
                    return Err(format!("未知参数: {other}"));
                }
                // 非选项参数（如拖放的文件路径）忽略，维持默认桌面模式
            }
        }
    }
    Ok(Cli { mode, port })
}

/// 校验端口：1024-65535（与设置页的端口范围一致）
fn parse_port(s: &str) -> Result<u16, String> {
    let invalid = || format!("端口必须是 1024-65535 的整数：{s}");
    let n: u16 = s.parse().map_err(|_| invalid())?;
    if !(1024..=65535).contains(&n) {
        return Err(invalid());
    }
    Ok(n)
}

fn main() {
    let cli = match parse_args(std::env::args().skip(1)) {
        Ok(c) => c,
        Err(msg) => {
            eprintln!("{msg}\n");
            print_help();
            std::process::exit(2);
        }
    };
    match cli.mode {
        RunMode::Mcp => {
            // MCP 为 stdio 服务，不监听端口，忽略 --port
            let conn = db::open(&db::db_path()).expect("打开数据库失败");
            mcp::serve(&conn);
        }
        RunMode::Serve => api::serve_blocking(cli.port),
        RunMode::Help => print_help(),
        RunMode::Version => println!("todo4agent {}", env!("CARGO_PKG_VERSION")),
        RunMode::Desktop => run_desktop(cli.port),
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
  todo4agent              启动桌面应用（后台同时提供 WebUI 服务，默认 3000 端口；
                          已在运行时不会开出第二个实例，而是唤起已有窗口）
  todo4agent serve        无界面启动 WebUI / HTTP API（默认 3000 端口，占用时顺延）
  todo4agent mcp          启动 MCP Server（stdio，供 Agent 客户端连接）
  todo4agent help         显示本帮助
  todo4agent version      显示版本号

选项:
  --port <端口>           指定 WebUI/API 监听端口（1024-65535），如 todo4agent serve --port 8080；
                          本次运行有效，优先于设置页保存的端口；适用于桌面与 serve 模式

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

fn run_desktop(port_override: Option<u16>) {
    let dev = std::env::var("TAURI_ENV_DEBUG").map(|v| v == "true").unwrap_or(false);

    tauri::Builder::default()
        // 桌面端单实例：重复启动时唤起已有主窗口，第二个进程随即退出。
        // 必须最先注册——插件在 build 阶段完成检测，早于下方 setup（启动 HTTP 服务、
        // 建窗口），第二个进程因此不会抢占顺延端口或闪现窗口。
        // serve / mcp 模式不构建 Tauri 应用，可继续多实例并行。
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_main_window(app);
        }))
        .setup(move |app| {
            // 能走到这里的一定是首个实例：先启动 HTTP 服务，再按实际端口创建窗口
            let port = api::spawn_server(port_override);
            let url = if dev {
                "http://localhost:3001".to_string()
            } else {
                format!("http://127.0.0.1:{port}")
            };
            let window = tauri::WebviewWindowBuilder::new(
                app,
                "main",
                tauri::WebviewUrl::External(url.parse().expect("非法窗口 URL")),
            )
            .title("Todo4Agent")
            .inner_size(1100.0, 720.0)
            .min_inner_size(800.0, 600.0)
            // 先隐藏：待按系统深浅色设置好窗口/网页背景后再显示，
            // 避免深色模式下网页首帧绘制前出现白屏闪烁
            .visible(false)
            .build()?;
            let bg = match window.theme() {
                Ok(tauri::Theme::Dark) => tauri::utils::config::Color(0x12, 0x12, 0x12, 0xff),
                _ => tauri::utils::config::Color(0xff, 0xff, 0xff, 0xff),
            };
            let _ = window.set_background_color(Some(bg));
            let _ = window.show();

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

    fn parse(v: &[&str]) -> Cli {
        parse_args(args(v)).expect("parse_args 应成功")
    }

    #[test]
    fn parses_modes() {
        assert_eq!(parse(&[]), Cli { mode: RunMode::Desktop, port: None });
        assert_eq!(parse(&["mcp"]).mode, RunMode::Mcp);
        assert_eq!(parse(&["--mcp"]).mode, RunMode::Mcp);
        assert_eq!(parse(&["serve"]).mode, RunMode::Serve);
        assert_eq!(parse(&["--serve"]).mode, RunMode::Serve);
        assert_eq!(parse(&["help"]).mode, RunMode::Help);
        assert_eq!(parse(&["--help"]).mode, RunMode::Help);
        assert_eq!(parse(&["-h"]).mode, RunMode::Help);
        assert_eq!(parse(&["version"]).mode, RunMode::Version);
        assert_eq!(parse(&["--version"]).mode, RunMode::Version);
        assert_eq!(parse(&["-V"]).mode, RunMode::Version);
        // 非选项参数（如文件路径）不改变默认桌面模式
        assert_eq!(parse(&["file.json"]).mode, RunMode::Desktop);
        // 首个被识别的模式参数生效
        assert_eq!(parse(&["x.json", "mcp"]).mode, RunMode::Mcp);
        assert_eq!(parse(&["serve", "mcp"]).mode, RunMode::Serve);
    }

    #[test]
    fn parses_port() {
        assert_eq!(parse(&["--port", "8080"]).port, Some(8080));
        assert_eq!(parse(&["--port=9000"]).port, Some(9000));
        // 与模式参数的组合、顺序无关
        assert_eq!(
            parse(&["serve", "--port", "8080"]),
            Cli { mode: RunMode::Serve, port: Some(8080) }
        );
        assert_eq!(parse(&["--port=8080", "serve"]).mode, RunMode::Serve);
        // 范围边界
        assert_eq!(parse(&["--port", "1024"]).port, Some(1024));
        assert_eq!(parse(&["--port", "65535"]).port, Some(65535));
        // 非法值：越界 / 非数字 / 缺参
        assert!(parse_args(args(&["--port", "1023"])).is_err());
        assert!(parse_args(args(&["--port", "65536"])).is_err());
        assert!(parse_args(args(&["--port", "abc"])).is_err());
        assert!(parse_args(args(&["--port=8o80"])).is_err());
        assert!(parse_args(args(&["--port"])).is_err());
    }

    #[test]
    fn unknown_flag_is_usage_error() {
        let err = parse_args(args(&["--bogus"])).unwrap_err();
        assert!(err.contains("--bogus"));
        // 未知选项在模式参数之前时同样报错
        assert!(parse_args(args(&["--bogus", "mcp"])).is_err());
    }
}
