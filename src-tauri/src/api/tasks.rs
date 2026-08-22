use axum::{
    extract::{Extension, Json, Path, Query, State},
    http::StatusCode,
};
use serde::Deserialize;
use serde_json::json;

use super::*;
use crate::db;

// ---------- 任务 ----------

#[derive(Deserialize)]
pub struct TasksQuery {
    group_id: Option<i64>,
}

#[derive(Deserialize)]
pub struct TaskInput {
    group_id: i64,
    title: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    due_at: Option<String>,
}

pub async fn list_tasks(
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

pub async fn create_task(
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

pub async fn update_task(
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

pub async fn delete_task(
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
pub struct ReorderInput {
    task_ids: Vec<i64>,
}

/// 重排某分组内的任务（按 task_ids 的顺序）
pub async fn reorder_tasks(
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
