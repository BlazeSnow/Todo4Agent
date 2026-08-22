//! HTTP API（axum）与内嵌 WebUI 静态资源服务。
//! 多用户模型：未创建用户时为“本地模式”（免登录）；创建首个用户后进入多用户模式，
//! 所有数据按当前登录用户隔离，接口需要 Bearer token。静态资源始终可访问（登录页由前端渲染）。

use axum::{
    extract::{Extension, Path, Query, Request, State},
    http::{header, HeaderMap, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
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

use crate::{auth, db};

pub struct AppState {
    pub db: Mutex<Connection>,
    /// 实际监听的端口（供设置页展示）
    pub effective_port: u16,
    /// 会话（token -> user_id）
    pub sessions: auth::Sessions,
}
pub type SharedState = Arc<AppState>;

/// 中间件注入的当前用户（None = 本地模式）
#[derive(Clone, Copy, Debug)]
pub struct CurrentUser(pub Option<i64>);

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

/// 从请求头提取 Bearer token
fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
}

// ---------- 分组 ----------

async fn list_groups(
    State(st): State<SharedState>,
    Extension(cur): Extension<CurrentUser>,
) -> ApiResult {
    let uid = cur.0;
    let c = st.db.lock().unwrap();
    match db::list_groups(&c, uid) {
        Ok(groups) => ok_json(json!({ "groups": groups })),
        Err(e) => internal(e),
    }
}

#[derive(Deserialize)]
struct GroupName {
    name: String,
}

async fn create_group(
    State(st): State<SharedState>,
    Extension(cur): Extension<CurrentUser>,
    Json(body): Json<GroupName>,
) -> ApiResult {
    let name = body.name.trim();
    if name.is_empty() {
        return err(StatusCode::BAD_REQUEST, "分组名不能为空");
    }
    let c = st.db.lock().unwrap();
    match db::create_group(&c, cur.0, name) {
        Ok(group) => ok_json(json!(group)),
        Err(e) if db::is_unique_violation(&e) => err(StatusCode::CONFLICT, "分组名已存在"),
        Err(e) => internal(e),
    }
}

async fn rename_group(
    State(st): State<SharedState>,
    Extension(cur): Extension<CurrentUser>,
    Path(id): Path<i64>,
    Json(body): Json<GroupName>,
) -> ApiResult {
    let name = body.name.trim();
    if name.is_empty() {
        return err(StatusCode::BAD_REQUEST, "分组名不能为空");
    }
    let c = st.db.lock().unwrap();
    match db::rename_group(&c, cur.0, id, name) {
        Ok(Some(group)) => ok_json(json!(group)),
        Ok(None) => err(StatusCode::NOT_FOUND, "分组不存在"),
        Err(e) if db::is_unique_violation(&e) => err(StatusCode::CONFLICT, "分组名已存在"),
        Err(e) => internal(e),
    }
}

async fn delete_group(
    State(st): State<SharedState>,
    Extension(cur): Extension<CurrentUser>,
    Path(id): Path<i64>,
) -> ApiResult {
    let c = st.db.lock().unwrap();
    match db::delete_group(&c, cur.0, id) {
        Ok(true) => ok_json(json!({ "ok": true })),
        Ok(false) => err(StatusCode::NOT_FOUND, "分组不存在"),
        Err(e) => internal(e),
    }
}

#[derive(Deserialize)]
struct GroupReorderInput {
    group_ids: Vec<i64>,
}

/// 重排当前用户的分组（按 group_ids 的顺序）
async fn reorder_groups(
    State(st): State<SharedState>,
    Extension(cur): Extension<CurrentUser>,
    Json(body): Json<GroupReorderInput>,
) -> ApiResult {
    if body.group_ids.is_empty() {
        return err(StatusCode::BAD_REQUEST, "group_ids 不能为空");
    }
    let c = st.db.lock().unwrap();
    match db::reorder_groups(&c, cur.0, &body.group_ids) {
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
    Extension(cur): Extension<CurrentUser>,
    Query(q): Query<TasksQuery>,
) -> ApiResult {
    let c = st.db.lock().unwrap();
    match db::list_tasks(&c, cur.0, q.group_id) {
        Ok(tasks) => ok_json(json!({ "tasks": tasks })),
        Err(e) => internal(e),
    }
}

async fn create_task(
    State(st): State<SharedState>,
    Extension(cur): Extension<CurrentUser>,
    Json(body): Json<TaskInput>,
) -> ApiResult {
    let title = body.title.trim();
    if title.is_empty() {
        return err(StatusCode::BAD_REQUEST, "任务标题不能为空");
    }
    let c = st.db.lock().unwrap();
    match db::create_task(
        &c,
        cur.0,
        body.group_id,
        title,
        body.description.trim(),
        body.due_at.as_deref(),
    ) {
        Ok(task) => ok_json(json!(task)),
        Err(e) if e == rusqlite::Error::QueryReturnedNoRows => {
            err(StatusCode::BAD_REQUEST, "分组不存在")
        }
        Err(e) => internal(e),
    }
}

async fn update_task(
    State(st): State<SharedState>,
    Extension(cur): Extension<CurrentUser>,
    Path(id): Path<i64>,
    Json(patch): Json<db::TaskUpdate>,
) -> ApiResult {
    if let Some(status) = &patch.status {
        if status != "pending" && status != "done" {
            return err(StatusCode::BAD_REQUEST, "status 只能是 pending 或 done");
        }
    }
    let c = st.db.lock().unwrap();
    match db::update_task(&c, cur.0, id, &patch) {
        Ok(Some(task)) => ok_json(json!(task)),
        Ok(None) => err(StatusCode::NOT_FOUND, "任务不存在"),
        Err(e) => internal(e),
    }
}

async fn delete_task(
    State(st): State<SharedState>,
    Extension(cur): Extension<CurrentUser>,
    Path(id): Path<i64>,
) -> ApiResult {
    let c = st.db.lock().unwrap();
    match db::delete_task(&c, cur.0, id) {
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
    Extension(cur): Extension<CurrentUser>,
    Path(group_id): Path<i64>,
    Json(body): Json<ReorderInput>,
) -> ApiResult {
    if body.task_ids.is_empty() {
        return err(StatusCode::BAD_REQUEST, "task_ids 不能为空");
    }
    let c = st.db.lock().unwrap();
    match db::reorder_tasks(&c, cur.0, group_id, &body.task_ids) {
        Ok(()) => ok_json(json!({ "ok": true })),
        Err(e) if e == rusqlite::Error::QueryReturnedNoRows => {
            err(StatusCode::NOT_FOUND, "分组不存在")
        }
        Err(e) => internal(e),
    }
}

// ---------- 导出 / 导入 ----------

async fn export_json(
    State(st): State<SharedState>,
    Extension(cur): Extension<CurrentUser>,
) -> ApiResult {
    let c = st.db.lock().unwrap();
    match db::export_all(&c, cur.0) {
        Ok(doc) => ok_json(serde_json::to_value(doc).unwrap()),
        Err(e) => internal(e),
    }
}

/// 导入 JSON（同名分组并入，新分组新建；仅导入到当前用户）
async fn import_json(
    State(st): State<SharedState>,
    Extension(cur): Extension<CurrentUser>,
    Json(doc): Json<db::ExportDoc>,
) -> ApiResult {
    if doc.groups.is_empty() {
        return err(StatusCode::BAD_REQUEST, "导入内容为空");
    }
    let c = st.db.lock().unwrap();
    match db::import_doc(&c, cur.0, &doc) {
        Ok(r) => ok_json(serde_json::to_value(r).unwrap()),
        Err(e) => internal(e),
    }
}

// ---------- 回收站 ----------

async fn get_trash(
    State(st): State<SharedState>,
    Extension(cur): Extension<CurrentUser>,
) -> ApiResult {
    let c = st.db.lock().unwrap();
    match db::list_trash(&c, cur.0) {
        Ok((groups, tasks)) => ok_json(json!({ "groups": groups, "tasks": tasks })),
        Err(e) => internal(e),
    }
}

/// 清空回收站（当前用户的已删除数据）
async fn empty_trash(
    State(st): State<SharedState>,
    Extension(cur): Extension<CurrentUser>,
) -> ApiResult {
    let c = st.db.lock().unwrap();
    match db::empty_trash(&c, cur.0) {
        Ok(()) => ok_json(json!({ "ok": true })),
        Err(e) => internal(e),
    }
}

async fn restore_task(
    State(st): State<SharedState>,
    Extension(cur): Extension<CurrentUser>,
    Path(id): Path<i64>,
) -> ApiResult {
    let c = st.db.lock().unwrap();
    match db::restore_task(&c, cur.0, id) {
        Ok(true) => ok_json(json!({ "ok": true })),
        Ok(false) => err(StatusCode::NOT_FOUND, "任务不在回收站"),
        Err(e) => internal(e),
    }
}

async fn purge_task(
    State(st): State<SharedState>,
    Extension(cur): Extension<CurrentUser>,
    Path(id): Path<i64>,
) -> ApiResult {
    let c = st.db.lock().unwrap();
    match db::purge_task(&c, cur.0, id) {
        Ok(true) => ok_json(json!({ "ok": true })),
        Ok(false) => err(StatusCode::NOT_FOUND, "任务不存在"),
        Err(e) => internal(e),
    }
}

async fn restore_group(
    State(st): State<SharedState>,
    Extension(cur): Extension<CurrentUser>,
    Path(id): Path<i64>,
) -> ApiResult {
    let c = st.db.lock().unwrap();
    match db::restore_group(&c, cur.0, id) {
        Ok(true) => ok_json(json!({ "ok": true })),
        Ok(false) => err(StatusCode::NOT_FOUND, "分组不在回收站"),
        Err(e) => internal(e),
    }
}

async fn purge_group(
    State(st): State<SharedState>,
    Extension(cur): Extension<CurrentUser>,
    Path(id): Path<i64>,
) -> ApiResult {
    let c = st.db.lock().unwrap();
    match db::purge_group(&c, cur.0, id) {
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

// ---------- 认证 ----------

async fn auth_status(
    State(st): State<SharedState>,
    Extension(cur): Extension<CurrentUser>,
) -> ApiResult {
    let c = st.db.lock().unwrap();
    let users_exist = db::user_count(&c).unwrap_or(0) > 0;
    let has_default_password = db::has_default_password_user(&c).unwrap_or(false);
    let username = match cur.0 {
        Some(uid) => db::list_users(&c)
            .ok()
            .and_then(|us| us.into_iter().find(|u| u.id == uid))
            .map(|u| u.username),
        None => None,
    };
    ok_json(json!({
        "mode": if users_exist { "users" } else { "local" },
        "user_id": cur.0,
        "username": username,
        "default_password": has_default_password
    }))
}

#[derive(Deserialize)]
struct AuthLoginInput {
    username: String,
    password: String,
}

async fn auth_login(
    State(st): State<SharedState>,
    Json(body): Json<AuthLoginInput>,
) -> ApiResult {
    let c = st.db.lock().unwrap();
    match db::verify_user(&c, &body.username, &body.password) {
        Ok(Some(user)) => {
            let token = st.sessions.issue(user.id);
            ok_json(json!({
                "token": token,
                "user_id": user.id,
                "username": user.username
            }))
        }
        Ok(None) => err(StatusCode::UNAUTHORIZED, "用户名或密码错误"),
        Err(e) => internal(e),
    }
}

/// 注册新用户；首个用户自动接管本地模式遗留数据，后续用户拥有独立数据空间
async fn auth_register(
    State(st): State<SharedState>,
    Json(body): Json<AuthLoginInput>,
) -> ApiResult {
    let username = body.username.trim();
    if username.is_empty() {
        return err(StatusCode::BAD_REQUEST, "用户名不能为空");
    }
    if body.password.len() < 4 {
        return err(StatusCode::BAD_REQUEST, "密码至少 4 位");
    }
    let c = st.db.lock().unwrap();
    match db::create_user(&c, username, &body.password) {
        Ok(user) => {
            let token = st.sessions.issue(user.id);
            ok_json(json!({
                "token": token,
                "user_id": user.id,
                "username": user.username
            }))
        }
        Err(e) if db::is_unique_violation(&e) => err(StatusCode::CONFLICT, "用户名已存在"),
        Err(e) => internal(e),
    }
}

/// 登出：撤销当前 token
async fn auth_logout(State(st): State<SharedState>, headers: HeaderMap) -> ApiResult {
    if let Some(t) = bearer_token(&headers) {
        st.sessions.revoke(t);
    }
    ok_json(json!({ "ok": true }))
}

#[derive(Deserialize)]
struct PasswordInput {
    old_password: String,
    new_password: String,
}

/// 修改当前用户密码
async fn auth_password(
    State(st): State<SharedState>,
    Extension(cur): Extension<CurrentUser>,
    Json(body): Json<PasswordInput>,
) -> ApiResult {
    let Some(uid) = cur.0 else {
        return err(StatusCode::UNAUTHORIZED, "未登录");
    };
    if body.new_password.len() < 4 {
        return err(StatusCode::BAD_REQUEST, "新密码至少 4 位");
    }
    let c = st.db.lock().unwrap();
    match db::change_user_password(&c, uid, &body.old_password, &body.new_password) {
        Ok(true) => ok_json(json!({ "ok": true })),
        Ok(false) => err(StatusCode::BAD_REQUEST, "原密码错误"),
        Err(e) => internal(e),
    }
}

/// 认证/用户中间件：注入当前用户；多用户模式下非公开接口需 Bearer token
async fn require_auth(State(st): State<SharedState>, mut req: Request, next: Next) -> Response {
    let path = req.uri().path().to_string();
    let is_api = path.starts_with("/api");
    let public =
        !is_api || matches!(path.as_str(), "/api/auth/status" | "/api/auth/login" | "/api/auth/register");
    let unauthorized =
        (StatusCode::UNAUTHORIZED, Json(json!({ "error": "未登录或登录已失效" }))).into_response();

    // 多用户模式判定
    let users_exist = {
        let c = st.db.lock().unwrap();
        db::user_count(&c).unwrap_or(0) > 0
    };
    if !users_exist {
        // 本地模式：无登录要求
        req.extensions_mut().insert(CurrentUser(None));
        return next.run(req).await;
    }

    let uid = bearer_token(req.headers()).and_then(|t| st.sessions.user_id(t));
    if public {
        // 公开接口：带有效 token 则附带用户身份
        req.extensions_mut().insert(CurrentUser(uid));
        return next.run(req).await;
    }
    match uid {
        Some(uid) => {
            req.extensions_mut().insert(CurrentUser(Some(uid)));
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

/// 在当前线程阻塞运行 HTTP 服务（headless serve 模式）
pub fn serve_blocking() {
    let rt = tokio::runtime::Runtime::new().expect("创建 tokio runtime 失败");
    rt.block_on(async move {
        let conn = db::open(&db::db_path()).expect("打开数据库失败");
        let preferred = db::get_port_setting(&conn).unwrap_or(db::DEFAULT_PORT);
        let (listener, port) = bind_tokio(preferred).await.expect("绑定端口失败");
        println!("Todo4Agent WebUI: http://127.0.0.1:{port}");
        let state = Arc::new(AppState {
            db: Mutex::new(conn),
            effective_port: port,
            sessions: auth::Sessions::default(),
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
            let (listener, port) = bind_tokio(preferred).await.expect("绑定端口失败");
            println!("Todo4Agent WebUI: http://127.0.0.1:{port}");
            let _ = tx.send(port);
            let state = Arc::new(AppState {
                db: Mutex::new(conn),
                effective_port: port,
                sessions: auth::Sessions::default(),
            });
            if let Err(e) = axum::serve(listener, app(state)).await {
                eprintln!("HTTP 服务错误: {e}");
            }
        });
    });
    rx.recv().expect("服务线程启动失败")
}