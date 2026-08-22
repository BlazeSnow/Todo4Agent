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
    ok_json(json!({
        "port": db::get_port_setting(&c).unwrap_or(db::DEFAULT_PORT),
        "effective_port": st.effective_port,
        "webui_lan": db::get_webui_lan(&c).unwrap_or(true),
        "allow_register": db::get_allow_register(&c).unwrap_or(true)
    }))
}

#[derive(Deserialize, Default)]
pub struct SettingsInput {
    port: Option<u16>,
    webui_lan: Option<bool>,
    allow_register: Option<bool>,
}

/// 保存服务设置：只更新传入的字段（webui_lan 重启后生效，其余立即生效）
pub async fn update_settings(State(st): State<SharedState>, Json(body): Json<SettingsInput>) -> ApiResult {
    if let Some(p) = body.port {
        if !(1024..=65535).contains(&p) {
            return err(StatusCode::BAD_REQUEST, "端口范围：1024-65535");
        }
    }
    let c = st.db.lock().unwrap();
    if let Some(p) = body.port {
        if let Err(e) = db::set_setting(&c, db::SETTINGS_PORT_KEY, &p.to_string()) {
            return internal(e);
        }
    }
    if let Some(v) = body.webui_lan {
        if let Err(e) = db::set_setting(&c, db::SETTINGS_WEBUI_LAN_KEY, if v { "1" } else { "0" })
        {
            return internal(e);
        }
    }
    if let Some(v) = body.allow_register {
        if let Err(e) = db::set_setting(&c, db::SETTINGS_ALLOW_REGISTER_KEY, if v { "1" } else { "0" })
        {
            return internal(e);
        }
    }
    ok_json(json!({
        "port": db::get_port_setting(&c).unwrap_or(db::DEFAULT_PORT),
        "webui_lan": db::get_webui_lan(&c).unwrap_or(true),
        "allow_register": db::get_allow_register(&c).unwrap_or(true)
    }))
}
