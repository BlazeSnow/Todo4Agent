use axum::{
    extract::{Extension, Json, State},
    http::StatusCode,
};
use serde::Deserialize;
use serde_json::json;

use super::*;
use crate::db;

// ---------- 导出 / 导入 ----------

pub async fn export_json(
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
pub async fn import_json(
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


// ---------- 设置 ----------

pub async fn get_settings(State(st): State<SharedState>) -> ApiResult {
    let c = st.db.lock().unwrap();
    let port = db::get_port_setting(&c).unwrap_or(db::DEFAULT_PORT);
    ok_json(json!({
        "port": port,
        "effective_port": st.effective_port
    }))
}

#[derive(Deserialize)]
pub struct PortInput {
    port: u16,
}

/// 保存端口配置（重启应用后生效）
pub async fn update_settings(State(st): State<SharedState>, Json(body): Json<PortInput>) -> ApiResult {
    if !(1024..=65535).contains(&body.port) {
        return err(StatusCode::BAD_REQUEST, "端口范围：1024-65535");
    }
    let c = st.db.lock().unwrap();
    match db::set_setting(&c, db::SETTINGS_PORT_KEY, &body.port.to_string()) {
        Ok(()) => ok_json(json!({ "port": body.port })),
        Err(e) => internal(e),
    }
}
