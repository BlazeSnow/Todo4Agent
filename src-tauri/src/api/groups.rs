use axum::{
    extract::{Extension, Json, Path, State},
    http::StatusCode,
};
use serde::Deserialize;
use serde_json::json;

use super::*;
use crate::db;

// ---------- 分组 ----------

pub async fn list_groups(
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
pub struct GroupCreate {
    name: String,
    /// 分组描述（可选）：说明该清单的用途
    #[serde(default)]
    description: String,
}

pub async fn create_group(
    State(st): State<SharedState>,
    Extension(cur): Extension<CurrentUser>,
    Json(body): Json<GroupCreate>,
) -> ApiResult {
    let name = body.name.trim();
    if name.is_empty() {
        return err(StatusCode::BAD_REQUEST, "分组名不能为空");
    }
    let c = st.db.lock().unwrap();
    match db::create_group(&c, cur.0, name, body.description.trim()) {
        Ok(group) => ok_json(json!(group)),
        Err(e) if db::is_unique_violation(&e) => err(StatusCode::CONFLICT, "分组名已存在"),
        Err(e) => internal(e),
    }
}

#[derive(Deserialize)]
pub struct GroupUpdate {
    name: Option<String>,
    /// 分组描述：说明该清单的用途，可为空
    description: Option<String>,
    /// 清单锁定：锁定后 Agent 无法通过 MCP 编辑该清单，界面编辑不受影响
    locked: Option<bool>,
}

/// 更新分组：重命名 / 修改描述 / 切换锁定（只处理传入的字段）
pub async fn update_group(
    State(st): State<SharedState>,
    Extension(cur): Extension<CurrentUser>,
    Path(id): Path<i64>,
    Json(body): Json<GroupUpdate>,
) -> ApiResult {
    if let Some(name) = &body.name {
        if name.trim().is_empty() {
            return err(StatusCode::BAD_REQUEST, "分组名不能为空");
        }
    }
    if body.name.is_none() && body.description.is_none() && body.locked.is_none() {
        return err(StatusCode::BAD_REQUEST, "没有需要更新的字段");
    }
    let c = st.db.lock().unwrap();
    if let Some(name) = &body.name {
        match db::rename_group(&c, cur.0, id, name.trim()) {
            Ok(Some(_)) => {}
            Ok(None) => return err(StatusCode::NOT_FOUND, "分组不存在"),
            Err(e) if db::is_unique_violation(&e) => {
                return err(StatusCode::CONFLICT, "分组名已存在")
            }
            Err(e) => return internal(e),
        }
    }
    if let Some(description) = &body.description {
        match db::set_group_description(&c, cur.0, id, description.trim()) {
            Ok(true) => {}
            Ok(false) => return err(StatusCode::NOT_FOUND, "分组不存在"),
            Err(e) => return internal(e),
        }
    }
    if let Some(locked) = body.locked {
        match db::set_group_locked(&c, cur.0, id, locked) {
            Ok(true) => {}
            Ok(false) => return err(StatusCode::NOT_FOUND, "分组不存在"),
            Err(e) => return internal(e),
        }
    }
    match db::get_group(&c, cur.0, id) {
        Ok(Some(group)) => ok_json(json!(group)),
        Ok(None) => err(StatusCode::NOT_FOUND, "分组不存在"),
        Err(e) => internal(e),
    }
}

pub async fn delete_group(
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
pub struct GroupReorderInput {
    group_ids: Vec<i64>,
}

/// 重排当前用户的分组（按 group_ids 的顺序）
pub async fn reorder_groups(
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
