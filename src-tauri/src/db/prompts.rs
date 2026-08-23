use rusqlite::{params, Connection, OptionalExtension, Result as SqlResult};

use super::*;

/// 默认提示词（AGENTS.md 风格的 Agent 协作规范）；用户未自定义时返回此内容。
/// 修改时注意与 MCP 工具清单、README 保持一致。
pub const DEFAULT_PROMPT: &str = r#"# Todo4Agent 协作规范（Agent 提示词）

你通过 MCP 连接到用户的任务清单 Todo4Agent。本文件是你的协作规范，用户可在界面中修改，你也可以通过 prompt_get / prompt_update 工具修改。

## 基本约定

- 数据按账号隔离，你只操作当前账号的数据
- 默认分组名为「快速清单」；分组名在同一账号内不能重复
- 任务状态只有 pending / done 两种
- 删除均为软删除：内容进入回收站，可在界面恢复；彻底删除需用户在回收站操作
- 时间一律使用 ISO 8601 格式（如 2026-08-23T12:00:00Z）

## 日常协作方式

- 用户交代待办事项时，立即用 task_create 写入清单，不要只口头确认
- 多步骤工作先整体拆解写入清单，再逐步执行，完成一步就用 task_complete 标记一步
- 任务标题以动词开头、一句话概括；背景、验收标准等细节放 description
- 有明确截止时间时填写 due_at；过期任务在界面会标红提醒
- 分组保持精简：无明确分类需求时一律使用「快速清单」，避免分组泛滥
- 会话开始或用户询问进度时，用 task_list 查看当前待办并汇报
- 用户要求删除时先确认对象再执行 task_delete（软删除可恢复，不必过度追问）

## 工具清单

- app_version / app_release：查询版本 / 发布页地址
- group_list / group_create / group_rename / group_delete：分组管理（删除分组会连同任务进回收站）
- task_list / task_create / task_update：任务查询与编辑（支持移动分组、改状态、截止时间）
- task_complete / task_delete：完成切换 / 移入回收站
- task_export / task_import：整单导出 / 导入 JSON（同名分组并入）
- user_password：修改当前账号密码（仅在用户明确要求时使用）
- prompt_get / prompt_update：读取 / 更新本提示词

## 修改本提示词

- 用户要求调整协作规范时：先 prompt_get 取当前内容，按需修改后用 prompt_update 全量写回（不做增量合并）
- 保持条目简洁可执行；修改后向用户复述变更要点
"#;

/// 读取用户自定义提示词；未设置返回 None（调用方回退 DEFAULT_PROMPT）
pub fn get_custom_prompt(conn: &Connection, user_id: i64) -> SqlResult<Option<(String, String)>> {
    conn.query_row(
        "SELECT content, updated_at FROM prompts WHERE user_id = ?1",
        params![user_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )
    .optional()
}

/// 全量保存用户提示词，返回更新时间
pub fn set_prompt(conn: &Connection, user_id: i64, content: &str) -> SqlResult<String> {
    let updated_at = now();
    conn.execute(
        "INSERT INTO prompts (user_id, content, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(user_id) DO UPDATE SET content = excluded.content, updated_at = excluded.updated_at",
        params![user_id, content, updated_at],
    )?;
    Ok(updated_at)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_conn;

    #[test]
    fn prompt_default_then_custom_roundtrip() {
        let (c, admin) = test_conn();
        // 初始：无自定义
        assert!(get_custom_prompt(&c, admin).unwrap().is_none());
        assert!(!DEFAULT_PROMPT.is_empty());
        assert!(DEFAULT_PROMPT.contains("prompt_get"));

        // 保存后可读回，时间戳非空
        let ts = set_prompt(&c, admin, "我的规范").unwrap();
        assert!(!ts.is_empty());
        let (content, updated_at) = get_custom_prompt(&c, admin).unwrap().unwrap();
        assert_eq!(content, "我的规范");
        assert_eq!(updated_at, ts);

        // 覆盖更新（时间戳为秒精度，不比较两次调用是否不同）
        set_prompt(&c, admin, "新规范").unwrap();
        let (content, _) = get_custom_prompt(&c, admin).unwrap().unwrap();
        assert_eq!(content, "新规范");
    }

    #[test]
    fn prompt_isolated_per_user() {
        let (c, admin) = test_conn();
        let other = create_user(&c, "carol", "pass1234").unwrap();
        set_prompt(&c, admin, "admin 的规范").unwrap();
        // 其他用户不受影响：读不到 admin 的，也回退默认
        assert!(get_custom_prompt(&c, other.id).unwrap().is_none());
        set_prompt(&c, other.id, "carol 的规范").unwrap();
        assert_eq!(get_custom_prompt(&c, admin).unwrap().unwrap().0, "admin 的规范");
        assert_eq!(get_custom_prompt(&c, other.id).unwrap().unwrap().0, "carol 的规范");
    }
}
