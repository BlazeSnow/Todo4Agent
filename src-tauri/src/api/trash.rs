use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
};

use serde_json::json;
use super::*;
use crate::db;

// ---------- 回收站 ----------

pub async fn get_trash(
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
pub async fn empty_trash(
    State(st): State<SharedState>,
    Extension(cur): Extension<CurrentUser>,
) -> ApiResult {
    let c = st.db.lock().unwrap();
    match db::empty_trash(&c, cur.0) {
        Ok(()) => ok_json(json!({ "ok": true })),
        Err(e) => internal(e),
    }
}

pub async fn restore_task(
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

pub async fn purge_task(
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

pub async fn restore_group(
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

pub async fn purge_group(
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
