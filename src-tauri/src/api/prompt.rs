use axum::{
    extract::{Extension, Json, State},
    http::StatusCode,
};
use serde::Deserialize;
use serde_json::json;

use super::*;
use crate::db;

// ---------- 提示词（Agent 协作规范，类似 AGENTS.md；默认为空，用户自行填写） ----------

/// 获取当前用户提示词（未设置时 content 为空、is_default 为 true）
pub async fn get_prompt(
    State(st): State<SharedState>,
    Extension(cur): Extension<CurrentUser>,
    lang: Lang,
) -> ApiResult {
    let c = st.db.lock().unwrap();
    match db::get_custom_prompt(&c, cur.0) {
        Ok(Some((content, updated_at))) => ok_json(json!({
            "content": content,
            "is_default": false,
            "updated_at": updated_at
        })),
        Ok(None) => ok_json(json!({
            "content": "",
            "is_default": true,
            "updated_at": null
        })),
        Err(e) => internal(lang, e),
    }
}

#[derive(Deserialize)]
pub struct PromptInput {
    content: String,
}

/// 全量保存提示词（与 MCP prompt_update 走同一 db 实现）；
/// 空白内容视为清空，回到默认空提示词
pub async fn put_prompt(
    State(st): State<SharedState>,
    Extension(cur): Extension<CurrentUser>,
    lang: Lang,
    Json(body): Json<PromptInput>,
) -> ApiResult {
    let c = st.db.lock().unwrap();
    match db::set_prompt(&c, cur.0, &body.content) {
        Ok((true, None)) => ok_json(json!({
            "content": "",
            "is_default": true,
            "updated_at": null
        })),
        Ok((false, Some(updated_at))) => ok_json(json!({
            "content": body.content,
            "is_default": false,
            "updated_at": updated_at
        })),
        // set_prompt 的两个返回形状已穷尽
        _ => err_l(
            lang,
            StatusCode::INTERNAL_SERVER_ERROR,
            "保存提示词结果异常",
            "Unexpected result while saving the prompt",
        ),
    }
}
