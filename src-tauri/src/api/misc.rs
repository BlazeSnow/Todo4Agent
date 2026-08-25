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
    lang: Lang,
) -> ApiResult {
    let c = st.db.lock().unwrap();
    match db::export_all(&c, cur.0) {
        Ok(doc) => ok_json(serde_json::to_value(doc).unwrap()),
        Err(e) => internal(lang, e),
    }
}

/// 导入 JSON（同名分组并入，新分组新建；仅导入到当前用户）
pub async fn import_json(
    State(st): State<SharedState>,
    Extension(cur): Extension<CurrentUser>,
    lang: Lang,
    Json(doc): Json<db::ExportDoc>,
) -> ApiResult {
    if doc.groups.is_empty() {
        return err(StatusCode::BAD_REQUEST, &tr(lang, "import-empty"));
    }
    let c = st.db.lock().unwrap();
    match db::import_doc(&c, cur.0, &doc) {
        Ok(r) => ok_json(serde_json::to_value(r).unwrap()),
        Err(e) => internal(lang, e),
    }
}


// ---------- 设置 ----------

pub async fn get_settings(State(st): State<SharedState>) -> ApiResult {
    let c = st.db.lock().unwrap();
    ok_json(json!({
        "port": db::get_port_setting(&c).unwrap_or(db::DEFAULT_PORT),
        "effective_port": st.effective_port,
        "webui_lan": db::get_webui_lan(&c).unwrap_or(true),
        "allow_register": db::get_allow_register(&c).unwrap_or(true),
        "db_path": db::db_path().display().to_string()
    }))
}

/// 在系统文件管理器中打开数据库文件所在位置并定位该文件，返回文件路径
pub async fn open_db_location() -> ApiResult {
    let path = db::db_path();
    reveal_in_file_manager(&path);
    ok_json(json!({ "ok": true, "path": path.display().to_string() }))
}

/// 在系统文件管理器中定位文件：Windows 资源管理器选中该文件、macOS 在
/// Finder 中显示、Linux 打开所在目录（xdg-open 无统一的"定位文件"协议）
fn reveal_in_file_manager(path: &std::path::Path) {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        // 去掉扩展长度路径前缀 \\?\（explorer 无法识别）；raw_arg 避免含空格
        // 路径被整体加引号后 /select 参数无法解析
        let shown = path.display().to_string();
        let shown = shown.strip_prefix(r"\\?\").unwrap_or(&shown);
        let _ = std::process::Command::new("explorer.exe")
            .raw_arg(format!("/select,\"{shown}\""))
            .spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let shown = path.display().to_string();
        let shown = shown.strip_prefix(r"\\?\").unwrap_or(&shown);
        let _ = std::process::Command::new("open").args(["-R", shown]).spawn();
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    let _ = std::process::Command::new("xdg-open")
        .arg(path.parent().unwrap_or(path))
        .spawn();
}

#[derive(Deserialize, Default)]
pub struct SettingsInput {
    port: Option<u16>,
    webui_lan: Option<bool>,
    allow_register: Option<bool>,
}

/// 保存服务设置：只更新传入的字段（webui_lan 与 port 重启后生效，其余立即生效）
pub async fn update_settings(
    State(st): State<SharedState>,
    lang: Lang,
    Json(body): Json<SettingsInput>,
) -> ApiResult {
    if let Some(p) = body.port {
        if !(1024..=65535).contains(&p) {
            return err(StatusCode::BAD_REQUEST, &tr(lang, "port-range"));
        }
    }
    let c = st.db.lock().unwrap();
    if let Some(p) = body.port {
        if let Err(e) = db::set_setting(&c, db::SETTINGS_PORT_KEY, &p.to_string()) {
            return internal(lang, e);
        }
    }
    if let Some(v) = body.webui_lan {
        if let Err(e) = db::set_setting(&c, db::SETTINGS_WEBUI_LAN_KEY, if v { "1" } else { "0" })
        {
            return internal(lang, e);
        }
    }
    if let Some(v) = body.allow_register {
        if let Err(e) = db::set_setting(&c, db::SETTINGS_ALLOW_REGISTER_KEY, if v { "1" } else { "0" })
        {
            return internal(lang, e);
        }
    }
    ok_json(json!({
        "port": db::get_port_setting(&c).unwrap_or(db::DEFAULT_PORT),
        "webui_lan": db::get_webui_lan(&c).unwrap_or(true),
        "allow_register": db::get_allow_register(&c).unwrap_or(true)
    }))
}
