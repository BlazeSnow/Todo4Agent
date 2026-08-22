//! HTTP API（axum）与内嵌 WebUI 静态资源服务。
//! 生产环境（桌面 app 或 headless serve）监听 127.0.0.1:3000（被占用时顺延），
//! 同时提供 REST API 与前端构建产物（rust-embed 编译期内嵌）。

use axum::{
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::Response,
    routing::{delete, get, patch, post},
    Json, Router,
};
use rusqlite::Connection;
use rust_embed::RustEmbed;
use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    convert::Infallible,
    future::{ready, Ready},
    sync::{Arc, Mutex},
    task::{Context, Poll},
};
use tower::Service;

use crate::db;

pub struct AppState {
    pub db: Mutex<Connection>,
    /// 实际监听的端口（供设置页展示）
    pub effective_port: u16,
}
pub type SharedState = Arc<AppState>;

type ApiResult = (StatusCode, Json<Value>);

fn ok_json(v: Value) -> ApiResult {
    (StatusCode::OK, Json(v))
}

fn err(code: StatusCode, msg: &str) -> ApiResult {
    (code, Json(json!({ "error": msg })))
}

fn internal(e: rusqlite::Error) -> ApiResult {
    err(
        StatusCode::INTERNAL_SERVER_ERROR,
        &format!("数据库错误: {e}"),
    )
}

// ---------- 分组 ----------

async fn list_groups(State(st): State<SharedState>) -> ApiResult {
    let c = st.db.lock().unwrap();
    match db::list_groups(&c) {
        Ok(groups) => ok_json(json!({ "groups": groups })),
        Err(e) => internal(e),
    }
}

#[derive(Deserialize)]
struct GroupName {
    name: String,
}

async fn create_group(State(st): State<SharedState>, Json(body): Json<GroupName>) -> ApiResult {
    let name = body.name.trim();
    if name.is_empty() {
        return err(StatusCode::BAD_REQUEST, "分组名不能为空");
    }
    let c = st.db.lock().unwrap();
    match db::create_group(&c, name) {
        Ok(group) => ok_json(json!(group)),
        Err(e) if db::is_unique_violation(&e) => err(StatusCode::CONFLICT, "分组名已存在"),
        Err(e) => internal(e),
    }
}

async fn rename_group(
    State(st): State<SharedState>,
    Path(id): Path<i64>,
    Json(body): Json<GroupName>,
) -> ApiResult {
    let name = body.name.trim();
    if name.is_empty() {
        return err(StatusCode::BAD_REQUEST, "分组名不能为空");
    }
    let c = st.db.lock().unwrap();
    match db::rename_group(&c, id, name) {
        Ok(Some(group)) => ok_json(json!(group)),
        Ok(None) => err(StatusCode::NOT_FOUND, "分组不存在"),
        Err(e) if db::is_unique_violation(&e) => err(StatusCode::CONFLICT, "分组名已存在"),
        Err(e) => internal(e),
    }
}

async fn delete_group(State(st): State<SharedState>, Path(id): Path<i64>) -> ApiResult {
    let c = st.db.lock().unwrap();
    match db::delete_group(&c, id) {
        Ok(true) => ok_json(json!({ "ok": true })),
        Ok(false) => err(StatusCode::NOT_FOUND, "分组不存在"),
        Err(e) => internal(e),
    }
}

#[derive(Deserialize)]
struct GroupReorderInput {
    group_ids: Vec<i64>,
}

/// 重排所有分组（按 group_ids 的顺序）
async fn reorder_groups(
    State(st): State<SharedState>,
    Json(body): Json<GroupReorderInput>,
) -> ApiResult {
    if body.group_ids.is_empty() {
        return err(StatusCode::BAD_REQUEST, "group_ids 不能为空");
    }
    let c = st.db.lock().unwrap();
    match db::reorder_groups(&c, &body.group_ids) {
        Ok(()) => ok_json(json!({ "ok": true })),
        Err(e) => internal(e),
    }
}

// ---------- 任务 ----------

#[derive(Deserialize)]
struct TasksQuery {
    group_id: Option<i64>,
}

#[derive(Deserialize)]
struct TaskInput {
    group_id: i64,
    title: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    due_at: Option<String>,
}

async fn list_tasks(
    State(st): State<SharedState>,
    Query(q): Query<TasksQuery>,
) -> ApiResult {
    let c = st.db.lock().unwrap();
    match db::list_tasks(&c, q.group_id) {
        Ok(tasks) => ok_json(json!({ "tasks": tasks })),
        Err(e) => internal(e),
    }
}

async fn create_task(State(st): State<SharedState>, Json(body): Json<TaskInput>) -> ApiResult {
    let title = body.title.trim();
    if title.is_empty() {
        return err(StatusCode::BAD_REQUEST, "任务标题不能为空");
    }
    let c = st.db.lock().unwrap();
    match db::create_task(&c, body.group_id, title, body.description.trim(), body.due_at.as_deref()) {
        Ok(task) => ok_json(json!(task)),
        // 分组不存在等外键约束违反
        Err(e) if db::is_unique_violation(&e) => err(StatusCode::BAD_REQUEST, "分组不存在"),
        Err(e) => internal(e),
    }
}

async fn update_task(
    State(st): State<SharedState>,
    Path(id): Path<i64>,
    Json(patch): Json<db::TaskUpdate>,
) -> ApiResult {
    if let Some(status) = &patch.status {
        if status != "pending" && status != "done" {
            return err(StatusCode::BAD_REQUEST, "status 只能是 pending 或 done");
        }
    }
    let c = st.db.lock().unwrap();
    match db::update_task(&c, id, &patch) {
        Ok(Some(task)) => ok_json(json!(task)),
        Ok(None) => err(StatusCode::NOT_FOUND, "任务不存在"),
        Err(e) => internal(e),
    }
}

async fn delete_task(State(st): State<SharedState>, Path(id): Path<i64>) -> ApiResult {
    let c = st.db.lock().unwrap();
    match db::delete_task(&c, id) {
        Ok(true) => ok_json(json!({ "ok": true })),
        Ok(false) => err(StatusCode::NOT_FOUND, "任务不存在"),
        Err(e) => internal(e),
    }
}

#[derive(Deserialize)]
struct ReorderInput {
    task_ids: Vec<i64>,
}

/// 重排某分组内的任务（按 task_ids 的顺序）
async fn reorder_tasks(
    State(st): State<SharedState>,
    Path(group_id): Path<i64>,
    Json(body): Json<ReorderInput>,
) -> ApiResult {
    if body.task_ids.is_empty() {
        return err(StatusCode::BAD_REQUEST, "task_ids 不能为空");
    }
    let c = st.db.lock().unwrap();
    match db::reorder_tasks(&c, group_id, &body.task_ids) {
        Ok(()) => ok_json(json!({ "ok": true })),
        Err(e) => internal(e),
    }
}

// ---------- 导出 ----------

async fn export_json(State(st): State<SharedState>) -> ApiResult {
    let c = st.db.lock().unwrap();
    match db::export_all(&c) {
        Ok(doc) => ok_json(serde_json::to_value(doc).unwrap()),
        Err(e) => internal(e),
    }
}

/// 导入 JSON（同名分组并入，新分组新建）
async fn import_json(State(st): State<SharedState>, Json(doc): Json<db::ExportDoc>) -> ApiResult {
    if doc.groups.is_empty() {
        return err(StatusCode::BAD_REQUEST, "导入内容为空");
    }
    let c = st.db.lock().unwrap();
    match db::import_doc(&c, &doc) {
        Ok(r) => ok_json(serde_json::to_value(r).unwrap()),
        Err(e) => internal(e),
    }
}

// ---------- 回收站 ----------

async fn get_trash(State(st): State<SharedState>) -> ApiResult {
    let c = st.db.lock().unwrap();
    match db::list_trash(&c) {
        Ok((groups, tasks)) => ok_json(json!({ "groups": groups, "tasks": tasks })),
        Err(e) => internal(e),
    }
}

/// 清空回收站（彻底删除已删除的分组与任务）
async fn empty_trash(State(st): State<SharedState>) -> ApiResult {
    let c = st.db.lock().unwrap();
    match db::empty_trash(&c) {
        Ok(()) => ok_json(json!({ "ok": true })),
        Err(e) => internal(e),
    }
}

async fn restore_task(State(st): State<SharedState>, Path(id): Path<i64>) -> ApiResult {
    let c = st.db.lock().unwrap();
    match db::restore_task(&c, id) {
        Ok(true) => ok_json(json!({ "ok": true })),
        Ok(false) => err(StatusCode::NOT_FOUND, "任务不在回收站"),
        Err(e) => internal(e),
    }
}

async fn purge_task(State(st): State<SharedState>, Path(id): Path<i64>) -> ApiResult {
    let c = st.db.lock().unwrap();
    match db::purge_task(&c, id) {
        Ok(true) => ok_json(json!({ "ok": true })),
        Ok(false) => err(StatusCode::NOT_FOUND, "任务不存在"),
        Err(e) => internal(e),
    }
}

async fn restore_group(State(st): State<SharedState>, Path(id): Path<i64>) -> ApiResult {
    let c = st.db.lock().unwrap();
    match db::restore_group(&c, id) {
        Ok(true) => ok_json(json!({ "ok": true })),
        Ok(false) => err(StatusCode::NOT_FOUND, "分组不在回收站"),
        Err(e) => internal(e),
    }
}

async fn purge_group(State(st): State<SharedState>, Path(id): Path<i64>) -> ApiResult {
    let c = st.db.lock().unwrap();
    match db::purge_group(&c, id) {
        Ok(true) => ok_json(json!({ "ok": true })),
        Ok(false) => err(StatusCode::NOT_FOUND, "分组不存在"),
        Err(e) => internal(e),
    }
}

// ---------- 设置 ----------

async fn get_settings(State(st): State<SharedState>) -> ApiResult {
    let c = st.db.lock().unwrap();
    let port = db::get_port_setting(&c).unwrap_or(db::DEFAULT_PORT);
    ok_json(json!({
        "port": port,
        "effective_port": st.effective_port
    }))
}

#[derive(Deserialize)]
struct PortInput {
    port: u16,
}

/// 保存端口配置（重启应用后生效）
async fn update_settings(State(st): State<SharedState>, Json(body): Json<PortInput>) -> ApiResult {
    if !(1024..=65535).contains(&body.port) {
        return err(StatusCode::BAD_REQUEST, "端口范围：1024-65535");
    }
    let c = st.db.lock().unwrap();
    match db::set_setting(&c, db::SETTINGS_PORT_KEY, &body.port.to_string()) {
        Ok(()) => ok_json(json!({ "port": body.port })),
        Err(e) => internal(e),
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
        .with_state(state)
}

fn app(state: SharedState) -> Router {
    Router::new()
        .nest("/api", api_router(state))
        .fallback_service(EmbeddedAssets)
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

// ---------- 端口与启动 ----------

/// 从配置端口开始绑定 127.0.0.1（占用时顺延最多 10 个端口），返回监听器与实际端口
pub async fn bind_tokio(preferred: u16) -> std::io::Result<(tokio::net::TcpListener, u16)> {
    let end = preferred.saturating_add(10).min(65535);
    for port in preferred..=end {
        match tokio::net::TcpListener::bind(("127.0.0.1", port)).await {
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

/// 打开数据库并读取端口配置，连同数据库一起返回
fn open_db() -> (Mutex<Connection>, u16) {
    let conn = db::open(&db::db_path()).expect("打开数据库失败");
    let port = db::get_port_setting(&conn).unwrap_or(db::DEFAULT_PORT);
    (Mutex::new(conn), port)
}

/// 在当前线程阻塞运行 HTTP 服务（headless serve 模式）
pub fn serve_blocking() {
    let rt = tokio::runtime::Runtime::new().expect("创建 tokio runtime 失败");
    rt.block_on(async move {
        let (db, preferred) = open_db();
        let (listener, port) = bind_tokio(preferred).await.expect("绑定端口失败");
        println!("Todo4Agent WebUI: http://127.0.0.1:{port}");
        let state = Arc::new(AppState {
            db,
            effective_port: port,
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
            let (db, preferred) = open_db();
            let (listener, port) = bind_tokio(preferred).await.expect("绑定端口失败");
            println!("Todo4Agent WebUI: http://127.0.0.1:{port}");
            let _ = tx.send(port);
            let state = Arc::new(AppState {
                db,
                effective_port: port,
            });
            if let Err(e) = axum::serve(listener, app(state)).await {
                eprintln!("HTTP 服务错误: {e}");
            }
        });
    });
    rx.recv().expect("服务线程启动失败")
}