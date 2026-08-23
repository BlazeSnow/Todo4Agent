use axum::{
    extract::{Extension, Json, State},
    http::StatusCode,
};
use serde::Deserialize;
use serde_json::json;

use super::*;
use crate::db;

// ---------- 提示词（Agent 协作规范，类似 AGENTS.md） ----------

/// 获取当前用户提示词（未自定义时返回默认）；附带默认内容供界面「恢复默认」
pub async fn get_prompt(
    State(st): State<SharedState>,
    Extension(cur): Extension<CurrentUser>,
) -> ApiResult {
    let c = st.db.lock().unwrap();
    match db::get_custom_prompt(&c, cur.0) {
        Ok(Some((content, updated_at))) => ok_json(json!({
            "content": content,
            "is_default": false,
            "updated_at": updated_at,
            "default_content": db::DEFAULT_PROMPT
        })),
        Ok(None) => ok_json(json!({
            "content": db::DEFAULT_PROMPT,
            "is_default": true,
            "updated_at": null,
            "default_content": db::DEFAULT_PROMPT
        })),
        Err(e) => internal(e),
    }
}

#[derive(Deserialize)]
pub struct PromptInput {
    content: String,
}

/// 全量保存提示词（与 MCP prompt_update 走同一 db 实现）
pub async fn put_prompt(
    State(st): State<SharedState>,
    Extension(cur): Extension<CurrentUser>,
    Json(body): Json<PromptInput>,
) -> ApiResult {
    if body.content.trim().is_empty() {
        return err(StatusCode::BAD_REQUEST, "提示词不能为空");
    }
    let c = st.db.lock().unwrap();
    match db::set_prompt(&c, cur.0, &body.content) {
        Ok(updated_at) => ok_json(json!({
            "content": body.content,
            "is_default": false,
            "updated_at": updated_at
        })),
        Err(e) => internal(e),
    }
}
