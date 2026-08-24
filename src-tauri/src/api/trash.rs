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
    lang: Lang,
) -> ApiResult {
    let c = st.db.lock().unwrap();
    match db::list_trash(&c, cur.0) {
        Ok((groups, tasks)) => ok_json(json!({ "groups": groups, "tasks": tasks })),
        Err(e) => internal(lang, e),
    }
}

/// 清空回收站（当前用户的已删除数据）
pub async fn empty_trash(
    State(st): State<SharedState>,
    Extension(cur): Extension<CurrentUser>,
    lang: Lang,
) -> ApiResult {
    let c = st.db.lock().unwrap();
    match db::empty_trash(&c, cur.0) {
        Ok(()) => ok_json(json!({ "ok": true })),
        Err(e) => internal(lang, e),
    }
}

pub async fn restore_task(
    State(st): State<SharedState>,
    Extension(cur): Extension<CurrentUser>,
    lang: Lang,
    Path(id): Path<i64>,
) -> ApiResult {
    let c = st.db.lock().unwrap();
    match db::restore_task(&c, cur.0, id) {
        Ok(true) => ok_json(json!({ "ok": true })),
        Ok(false) => err_l(lang, StatusCode::NOT_FOUND, "任务不在回收站", "Task is not in the trash"),
        Err(e) => internal(lang, e),
    }
}

pub async fn purge_task(
    State(st): State<SharedState>,
    Extension(cur): Extension<CurrentUser>,
    lang: Lang,
    Path(id): Path<i64>,
) -> ApiResult {
    let c = st.db.lock().unwrap();
    match db::purge_task(&c, cur.0, id) {
        Ok(true) => ok_json(json!({ "ok": true })),
        Ok(false) => err_l(lang, StatusCode::NOT_FOUND, "任务不存在", "Task not found"),
        Err(e) => internal(lang, e),
    }
}

pub async fn restore_group(
    State(st): State<SharedState>,
    Extension(cur): Extension<CurrentUser>,
    lang: Lang,
    Path(id): Path<i64>,
) -> ApiResult {
    let c = st.db.lock().unwrap();
    match db::restore_group(&c, cur.0, id) {
        Ok(None) => err_l(lang, StatusCode::NOT_FOUND, "分组不在回收站", "Group is not in the trash"),
        // 原名被占用时自动重命名，renamed_to 告知前端新名字
        Ok(Some(None)) => ok_json(json!({ "ok": true })),
        Ok(Some(Some(new_name))) => ok_json(json!({ "ok": true, "renamed_to": new_name })),
        Err(e) => internal(lang, e),
    }
}

pub async fn purge_group(
    State(st): State<SharedState>,
    Extension(cur): Extension<CurrentUser>,
    lang: Lang,
    Path(id): Path<i64>,
) -> ApiResult {
    let c = st.db.lock().unwrap();
    // 系统分组不可清理，先行拦截给出可读错误（db 层同样兜底）
    if let Ok(Some(g)) = db::get_group(&c, cur.0, id) {
        if g.name == db::NO_GROUP {
            return err_l(
                lang,
                StatusCode::CONFLICT,
                "系统分组「无分组」不可删除",
                "The system group \"Ungrouped\" cannot be deleted",
            );
        }
    }
    match db::purge_group(&c, cur.0, id) {
        Ok(true) => ok_json(json!({ "ok": true })),
        Ok(false) => err_l(lang, StatusCode::NOT_FOUND, "分组不存在", "Group not found"),
        Err(e) => internal(lang, e),
    }
}
