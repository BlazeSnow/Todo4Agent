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
pub struct GroupName {
    name: String,
}

pub async fn create_group(
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

pub async fn rename_group(
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
