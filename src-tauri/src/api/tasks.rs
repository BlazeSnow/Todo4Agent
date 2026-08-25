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
    lang: Lang,
    Query(q): Query<TasksQuery>,
) -> ApiResult {
    let c = st.db.lock().unwrap();
    match db::list_tasks(&c, cur.0, q.group_id) {
        Ok(tasks) => ok_json(json!({ "tasks": tasks })),
        Err(e) => internal(lang, e),
    }
}

pub async fn create_task(
    State(st): State<SharedState>,
    Extension(cur): Extension<CurrentUser>,
    lang: Lang,
    Json(body): Json<TaskInput>,
) -> ApiResult {
    let title = body.title.trim();
    if title.is_empty() {
        return err(StatusCode::BAD_REQUEST, &tr(lang, "task-title-empty"));
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
            err(StatusCode::BAD_REQUEST, &tr(lang, "group-not-found"))
        }
        Err(e) => internal(lang, e),
    }
}

pub async fn update_task(
    State(st): State<SharedState>,
    Extension(cur): Extension<CurrentUser>,
    lang: Lang,
    Path(id): Path<i64>,
    Json(patch): Json<db::TaskUpdate>,
) -> ApiResult {
    if let Some(status) = &patch.status {
        if status != "pending" && status != "done" {
            return err(StatusCode::BAD_REQUEST, &tr(lang, "status-invalid"));
        }
    }
    let c = st.db.lock().unwrap();
    match db::update_task(&c, cur.0, id, &patch) {
        Ok(Some(task)) => ok_json(json!(task)),
        Ok(None) => err(StatusCode::NOT_FOUND, &tr(lang, "task-not-found")),
        Err(e) => internal(lang, e),
    }
}

pub async fn delete_task(
    State(st): State<SharedState>,
    Extension(cur): Extension<CurrentUser>,
    lang: Lang,
    Path(id): Path<i64>,
) -> ApiResult {
    let c = st.db.lock().unwrap();
    match db::delete_task(&c, cur.0, id) {
        Ok(true) => ok_json(json!({ "ok": true })),
        Ok(false) => err(StatusCode::NOT_FOUND, &tr(lang, "task-not-found")),
        Err(e) => internal(lang, e),
    }
}

// ---------- 归档 ----------

/// 归档列表（时间线展示，按归档时间倒序）
pub async fn get_archive(
    State(st): State<SharedState>,
    Extension(cur): Extension<CurrentUser>,
    lang: Lang,
) -> ApiResult {
    let c = st.db.lock().unwrap();
    match db::list_archived(&c, cur.0) {
        Ok(tasks) => ok_json(json!({ "tasks": tasks })),
        Err(e) => internal(lang, e),
    }
}

/// 归档任务（从清单移入归档）
pub async fn archive_task(
    State(st): State<SharedState>,
    Extension(cur): Extension<CurrentUser>,
    lang: Lang,
    Path(id): Path<i64>,
) -> ApiResult {
    let c = st.db.lock().unwrap();
    match db::archive_task(&c, cur.0, id) {
        Ok(true) => ok_json(json!({ "ok": true })),
        Ok(false) => err(StatusCode::NOT_FOUND, &tr(lang, "task-not-found-or-archived")),
        Err(e) => internal(lang, e),
    }
}

/// 取消归档（回到原清单）
pub async fn unarchive_task(
    State(st): State<SharedState>,
    Extension(cur): Extension<CurrentUser>,
    lang: Lang,
    Path(id): Path<i64>,
) -> ApiResult {
    let c = st.db.lock().unwrap();
    match db::unarchive_task(&c, cur.0, id) {
        Ok(true) => ok_json(json!({ "ok": true })),
        Ok(false) => err(StatusCode::NOT_FOUND, &tr(lang, "task-not-archived")),
        Err(e) => internal(lang, e),
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
    lang: Lang,
    Path(group_id): Path<i64>,
    Json(body): Json<ReorderInput>,
) -> ApiResult {
    if body.task_ids.is_empty() {
        return err(StatusCode::BAD_REQUEST, &tr(lang, "task-ids-empty"));
    }
    let c = st.db.lock().unwrap();
    match db::reorder_tasks(&c, cur.0, group_id, &body.task_ids) {
        Ok(()) => ok_json(json!({ "ok": true })),
        Err(e) if e == rusqlite::Error::QueryReturnedNoRows => {
            err(StatusCode::NOT_FOUND, &tr(lang, "group-not-found"))
        }
        Err(e) => internal(lang, e),
    }
}
