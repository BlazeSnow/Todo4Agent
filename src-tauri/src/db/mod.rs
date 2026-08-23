use rusqlite::{params, Connection, OptionalExtension, Result as SqlResult};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};


/// 默认分组名（AGENTS.md 约定）
pub const DEFAULT_GROUP: &str = "快速清单";
/// 初始用户与默认密码（登录后应立即修改）
pub const DEFAULT_ADMIN_USERNAME: &str = "admin";
pub const DEFAULT_ADMIN_PASSWORD: &str = "admin123";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Group {
    pub id: i64,
    pub name: String,
    pub sort_order: i64,
    pub created_at: String,
    /// 回收站标记：非 null 表示已删除（软删除）
    pub deleted_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: i64,
    pub group_id: i64,
    pub title: String,
    pub description: String,
    pub status: String,
    pub due_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    /// 手动排序序号（越小越靠前），默认 0
    pub sort_order: i64,
    /// 回收站标记：非 null 表示已删除（软删除）
    pub deleted_at: Option<String>,
}

/// 导出文档结构（与前端约定一致)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportDoc {
    pub version: u32,
    pub exported_at: String,
    /// 用户提示词（Agent 协作规范）；未设置为 null，旧版导出文件无此字段
    #[serde(default)]
    pub prompt: Option<String>,
    pub groups: Vec<ExportGroup>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportGroup {
    pub name: String,
    pub tasks: Vec<ExportTask>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportTask {
    pub title: String,
    pub description: String,
    pub status: String,
    pub due_at: Option<String>,
}

/// 任务局部更新。外层 None 表示不修改该字段；
/// due_at 为双重 Option：Some(None) 表示清空截止时间。
#[derive(Debug, Default, Deserialize)]
pub struct TaskUpdate {
    pub group_id: Option<i64>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>,
    pub due_at: Option<Option<String>>,
}

/// 当前时间（UTC，ISO8601，秒精度，如 2026-08-22T05:00:00Z）
fn now() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

// ---------- 设置 ----------

/// 数据库文件位置：环境变量 TODO4AGENT_DB 优先，否则平台数据目录 Todo4Agent/todo.db
pub fn db_path() -> PathBuf {
    if let Ok(p) = std::env::var("TODO4AGENT_DB") {
        return PathBuf::from(p);
    }
    let dir = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
    dir.join("Todo4Agent").join("todo.db")
}

/// 打开数据库：建表、执行迁移，并播种初始用户 admin 与默认分组「快速清单」
pub fn open(path: &Path) -> SqlResult<Connection> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            let _ = std::fs::create_dir_all(parent);
        }
    }
    let conn = Connection::open(path)?;
    conn.execute_batch(
        r#"
        PRAGMA foreign_keys = ON;
        CREATE TABLE IF NOT EXISTS groups (
            id         INTEGER PRIMARY KEY AUTOINCREMENT,
            name       TEXT NOT NULL,
            sort_order INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL,
            deleted_at TEXT,
            user_id    INTEGER
        );
        CREATE TABLE IF NOT EXISTS users (
            id            INTEGER PRIMARY KEY AUTOINCREMENT,
            username      TEXT NOT NULL UNIQUE,
            salt          TEXT NOT NULL,
            password_hash TEXT NOT NULL,
            created_at    TEXT NOT NULL,
            default_password INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS tasks (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            group_id    INTEGER NOT NULL REFERENCES groups(id) ON DELETE CASCADE,
            title       TEXT NOT NULL,
            description TEXT NOT NULL DEFAULT '',
            status      TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'done')),
            due_at      TEXT,
            created_at  TEXT NOT NULL,
            updated_at  TEXT NOT NULL,
            sort_order  INTEGER NOT NULL DEFAULT 0,
            deleted_at  TEXT
        );
        CREATE INDEX IF NOT EXISTS idx_tasks_group ON tasks(group_id);
        CREATE TABLE IF NOT EXISTS settings (
            key   TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS sessions (
            token   TEXT PRIMARY KEY,
            user_id INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS prompts (
            user_id    INTEGER PRIMARY KEY,
            content    TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS user_settings (
            user_id INTEGER NOT NULL,
            key     TEXT NOT NULL,
            value   TEXT NOT NULL,
            PRIMARY KEY (user_id, key)
        );
        "#,
    )?;
    ensure_task_sort_column(&conn)?;
    ensure_deleted_columns(&conn)?;
    ensure_group_user_column(&conn)?;
    ensure_user_default_password_column(&conn)?;
    ensure_group_name_scoped(&conn)?;
    seed_default_admin(&conn)?;
    Ok(conn)
}

/// 为旧数据库迁移 sort_order 列（新建库已包含该列）
fn ensure_task_sort_column(conn: &Connection) -> SqlResult<()> {
    let has: bool = conn
        .prepare("SELECT COUNT(*) FROM pragma_table_info('tasks') WHERE name = 'sort_order'")?
        .query_row([], |row| row.get::<_, i64>(0))
        .map(|n| n > 0)?;
    if !has {
        conn.execute(
            "ALTER TABLE tasks ADD COLUMN sort_order INTEGER NOT NULL DEFAULT 0",
            [],
        )?;
    }
    Ok(())
}

/// 为旧数据库迁移回收站 deleted_at 列（groups 与 tasks）
fn ensure_deleted_columns(conn: &Connection) -> SqlResult<()> {
    for (table, col) in [("groups", "deleted_at"), ("tasks", "deleted_at")] {
        let has: bool = conn
            .prepare(&format!(
                "SELECT COUNT(*) FROM pragma_table_info('{table}') WHERE name = '{col}'"
            ))?
            .query_row([], |row| row.get::<_, i64>(0))
            .map(|n| n > 0)?;
        if !has {
            conn.execute(
                &format!("ALTER TABLE {table} ADD COLUMN {col} TEXT"),
                [],
            )?;
        }
    }
    Ok(())
}

/// 为旧数据库迁移 groups.user_id 列（多用户归属）
fn ensure_group_user_column(conn: &Connection) -> SqlResult<()> {
    let has: bool = conn
        .prepare("SELECT COUNT(*) FROM pragma_table_info('groups') WHERE name = 'user_id'")?
        .query_row([], |row| row.get::<_, i64>(0))
        .map(|n| n > 0)?;
    if !has {
        conn.execute("ALTER TABLE groups ADD COLUMN user_id INTEGER", [])?;
    }
    Ok(())
}
/// 为旧数据库迁移 users.default_password 列（初始密码标记）
fn ensure_user_default_password_column(conn: &Connection) -> SqlResult<()> {
    let has: bool = conn
        .prepare("SELECT COUNT(*) FROM pragma_table_info('users') WHERE name = 'default_password'")?
        .query_row([], |row| row.get::<_, i64>(0))
        .map(|n| n > 0)?;
    if !has {
        conn.execute(
            "ALTER TABLE users ADD COLUMN default_password INTEGER NOT NULL DEFAULT 0",
            [],
        )?;
    }
    Ok(())
}

/// 分组名唯一性按用户生效且不含回收站中的分组（部分唯一索引）。
/// 旧库的 name 列级 UNIQUE 是全局的（跨用户、含软删除），SQLite 无法
/// 去除列级约束，需重建 groups 表；重建期间关闭外键检查以免触发级联删除。
fn ensure_group_name_scoped(conn: &Connection) -> SqlResult<()> {
    let table_sql: Option<String> = conn
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'groups'",
            [],
            |row| row.get(0),
        )
        .optional()?;
    if table_sql.is_some_and(|sql| sql.contains("UNIQUE")) {
        conn.execute_batch("PRAGMA foreign_keys = OFF;")?;
        let rebuild = conn.execute_batch(
            r#"
            BEGIN;
            CREATE TABLE groups_migrate (
                id         INTEGER PRIMARY KEY AUTOINCREMENT,
                name       TEXT NOT NULL,
                sort_order INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL,
                deleted_at TEXT,
                user_id    INTEGER
            );
            INSERT INTO groups_migrate (id, name, sort_order, created_at, deleted_at, user_id)
                SELECT id, name, sort_order, created_at, deleted_at, user_id FROM groups;
            DROP TABLE groups;
            ALTER TABLE groups_migrate RENAME TO groups;
            COMMIT;
            "#,
        );
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        rebuild?;
    }
    conn.execute_batch(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_groups_user_name
         ON groups(user_id, name) WHERE deleted_at IS NULL;",
    )?;
    Ok(())
}

/// 初始化时播种初始用户 admin（仅当尚无任何用户）。admin 创建时接管
/// 本地模式遗留的无主数据；若接管后仍无任何分组，播种默认分组「快速清单」。
/// 已有用户的数据库不会重复播种（用户彻底删除默认分组后不会再现）。
fn seed_default_admin(conn: &Connection) -> SqlResult<()> {
    if user_count(conn)? == 0 {
        let user = create_user(conn, DEFAULT_ADMIN_USERNAME, DEFAULT_ADMIN_PASSWORD)?;
        conn.execute(
            "UPDATE users SET default_password = 1 WHERE id = ?1",
            params![user.id],
        )?;
        if list_groups(conn, user.id)?.is_empty() {
            create_group(conn, user.id, DEFAULT_GROUP)?;
        }
    }
    Ok(())
}

/// 是否存在仍在使用默认密码的用户（用于登录页提示）
pub fn has_default_password_user(conn: &Connection) -> SqlResult<bool> {
    conn.query_row(
        "SELECT COUNT(*) FROM users WHERE default_password = 1",
        [],
        |row| row.get::<_, i64>(0),
    )
    .map(|n| n > 0)
}
// ---------- 会话持久化 ----------
// 签发 / 校验 / 吊销见 db::sessions（直接落库，多进程共享数据库时实时一致）

// ---------- 用户 ----------

/// 是否为 UNIQUE 约束冲突（分组/用户重名）。按 SQLite 扩展错误码精确判定，
/// 不与其他 ConstraintViolation（外键、CHECK 等）混淆
pub fn is_unique_violation(e: &rusqlite::Error) -> bool {
    matches!(
        e,
        rusqlite::Error::SqliteFailure(ref f, _)
            if f.extended_code == rusqlite::ffi::SQLITE_CONSTRAINT_UNIQUE
    )
}

fn group_from_row(row: &rusqlite::Row<'_>) -> SqlResult<Group> {
    Ok(Group {
        id: row.get(0)?,
        name: row.get(1)?,
        sort_order: row.get(2)?,
        created_at: row.get(3)?,
        deleted_at: row.get(4)?,
    })
}

fn task_from_row(row: &rusqlite::Row<'_>) -> SqlResult<Task> {
    Ok(Task {
        id: row.get(0)?,
        group_id: row.get(1)?,
        title: row.get(2)?,
        description: row.get(3)?,
        status: row.get(4)?,
        due_at: row.get(5)?,
        created_at: row.get(6)?,
        updated_at: row.get(7)?,
        sort_order: row.get(8)?,
        deleted_at: row.get(9)?,
    })
}

/// 分组查询的列顺序必须与 group_from_row 一致
const GROUP_COLS: &str = "id, name, sort_order, created_at, deleted_at";
/// 任务查询的列顺序必须与 task_from_row 一致
const TASK_COLS: &str =
    "id, group_id, title, description, status, due_at, created_at, updated_at, sort_order, deleted_at";


pub mod export;
pub mod groups;
pub mod prompts;
pub mod sessions;
pub mod settings;
pub mod tasks;
pub mod trash;
pub mod user_settings;
pub mod users;

pub use export::*;
pub use groups::*;
pub use prompts::*;
pub use sessions::*;
pub use settings::*;
pub use tasks::*;
pub use trash::*;
pub use user_settings::*;
pub use users::*;

#[cfg(test)]
pub(crate) fn test_conn() -> (Connection, i64) {
    let c = open(Path::new(":memory:")).expect("open memory db");
    let admin = list_users(&c).unwrap()[0].id;
    (c, admin)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seeds_default_group() {
        let (c, admin) = test_conn();
        let groups = list_groups(&c, admin).unwrap();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].name, DEFAULT_GROUP);
    }

    #[test]
    fn existing_db_does_not_reseed_default_group() {
        let path = std::env::temp_dir().join(format!("todo4agent-reseed-{}.db", std::process::id()));
        let _ = std::fs::remove_file(&path);
        // 首次打开播种 admin 与默认分组；用户彻底删除默认分组
        {
            let c = open(&path).unwrap();
            let admin = list_users(&c).unwrap()[0].id;
            let gid = list_groups(&c, admin).unwrap()[0].id;
            purge_group(&c, admin, gid).unwrap();
        }
        // 重新打开：不再播种默认分组，也不产生无主分组
        let c = open(&path).unwrap();
        let admin = list_users(&c).unwrap()[0].id;
        assert!(list_groups(&c, admin).unwrap().is_empty());
        let orphans: i64 = c
            .query_row("SELECT COUNT(*) FROM groups WHERE user_id IS NULL", [], |r| r.get(0))
            .unwrap();
        assert_eq!(orphans, 0);
        drop(c);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn legacy_local_db_admin_takes_over() {
        let path = std::env::temp_dir().join(format!("todo4agent-legacy-{}.db", std::process::id()));
        let _ = std::fs::remove_file(&path);
        // 构造旧本地模式库：有无主分组（user_id NULL）、没有任何用户
        {
            let c = Connection::open(&path).unwrap();
            c.execute_batch(
                r#"
                CREATE TABLE groups (
                    id         INTEGER PRIMARY KEY AUTOINCREMENT,
                    name       TEXT NOT NULL,
                    sort_order INTEGER NOT NULL DEFAULT 0,
                    created_at TEXT NOT NULL,
                    deleted_at TEXT,
                    user_id    INTEGER
                );
                INSERT INTO groups (name, created_at) VALUES ('遗留分组', '2026-01-01T00:00:00Z');
                "#,
            )
            .unwrap();
        }
        // 打开：播种 admin 并接管无主数据，不再追加默认分组
        let c = open(&path).unwrap();
        let admin = list_users(&c).unwrap()[0].id;
        let groups = list_groups(&c, admin).unwrap();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].name, "遗留分组");
        drop(c);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn migrates_legacy_global_unique() {
        let path = std::env::temp_dir().join(format!("todo4agent-migrate-{}.db", std::process::id()));
        let _ = std::fs::remove_file(&path);
        // 构造旧版库：name 列级 UNIQUE（全局、含软删除）
        {
            let c = Connection::open(&path).unwrap();
            c.execute_batch(
                r#"
                CREATE TABLE groups (
                    id         INTEGER PRIMARY KEY AUTOINCREMENT,
                    name       TEXT NOT NULL UNIQUE,
                    sort_order INTEGER NOT NULL DEFAULT 0,
                    created_at TEXT NOT NULL,
                    deleted_at TEXT,
                    user_id    INTEGER
                );
                CREATE TABLE tasks (
                    id          INTEGER PRIMARY KEY AUTOINCREMENT,
                    group_id    INTEGER NOT NULL REFERENCES groups(id) ON DELETE CASCADE,
                    title       TEXT NOT NULL,
                    description TEXT NOT NULL DEFAULT '',
                    status      TEXT NOT NULL DEFAULT 'pending',
                    due_at      TEXT,
                    created_at  TEXT NOT NULL,
                    updated_at  TEXT NOT NULL,
                    sort_order  INTEGER NOT NULL DEFAULT 0,
                    deleted_at  TEXT
                );
                INSERT INTO groups (id, name, created_at, user_id) VALUES
                    (1, '旧组', '2026-01-01T00:00:00Z', 1),
                    (2, '删除组', '2026-01-01T00:00:00Z', 1);
                UPDATE groups SET deleted_at = '2026-01-02T00:00:00Z' WHERE id = 2;
                INSERT INTO tasks (id, group_id, title, created_at, updated_at)
                    VALUES (1, 1, '旧任务', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z');
                "#,
            )
            .unwrap();
        }

        // 重新打开触发迁移：表重建 + 部分唯一索引
        let c = open(&path).unwrap();
        let table_sql: String = c
            .query_row(
                "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'groups'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(!table_sql.contains("UNIQUE"));
        let idx_count: i64 = c
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'index' AND name = 'idx_groups_user_name'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(idx_count, 1);

        // 数据完整保留
        let task_count: i64 = c.query_row("SELECT COUNT(*) FROM tasks", [], |r| r.get(0)).unwrap();
        assert_eq!(task_count, 1);

        // 回收站中的名字可以复用；其他用户可以用同名分组
        let reused = create_group(&c, 1, "删除组").unwrap();
        assert_ne!(reused.id, 2);
        create_group(&c, 2, "旧组").unwrap();
        // 同一用户的活动分组仍然不能重名
        assert!(create_group(&c, 1, "删除组").is_err());

        drop(c);
        let _ = std::fs::remove_file(&path);
    }
}
