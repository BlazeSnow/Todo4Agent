use axum::{
    extract::{Extension, Json, State},
    http::{HeaderMap, StatusCode},
};
use serde::Deserialize;
use serde_json::json;

use super::*;
use crate::db;

// ---------- 认证 ----------

pub async fn auth_status(
    State(st): State<SharedState>,
    Extension(cur): Extension<CurrentUser>,
) -> ApiResult {
    let c = st.db.lock().unwrap();
    let users_exist = db::user_count(&c).unwrap_or(0) > 0;
    let has_default_password = db::has_default_password_user(&c).unwrap_or(false);
    let allow_register = db::get_allow_register(&c).unwrap_or(true);
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
        "default_password": has_default_password,
        "allow_register": allow_register
    }))
}

#[derive(Deserialize)]
pub struct AuthLoginInput {
    username: String,
    password: String,
}

pub async fn auth_login(
    State(st): State<SharedState>,
    Json(body): Json<AuthLoginInput>,
) -> ApiResult {
    let c = st.db.lock().unwrap();
    match db::verify_user(&c, &body.username, &body.password) {
        Ok(Some(user)) => {
            let token = st.sessions.issue(&c, user.id);
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
pub async fn auth_register(
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
    if !db::get_allow_register(&c).unwrap_or(true) {
        return err(StatusCode::FORBIDDEN, "注册已关闭");
    }
    match db::create_user(&c, username, &body.password) {
        Ok(user) => {
            let token = st.sessions.issue(&c, user.id);
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

/// 登出：撤销当前 token（含数据库会话）
pub async fn auth_logout(State(st): State<SharedState>, headers: HeaderMap) -> ApiResult {
    if let Some(t) = bearer_token(&headers) {
        let c = st.db.lock().unwrap();
        st.sessions.revoke(&c, t);
    }
    ok_json(json!({ "ok": true }))
}

#[derive(Deserialize)]
pub struct PasswordInput {
    old_password: String,
    new_password: String,
}

/// 修改当前用户密码
pub async fn auth_password(
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
