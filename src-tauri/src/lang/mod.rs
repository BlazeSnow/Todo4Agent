//! 后端多语言。全部文案集中在 `locales/zh-CN/main.ftl` 与
//! `locales/en-US/main.ftl` 两份 Fluent 语言包中（编译期内嵌，由 main.rs
//! 的 `i18n!` 宏加载为静态 `LOCALES`）；本模块负责语言判定与按键查询。
//! - HTTP API：依据请求头 `Accept-Language`（前端每次请求携带界面语言）
//! - MCP：依据 initialize 请求的 `locale` 字段（可被环境变量
//!   `TODO4AGENT_LANG` 覆盖），未提供时默认中文
//! - 查询始终显式传入语言（`LOCALES.lookup(&lang, key)`），不使用
//!   fluent-i18n 的线程局部 set_locale，异步并发请求不会串语言

use std::borrow::Cow;
use std::collections::HashMap;

use fluent_i18n::fluent_templates::{langid, LanguageIdentifier, Loader};
use fluent_i18n::FluentValue;

/// 后端消息语言
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Lang {
    #[default]
    Zh,
    En,
}

static LANG_ZH: LanguageIdentifier = langid!("zh-CN");
static LANG_EN: LanguageIdentifier = langid!("en-US");

impl Lang {
    /// 对应的 Fluent 语言标识
    fn id(self) -> &'static LanguageIdentifier {
        match self {
            Self::Zh => &LANG_ZH,
            Self::En => &LANG_EN,
        }
    }

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

    // ---------- 带参消息便捷方法（仅少数调用点使用） ----------

    /// 清单锁定提示：锁定后 Agent 无法编辑该清单，界面编辑不受影响
    pub fn locked_err(self, name: &str) -> String {
        tr_a(self, "locked-err", &[("name", name)])
    }

    /// 导入文档包含已锁定清单（清单名列表按语言选择连接符）
    pub fn import_locked(self, names: &[String]) -> String {
        let joined = match self {
            Self::Zh => names.join("、"),
            Self::En => names.join(", "),
        };
        tr_a(self, "import-locked", &[("names", &joined)])
    }
}

/// 按语言查静态文案；缺失键回落 fallback（zh-CN）
pub fn tr(lang: Lang, key: &str) -> String {
    crate::LOCALES.lookup(&lang.id(), key)
}

/// 按语言查带参文案（字符串参数）
pub fn tr_a(lang: Lang, key: &str, args: &[(&str, &str)]) -> String {
    // 查询接口要求 'static 的参数表，此处按需拷贝（消息量小，开销可忽略）
    let args: HashMap<Cow<'static, str>, FluentValue<'static>> = args
        .iter()
        .map(|(k, v)| (Cow::Owned(k.to_string()), FluentValue::from(v.to_string())))
        .collect();
    crate::LOCALES.lookup_with_args(&lang.id(), key, &args)
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
    fn translates_keys_per_language() {
        assert_eq!(tr(Lang::Zh, "group-not-found"), "分组不存在");
        assert_eq!(tr(Lang::En, "group-not-found"), "Group not found");
        assert_eq!(tr(Lang::En, "tool-group-list"), "List all task groups");
    }

    #[test]
    fn interpolates_arguments() {
        assert_eq!(
            tr_a(Lang::Zh, "db-error", &[("err", "boom")]),
            "数据库错误: boom"
        );
        assert_eq!(
            tr_a(Lang::En, "db-error", &[("err", "boom")]),
            "Database error: boom"
        );
        assert!(Lang::Zh.locked_err("工作").contains("「工作」"));
        assert!(Lang::En.locked_err("Work").contains("\"Work\""));
        let names = vec!["甲".to_string(), "乙".to_string()];
        assert!(Lang::Zh.import_locked(&names).contains("甲、乙"));
        assert!(Lang::En.import_locked(&names).contains("甲, 乙"));
    }

    #[test]
    fn all_tool_descriptions_present_in_both_languages() {
        for key in [
            "tool-app-version", "tool-app-release", "tool-db-path", "tool-group-list",
            "tool-group-create", "tool-group-rename", "tool-group-delete", "tool-task-list",
            "tool-task-create", "tool-task-update", "tool-task-complete", "tool-task-archive",
            "tool-task-unarchive", "tool-task-delete", "tool-task-export", "tool-task-import",
            "tool-user-password", "tool-prompt-get", "tool-prompt-update",
        ] {
            assert!(!tr(Lang::Zh, key).is_empty(), "zh missing {key}");
            // 英文缺失时会回落中文，两语言相同说明英文包漏了该键
            assert_ne!(tr(Lang::Zh, key), tr(Lang::En, key), "en missing {key}");
        }
    }
}
