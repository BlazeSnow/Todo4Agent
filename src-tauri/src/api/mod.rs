//! HTTP API（axum）与内嵌 WebUI 静态资源服务。
//! 多用户模型：数据按当前登录用户隔离，除登录/注册/状态外的 /api 接口
//! 均需 Bearer token。静态资源始终可访问（登录页由前端渲染）。
//! 应用启动即播种初始用户 admin，故不存在无用户的“本地模式”。

use axum::{
    extract::{Request, State},
    http::{header, HeaderMap, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{delete, get, patch, post},
    Json, Router,
};
use rusqlite::Connection;
use rust_embed::RustEmbed;
use serde_json::{json, Value};
use std::{
    convert::Infallible,
    future::{ready, Ready},
    sync::{Arc, Mutex},
    task::{Context, Poll},
};
use tower::Service;

use crate::db;
use crate::auth as auth_crate;

pub struct AppState {
    pub db: Mutex<Connection>,
    /// 实际监听的端口（供设置页展示）
    pub effective_port: u16,
    /// 会话（token -> user_id）
    pub sessions: auth_crate::Sessions,
}
pub type SharedState = Arc<AppState>;

/// 中间件注入的当前登录用户（受保护接口必经认证，恒有值）
#[derive(Clone, Copy, Debug)]
pub struct CurrentUser(pub i64);

type ApiResult = (StatusCode, Json<Value>);

pub(crate) fn ok_json(v: Value) -> ApiResult {
    (StatusCode::OK, Json(v))
}

pub(crate) fn err(code: StatusCode, msg: &str) -> ApiResult {
    (code, Json(json!({ "error": msg })))
}

pub(crate) fn internal(e: rusqlite::Error) -> ApiResult {
    err(
        StatusCode::INTERNAL_SERVER_ERROR,
        &format!("数据库错误: {e}"),
    )
}

/// 从请求头提取 Bearer token
pub(crate) fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
}


pub mod auth;
pub mod groups;
pub mod misc;

pub use auth::*;
pub use groups::*;
pub use misc::*;
pub use tasks::*;
pub use trash::*;
pub mod tasks;
pub mod trash;

async fn require_auth(State(st): State<SharedState>, mut req: Request, next: Next) -> Response {
    let path = req.uri().path().to_string();
    let is_api = path.starts_with("/api");
    // 静态资源与认证相关接口公开；其余 /api 接口需要有效 Bearer token
    let public =
        !is_api || matches!(path.as_str(), "/api/auth/status" | "/api/auth/login" | "/api/auth/register");
    if public {
        return next.run(req).await;
    }
    let unauthorized =
        (StatusCode::UNAUTHORIZED, Json(json!({ "error": "未登录或登录已失效" }))).into_response();
    match bearer_token(req.headers()).and_then(|t| st.sessions.user_id(t)) {
        Some(uid) => {
            req.extensions_mut().insert(CurrentUser(uid));
            next.run(req).await
        }
        None => unauthorized,
    }
}

// ---------- 路由与内嵌静态资源 ----------

fn api_router(state: SharedState) -> Router {
    Router::new()
        .route("/groups", get(list_groups).post(create_group))
        .route("/groups/reorder", post(reorder_groups))
        .route("/groups/{id}", patch(rename_group).delete(delete_group))
        .route("/groups/{id}/restore", post(restore_group))
        .route("/groups/{id}/purge", delete(purge_group))
        .route("/tasks", get(list_tasks).post(create_task))
        .route("/tasks/reorder/{group_id}", post(reorder_tasks))
        .route("/tasks/{id}", patch(update_task).delete(delete_task))
        .route("/tasks/{id}/restore", post(restore_task))
        .route("/tasks/{id}/purge", delete(purge_task))
        .route("/trash", get(get_trash).delete(empty_trash))
        .route("/export", get(export_json))
        .route("/import", post(import_json))
        .route("/settings", get(get_settings).patch(update_settings))
        .route("/auth/status", get(auth_status))
        .route("/auth/login", post(auth_login))
        .route("/auth/register", post(auth_register))
        .route("/auth/logout", post(auth_logout))
        .route("/auth/password", post(auth_password))
        .with_state(state)
}

/// 前端构建产物（编译期内嵌于二进制）
#[derive(Clone, RustEmbed)]
#[folder = "../dist"]
struct EmbeddedAssets;

/// 将内嵌资源作为后备服务：命中文件名则返回，未命中回退 index.html（SPA 路由）
impl Service<axum::extract::Request> for EmbeddedAssets {
    type Response = Response;
    type Error = Infallible;
    type Future = Ready<Result<Response, Infallible>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: axum::extract::Request) -> Self::Future {
        let path = req.uri().path().trim_start_matches('/');
        let path = if path.is_empty() { "index.html" } else { path };
        let asset = EmbeddedAssets::get(path).or_else(|| EmbeddedAssets::get("index.html"));
        let response = match asset {
            Some(f) => Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, f.metadata.mimetype())
                .header(header::CACHE_CONTROL, "no-cache")
                .body(axum::body::Body::from(f.data.into_owned()))
                .unwrap(),
            None => Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(axum::body::Body::from("404 Not Found"))
                .unwrap(),
        };
        ready(Ok(response))
    }
}

fn app(state: SharedState) -> Router {
    Router::new()
        .nest("/api", api_router(state.clone()))
        .fallback_service(EmbeddedAssets)
        .layer(middleware::from_fn_with_state(state, require_auth))
}

// ---------- 端口与启动 ----------

/// 从配置端口开始绑定（`lan=true` 监听 0.0.0.0，否则仅 127.0.0.1；
/// 占用时顺延最多 10 个端口），返回监听器与实际端口
pub async fn bind_tokio(preferred: u16, lan: bool) -> std::io::Result<(tokio::net::TcpListener, u16)> {
    let bind_addr: &str = if lan { "0.0.0.0" } else { "127.0.0.1" };
    let end = preferred.saturating_add(10).min(65535);
    for port in preferred..=end {
        match tokio::net::TcpListener::bind((bind_addr, port)).await {
            Ok(l) => return Ok((l, port)),
            Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => continue,
            Err(e) => return Err(e),
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::AddrInUse,
        "端口被占用（含顺延 10 个端口）",
    ))
}

/// 在当前线程阻塞运行 HTTP 服务（headless serve 模式）
pub fn serve_blocking() {
    let rt = tokio::runtime::Runtime::new().expect("创建 tokio runtime 失败");
    rt.block_on(async move {
        let conn = db::open(&db::db_path()).expect("打开数据库失败");
        let preferred = db::get_port_setting(&conn).unwrap_or(db::DEFAULT_PORT);
        let lan = db::get_webui_lan(&conn).unwrap_or(true);
        let (listener, port) = bind_tokio(preferred, lan).await.expect("绑定端口失败");
        if lan {
            println!("Todo4Agent WebUI: http://127.0.0.1:{port} （已监听 0.0.0.0，局域网可访问）");
        } else {
            println!("Todo4Agent WebUI: http://127.0.0.1:{port} （仅本机可访问）");
        }
        let sessions = auth_crate::Sessions::default();
        sessions.load_from_db(&conn);
        let state = Arc::new(AppState {
            db: Mutex::new(conn),
            effective_port: port,
            sessions,
        });
        if let Err(e) = axum::serve(listener, app(state)).await {
            eprintln!("HTTP 服务错误: {e}");
        }
    });
}

/// 在后台线程运行 HTTP 服务，返回实际端口（tauri 桌面模式使用）
pub fn spawn_server() -> u16 {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("创建 tokio runtime 失败");
        rt.block_on(async move {
            let conn = db::open(&db::db_path()).expect("打开数据库失败");
            let preferred = db::get_port_setting(&conn).unwrap_or(db::DEFAULT_PORT);
            let lan = db::get_webui_lan(&conn).unwrap_or(true);
            let (listener, port) = bind_tokio(preferred, lan).await.expect("绑定端口失败");
            if lan {
                println!("Todo4Agent WebUI: http://127.0.0.1:{port} （已监听 0.0.0.0，局域网可访问）");
            } else {
                println!("Todo4Agent WebUI: http://127.0.0.1:{port} （仅本机可访问）");
            }
            let _ = tx.send(port);
            let sessions = auth_crate::Sessions::default();
            sessions.load_from_db(&conn);
            let state = Arc::new(AppState {
                db: Mutex::new(conn),
                effective_port: port,
                sessions,
            });
            if let Err(e) = axum::serve(listener, app(state)).await {
                eprintln!("HTTP 服务错误: {e}");
            }
        });
    });
    rx.recv().expect("服务线程启动失败")
}
