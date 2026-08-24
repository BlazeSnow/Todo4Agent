//! 后端多语言：HTTP API 错误信息与 MCP 消息按语言返回。
//! - HTTP API：依据请求头 `Accept-Language`（前端每次请求携带界面语言）
//! - MCP：依据 initialize 请求的 `locale` 字段（可被环境变量
//!   `TODO4AGENT_LANG` 覆盖），未提供时默认中文
//! 取值辅助：`t(lang, "中文", "English")`，消息就近内联对照，不设独立键表。

/// 后端消息语言
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Lang {
    #[default]
    Zh,
    En,
}

/// 中英对照取值
pub fn t(lang: Lang, zh: &'static str, en: &'static str) -> &'static str {
    match lang {
        Lang::Zh => zh,
        Lang::En => en,
    }
}

impl Lang {
    /// 解析 Accept-Language 头：按逗号拆分取各条首选语言，
    /// 首个 `zh*` 即中文，出现任何非 zh 语言即英文；通配符与缺省回落中文
    pub fn from_accept_language(header: Option<&str>) -> Lang {
        let Some(header) = header else { return Lang::Zh };
        for part in header.split(',') {
            let tag = part.split(';').next().unwrap_or("").trim();
            if tag.is_empty() || tag == "*" {
                continue;
            }
            if tag.to_ascii_lowercase().starts_with("zh") {
                return Lang::Zh;
            }
            return Lang::En;
        }
        Lang::Zh
    }

    /// 解析单个语言标签（MCP initialize 的 locale / TODO4AGENT_LANG 环境变量）：
    /// `zh*` 为中文，其他非空值视为英文，缺省 None 由调用方决定默认
    pub fn parse_tag(tag: Option<&str>) -> Option<Lang> {
        let tag = tag?.trim();
        if tag.is_empty() {
            return None;
        }
        Some(if tag.to_ascii_lowercase().starts_with("zh") { Lang::Zh } else { Lang::En })
    }

    /// MCP 会话语言：TODO4AGENT_LANG 环境变量（zh / en）优先，缺省 None
    pub fn from_env() -> Option<Lang> {
        Lang::parse_tag(std::env::var("TODO4AGENT_LANG").ok().as_deref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_accept_language() {
        assert_eq!(Lang::from_accept_language(None), Lang::Zh);
        assert_eq!(Lang::from_accept_language(Some("")), Lang::Zh);
        assert_eq!(Lang::from_accept_language(Some("zh-CN,zh;q=0.9,en;q=0.8")), Lang::Zh);
        assert_eq!(Lang::from_accept_language(Some("en-US,en;q=0.9")), Lang::En);
        assert_eq!(Lang::from_accept_language(Some(" ja JP ")), Lang::En);
        // 通配符（无语言偏好）与缺省一致，回落默认中文
        assert_eq!(Lang::from_accept_language(Some("*")), Lang::Zh);
    }

    #[test]
    fn parses_single_tag() {
        assert_eq!(Lang::parse_tag(None), None);
        assert_eq!(Lang::parse_tag(Some("")), None);
        assert_eq!(Lang::parse_tag(Some("  ")), None);
        assert_eq!(Lang::parse_tag(Some("zh-CN")), Some(Lang::Zh));
        assert_eq!(Lang::parse_tag(Some("ZH")), Some(Lang::Zh));
        assert_eq!(Lang::parse_tag(Some("en-US")), Some(Lang::En));
        assert_eq!(Lang::parse_tag(Some("fr")), Some(Lang::En));
    }

    #[test]
    fn picks_message() {
        assert_eq!(t(Lang::Zh, "分组不存在", "Group not found"), "分组不存在");
        assert_eq!(t(Lang::En, "分组不存在", "Group not found"), "Group not found");
    }
}
