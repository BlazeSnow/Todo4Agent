use axum::{
    extract::{Extension, Json, State},
    http::{HeaderMap, StatusCode},
};
use serde::Deserialize;
use serde_json::json;

use super::*;
use crate::db;

// ---------- 认证 ----------

/// 认证状态（公开接口）：按请求头 token 判定当前会话
pub async fn auth_status(State(st): State<SharedState>, headers: HeaderMap) -> ApiResult {
    let c = st.db.lock().unwrap();
    let has_default_password = db::has_default_password_user(&c).unwrap_or(false);
    let allow_register = db::get_allow_register(&c).unwrap_or(true);
    let uid = bearer_token(&headers).and_then(|t| db::session_user_id(&c, t).ok().flatten());
    let username = uid.and_then(|uid| {
        db::list_users(&c)
            .ok()
            .and_then(|us| us.into_iter().find(|u| u.id == uid))
            .map(|u| u.username)
    });
    ok_json(json!({
        "user_id": uid,
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
        Ok(Some(user)) => match db::issue_session(&c, user.id) {
            Ok(token) => ok_json(json!({
                "token": token,
                "user_id": user.id,
                "username": user.username
            })),
            Err(e) => internal(e),
        },
        Ok(None) => err(StatusCode::UNAUTHORIZED, "用户名或密码错误"),
        Err(e) => internal(e),
    }
}

/// 注册新用户；新用户拥有独立数据空间
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
            // 新用户自带系统分组「无分组」（分组被删除时其任务的去处）
            if let Err(e) = db::ensure_no_group(&c, user.id) {
                return internal(e);
            }
            match db::issue_session(&c, user.id) {
                Ok(token) => ok_json(json!({
                    "token": token,
                    "user_id": user.id,
                    "username": user.username
                })),
                Err(e) => internal(e),
            }
        }
        Err(e) if db::is_unique_violation(&e) => err(StatusCode::CONFLICT, "用户名已存在"),
        Err(e) => internal(e),
    }
}

/// 登出：撤销当前 token（含数据库会话）
pub async fn auth_logout(State(st): State<SharedState>, headers: HeaderMap) -> ApiResult {
    if let Some(t) = bearer_token(&headers) {
        let c = st.db.lock().unwrap();
        let _ = db::delete_session(&c, t);
    }
    ok_json(json!({ "ok": true }))
}

#[derive(Deserialize)]
pub struct PasswordInput {
    old_password: String,
    new_password: String,
}

/// 修改当前用户密码；成功后吊销该用户的其他会话（当前登录保留）
pub async fn auth_password(
    State(st): State<SharedState>,
    Extension(cur): Extension<CurrentUser>,
    headers: HeaderMap,
    Json(body): Json<PasswordInput>,
) -> ApiResult {
    let uid = cur.0;
    if body.new_password.len() < 4 {
        return err(StatusCode::BAD_REQUEST, "新密码至少 4 位");
    }
    let c = st.db.lock().unwrap();
    match db::change_user_password(&c, uid, &body.old_password, &body.new_password) {
        Ok(true) => {
            let keep = bearer_token(&headers).map(String::from);
            let _ = db::delete_user_sessions(&c, uid, keep.as_deref());
            ok_json(json!({ "ok": true }))
        }
        Ok(false) => err(StatusCode::BAD_REQUEST, "原密码错误"),
        Err(e) => internal(e),
    }
}
