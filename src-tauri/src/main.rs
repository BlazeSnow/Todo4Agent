//! Todo4Agent 入口。
//! 三种模式：
//! - 默认：Tauri 桌面应用（后台启动 HTTP 服务，窗口加载 WebUI）
//! - `serve`：headless HTTP 服务（WebUI 于 3000 端口）
//! - `mcp`：MCP stdio 服务（供 Agent 连接操作任务清单）

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

/// 显示并聚焦主窗口（托盘左键点击 / 托盘菜单"显示主界面"）
fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

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