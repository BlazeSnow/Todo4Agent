use rusqlite::{params, Connection, Result as SqlResult};
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

/// WebUI/API 端口设置键，默认 3000
pub fn db_path() -> PathBuf {
    if let Ok(p) = std::env::var("TODO4AGENT_DB") {
        return PathBuf::from(p);
    }
    let dir = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
    dir.join("Todo4Agent").join("todo.db")
}

/// 打开数据库：建表并播种默认分组「快速清单」
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
            name       TEXT NOT NULL UNIQUE,
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
        "#,
    )?;
    ensure_task_sort_column(&conn)?;
    ensure_deleted_columns(&conn)?;
    ensure_group_user_column(&conn)?;
    ensure_user_default_password_column(&conn)?;
    seed_default_group(&conn)?;
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

/// 初始化时播种初始用户 admin（仅当尚无任何用户；同时接管无主数据）
fn seed_default_admin(conn: &Connection) -> SqlResult<()> {
    if user_count(conn)? == 0 {
        let user = create_user(conn, DEFAULT_ADMIN_USERNAME, DEFAULT_ADMIN_PASSWORD)?;
        conn.execute(
            "UPDATE users SET default_password = 1 WHERE id = ?1",
            params![user.id],
        )?;
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

pub fn load_sessions(conn: &Connection) -> SqlResult<Vec<(String, i64)>> {
    let mut stmt = conn.prepare("SELECT token, user_id FROM sessions")?;
    let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
    rows.collect()
}

pub fn save_session(conn: &Connection, token: &str, user_id: i64) -> SqlResult<()> {
    conn.execute(
        "INSERT INTO sessions (token, user_id) VALUES (?1, ?2)
         ON CONFLICT(token) DO UPDATE SET user_id = excluded.user_id",
        params![token, user_id],
    )?;
    Ok(())
}

pub fn delete_session(conn: &Connection, token: &str) -> SqlResult<()> {
    conn.execute("DELETE FROM sessions WHERE token = ?1", params![token])?;
    Ok(())
}


// ---------- 用户 ----------

fn seed_default_group(conn: &Connection) -> SqlResult<()> {
    conn.execute(
        "INSERT INTO groups (name, sort_order, created_at, user_id)
         SELECT ?1, 0, ?2, NULL
         WHERE NOT EXISTS (SELECT 1 FROM groups WHERE name = ?1)",
        params![DEFAULT_GROUP, now()],
    )?;
    Ok(())
}

/// 是否为 UNIQUE 约束冲突（分组重名）
pub fn is_unique_violation(e: &rusqlite::Error) -> bool {
    matches!(
        e,
        rusqlite::Error::SqliteFailure(ref f, _) if f.code == rusqlite::ErrorCode::ConstraintViolation
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
pub mod sessions;
pub mod settings;
pub mod tasks;
pub mod trash;
pub mod users;

pub use export::*;
pub use groups::*;
pub use settings::*;
pub use tasks::*;
pub use trash::*;
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
        let groups = list_groups(&c, Some(admin)).unwrap();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].name, DEFAULT_GROUP);
    }
}
