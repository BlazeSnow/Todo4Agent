//! SQLite 数据层：分组与任务，建表并播种默认分组「快速清单」。

use rusqlite::{params, Connection, OptionalExtension, Result as SqlResult};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::auth;

/// 默认分组名（AGENTS.md 约定）
pub const DEFAULT_GROUP: &str = "快速清单";

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
pub const SETTINGS_PORT_KEY: &str = "port";
pub const DEFAULT_PORT: u16 = 3000;

pub fn get_setting(conn: &Connection, key: &str) -> SqlResult<Option<String>> {
    conn.query_row(
        "SELECT value FROM settings WHERE key = ?1",
        params![key],
        |row| row.get(0),
    )
    .optional()
}

pub fn set_setting(conn: &Connection, key: &str, value: &str) -> SqlResult<()> {
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

/// 读取端口配置；未设置或非法时回退默认值
pub fn get_port_setting(conn: &Connection) -> SqlResult<u16> {
    match get_setting(conn, SETTINGS_PORT_KEY)? {
        Some(v) => Ok(v.trim().parse().unwrap_or(DEFAULT_PORT)),
        None => Ok(DEFAULT_PORT),
    }
}

/// 数据库文件位置：环境变量 TODO4AGENT_DB 优先，否则平台数据目录
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
            created_at    TEXT NOT NULL
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
        "#,
    )?;
    ensure_task_sort_column(&conn)?;
    ensure_deleted_columns(&conn)?;
    ensure_group_user_column(&conn)?;
    seed_default_group(&conn)?;
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

// ---------- 用户 ----------

pub fn user_count(conn: &Connection) -> SqlResult<u32> {
    conn.query_row("SELECT COUNT(*) FROM users", [], |row| row.get(0))
}

pub fn list_users(conn: &Connection) -> SqlResult<Vec<User>> {
    let mut stmt = conn.prepare("SELECT id, username, created_at FROM users ORDER BY id")?;
    let rows = stmt.query_map([], |row| {
        Ok(User {
            id: row.get(0)?,
            username: row.get(1)?,
            created_at: row.get(2)?,
        })
    })?;
    rows.collect()
}

/// 创建用户；用户名重复返回 Err（UNIQUE 约束）
pub fn create_user(conn: &Connection, username: &str, password: &str) -> SqlResult<User> {
    let salt = auth::new_salt();
    let hash = auth::hash_password(password, &salt);
    let created_at = now();
    conn.execute(
        "INSERT INTO users (username, salt, password_hash, created_at) VALUES (?1, ?2, ?3, ?4)",
        params![username, salt, hash, created_at],
    )?;
    let id = conn.last_insert_rowid();
    // 首个用户接管本地模式遗留的无主数据
    if user_count(conn)? == 1 {
        conn.execute(
            "UPDATE groups SET user_id = ?1 WHERE user_id IS NULL",
            params![id],
        )?;
    }
    Ok(User {
        id,
        username: username.to_string(),
        created_at,
    })
}

/// 校验用户名密码；成功返回用户
pub fn verify_user(conn: &Connection, username: &str, password: &str) -> SqlResult<Option<User>> {
    let row = conn
        .query_row(
            "SELECT id, username, salt, password_hash, created_at FROM users WHERE username = ?1",
            params![username],
            |row| {
                Ok((
                    User {
                        id: row.get(0)?,
                        username: row.get(1)?,
                        created_at: row.get(4)?,
                    },
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            },
        )
        .optional()?;
    Ok(row.and_then(|(user, salt, hash)| {
        if auth::hash_password(password, &salt) == hash {
            Some(user)
        } else {
            None
        }
    }))
}

/// 修改用户密码
pub fn change_user_password(
    conn: &Connection,
    user_id: i64,
    old_password: &str,
    new_password: &str,
) -> SqlResult<bool> {
    let row = conn
        .query_row(
            "SELECT salt, password_hash FROM users WHERE id = ?1",
            params![user_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()?;
    let Some((salt, hash)) = row else { return Ok(false) };
    if auth::hash_password(old_password, &salt) != hash {
        return Ok(false);
    }
    let new_salt = auth::new_salt();
    let new_hash = auth::hash_password(new_password, &new_salt);
    conn.execute(
        "UPDATE users SET salt = ?1, password_hash = ?2 WHERE id = ?3",
        params![new_salt, new_hash, user_id],
    )?;
    Ok(true)
}

/// 播种默认分组（不存在才插入，可重复调用）
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

pub fn list_groups(conn: &Connection, user_id: Option<i64>) -> SqlResult<Vec<Group>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {GROUP_COLS} FROM groups WHERE deleted_at IS NULL AND user_id IS ?1 ORDER BY sort_order, id"
    ))?;
    let rows = stmt.query_map(params![user_id], group_from_row)?;
    rows.collect()
}

pub fn create_group(conn: &Connection, user_id: Option<i64>, name: &str) -> SqlResult<Group> {
    let created_at = now();
    conn.execute(
        "INSERT INTO groups (name, sort_order, created_at, user_id) VALUES (?1, 0, ?2, ?3)",
        params![name, created_at, user_id],
    )?;
    Ok(Group {
        id: conn.last_insert_rowid(),
        name: name.to_string(),
        sort_order: 0,
        created_at,
        deleted_at: None,
    })
}

/// 分组是否属于该用户
pub fn group_owned_by(conn: &Connection, user_id: Option<i64>, group_id: i64) -> SqlResult<bool> {
    conn.query_row(
        "SELECT 1 FROM groups WHERE id = ?1 AND user_id IS ?2",
        params![group_id, user_id],
        |_| Ok(()),
    )
    .optional()
    .map(|r| r.is_some())
}

/// 重命名分组；分组不存在、已删除或不属于该用户返回 Ok(None)
pub fn rename_group(
    conn: &Connection,
    user_id: Option<i64>,
    id: i64,
    name: &str,
) -> SqlResult<Option<Group>> {
    let updated = conn.execute(
        "UPDATE groups SET name = ?1 WHERE id = ?2 AND user_id IS ?3 AND deleted_at IS NULL",
        params![name, id, user_id],
    )?;
    if updated == 0 {
        return Ok(None);
    }
    let row = conn.query_row(
        &format!("SELECT {GROUP_COLS} FROM groups WHERE id = ?1"),
        params![id],
        group_from_row,
    )?;
    Ok(Some(row))
}

/// 按给定顺序重排某用户的分组（group_ids 中分组的 sort_order 依次赋 0,1,2,...）
/// 调用方需持锁独占访问，故使用 unchecked_transaction
pub fn reorder_groups(conn: &Connection, user_id: Option<i64>, group_ids: &[i64]) -> SqlResult<()> {
    let tx = conn.unchecked_transaction()?;
    {
        let mut stmt = tx.prepare(
            "UPDATE groups SET sort_order = ?1 WHERE id = ?2 AND user_id IS ?3 AND deleted_at IS NULL",
        )?;
        for (i, gid) in group_ids.iter().enumerate() {
            stmt.execute(params![i as i64, gid, user_id])?;
        }
    }
    tx.commit()
}

/// 软删除分组（连同其下任务一并进入回收站）；不存在、已删除或不属于该用户返回 Ok(false)
pub fn delete_group(conn: &Connection, user_id: Option<i64>, id: i64) -> SqlResult<bool> {
    let tx = conn.unchecked_transaction()?;
    let n = tx.execute(
        "UPDATE groups SET deleted_at = ?1 WHERE id = ?2 AND user_id IS ?3 AND deleted_at IS NULL",
        params![now(), id, user_id],
    )?;
    if n > 0 {
        tx.execute(
            "UPDATE tasks SET deleted_at = ?1 WHERE group_id = ?2 AND deleted_at IS NULL",
            params![now(), id],
        )?;
    }
    tx.commit()?;
    Ok(n > 0)
}

pub fn list_tasks(
    conn: &Connection,
    user_id: Option<i64>,
    group_id: Option<i64>,
) -> SqlResult<Vec<Task>> {
    // 默认按手动排序序号，再按 id（创建先后）稳定排序；不含已删除任务；仅本用户分组下的任务
    let sql = match group_id {
        Some(_) => format!(
            "SELECT {TASK_COLS} FROM tasks WHERE group_id = ?1 AND deleted_at IS NULL
             AND group_id IN (SELECT id FROM groups WHERE user_id IS ?2)
             ORDER BY sort_order, id"
        ),
        None => format!(
            "SELECT {TASK_COLS} FROM tasks WHERE deleted_at IS NULL
             AND group_id IN (SELECT id FROM groups WHERE user_id IS ?1)
             ORDER BY sort_order, id"
        ),
    };
    let mut stmt = conn.prepare(&sql)?;
    let rows = match group_id {
        Some(gid) => stmt.query_map(params![gid, user_id], task_from_row)?,
        None => stmt.query_map(params![user_id], task_from_row)?,
    };
    rows.collect()
}

pub fn create_task(
    conn: &Connection,
    user_id: Option<i64>,
    group_id: i64,
    title: &str,
    description: &str,
    due_at: Option<&str>,
) -> SqlResult<Task> {
    // 分组必须属于当前用户
    if !group_owned_by(conn, user_id, group_id)? {
        return Err(rusqlite::Error::QueryReturnedNoRows);
    }
    let ts = now();
    conn.execute(
        "INSERT INTO tasks (group_id, title, description, status, due_at, created_at, updated_at)
         VALUES (?1, ?2, ?3, 'pending', ?4, ?5, ?5)",
        params![group_id, title, description, due_at, ts],
    )?;
    Ok(Task {
        id: conn.last_insert_rowid(),
        group_id,
        title: title.to_string(),
        description: description.to_string(),
        status: "pending".to_string(),
        due_at: due_at.map(String::from),
        created_at: ts.clone(),
        updated_at: ts,
        sort_order: 0,
        deleted_at: None,
    })
}

/// 按给定顺序重排某分组内的任务（task_ids 中任务的 sort_order 依次赋 0,1,2,...）
/// 调用方需持锁独占访问（如 api 层 Mutex<Connection>），故使用 unchecked_transaction
pub fn reorder_tasks(
    conn: &Connection,
    user_id: Option<i64>,
    group_id: i64,
    task_ids: &[i64],
) -> SqlResult<()> {
    if !group_owned_by(conn, user_id, group_id)? {
        return Err(rusqlite::Error::QueryReturnedNoRows);
    }
    let tx = conn.unchecked_transaction()?;
    {
        let mut stmt = tx.prepare(
            "UPDATE tasks SET sort_order = ?1, updated_at = ?2 WHERE id = ?3 AND group_id = ?4",
        )?;
        for (i, tid) in task_ids.iter().enumerate() {
            stmt.execute(params![i as i64, now(), tid, group_id])?;
        }
    }
    tx.commit()
}

/// 局部更新任务；任务不存在、不属于该用户返回 Ok(None)
pub fn update_task(
    conn: &Connection,
    user_id: Option<i64>,
    id: i64,
    patch: &TaskUpdate,
) -> SqlResult<Option<Task>> {
    let mut sets: Vec<&str> = Vec::new();
    let mut vals: Vec<rusqlite::types::Value> = Vec::new();

    if let Some(v) = &patch.title {
        sets.push("title = ?");
        vals.push(v.clone().into());
    }
    if let Some(v) = &patch.description {
        sets.push("description = ?");
        vals.push(v.clone().into());
    }
    if let Some(v) = &patch.status {
        sets.push("status = ?");
        vals.push(v.clone().into());
    }
    if let Some(v) = &patch.group_id {
        sets.push("group_id = ?");
        vals.push((*v).into());
    }
    match &patch.due_at {
        Some(Some(v)) => {
            sets.push("due_at = ?");
            vals.push(v.clone().into());
        }
        Some(None) => sets.push("due_at = NULL"),
        None => {}
    }
    sets.push("updated_at = ?");
    vals.push(now().into());

    // 全部使用匿名占位符：SET 依次绑定 vals，最后绑定 id 与 user_id
    // （WHERE 中的 user_id 与 id 交叉引用，故显式保持顺序）
    let sql = format!(
        "UPDATE tasks SET {} WHERE id = ? RETURNING {TASK_COLS}",
        sets.join(", ")
    );
    // 先按 id 更新并检查归属：归属检查通过子查询
    let user_ok = conn.query_row(
        "SELECT 1 FROM tasks WHERE id = ?1 AND deleted_at IS NULL
         AND group_id IN (SELECT id FROM groups WHERE user_id IS ?2)",
        params![id, user_id],
        |_| Ok(()),
    ).optional()?.is_some();
    if !user_ok {
        return Ok(None);
    }
    vals.push(id.into());
    let row = conn
        .query_row(&sql, rusqlite::params_from_iter(vals.iter()), task_from_row)
        .optional()?;
    Ok(row)
}

/// 软删除任务（进入回收站）；不存在、已删除或不属于该用户返回 Ok(false)
pub fn delete_task(conn: &Connection, user_id: Option<i64>, id: i64) -> SqlResult<bool> {
    let n = conn.execute(
        "UPDATE tasks SET deleted_at = ?1, updated_at = ?1 WHERE id = ?2 AND deleted_at IS NULL
         AND group_id IN (SELECT id FROM groups WHERE user_id IS ?3)",
        params![now(), id, user_id],
    )?;
    Ok(n > 0)
}

// ---------- 回收站 ----------

/// 回收站内容：该用户已删除的分组与任务（按删除时间倒序）
pub fn list_trash(conn: &Connection, user_id: Option<i64>) -> SqlResult<(Vec<Group>, Vec<Task>)> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {GROUP_COLS} FROM groups WHERE deleted_at IS NOT NULL AND user_id IS ?1 ORDER BY deleted_at DESC, id DESC"
    ))?;
    let groups: Vec<Group> =
        stmt.query_map(params![user_id], group_from_row)?.collect::<SqlResult<_>>()?;

    let mut stmt = conn.prepare(&format!(
        "SELECT {TASK_COLS} FROM tasks WHERE deleted_at IS NOT NULL
         AND group_id IN (SELECT id FROM groups WHERE user_id IS ?1)
         ORDER BY deleted_at DESC, id DESC"
    ))?;
    let tasks: Vec<Task> =
        stmt.query_map(params![user_id], task_from_row)?.collect::<SqlResult<_>>()?;
    Ok((groups, tasks))
}

/// 从回收站恢复任务；不存在、未删除或不属于该用户返回 Ok(false)
pub fn restore_task(conn: &Connection, user_id: Option<i64>, id: i64) -> SqlResult<bool> {
    let n = conn.execute(
        "UPDATE tasks SET deleted_at = NULL, updated_at = ?1
         WHERE id = ?2 AND deleted_at IS NOT NULL
           AND group_id IN (SELECT id FROM groups WHERE user_id IS ?3)",
        params![now(), id, user_id],
    )?;
    Ok(n > 0)
}

/// 彻底删除任务（物理删除）；不存在或不属于该用户返回 Ok(false)
pub fn purge_task(conn: &Connection, user_id: Option<i64>, id: i64) -> SqlResult<bool> {
    let n = conn.execute(
        "DELETE FROM tasks WHERE id = ?1
         AND group_id IN (SELECT id FROM groups WHERE user_id IS ?2)",
        params![id, user_id],
    )?;
    Ok(n > 0)
}

/// 从回收站恢复分组及其下任务（仅限该用户的回收站项）
pub fn restore_group(conn: &Connection, user_id: Option<i64>, id: i64) -> SqlResult<bool> {
    let tx = conn.unchecked_transaction()?;
    let n = tx.execute(
        "UPDATE groups SET deleted_at = NULL WHERE id = ?1 AND user_id IS ?2 AND deleted_at IS NOT NULL",
        params![id, user_id],
    )?;
    if n > 0 {
        tx.execute(
            "UPDATE tasks SET deleted_at = NULL WHERE group_id = ?1",
            params![id],
        )?;
    }
    tx.commit()?;
    Ok(n > 0)
}

/// 彻底删除分组及其下任务（物理删除，不可恢复；仅限该用户的分组）
pub fn purge_group(conn: &Connection, user_id: Option<i64>, id: i64) -> SqlResult<bool> {
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "DELETE FROM tasks WHERE group_id = ?1 AND group_id IN (SELECT id FROM groups WHERE user_id IS ?2)",
        params![id, user_id],
    )?;
    let n = tx.execute(
        "DELETE FROM groups WHERE id = ?1 AND user_id IS ?2",
        params![id, user_id],
    )?;
    tx.commit()?;
    Ok(n > 0)
}

/// 清空回收站：彻底删除该用户所有已删除的分组与任务
pub fn empty_trash(conn: &Connection, user_id: Option<i64>) -> SqlResult<()> {
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "DELETE FROM tasks WHERE deleted_at IS NOT NULL
         AND group_id IN (SELECT id FROM groups WHERE user_id IS ?1)",
        params![user_id],
    )?;
    tx.execute(
        "DELETE FROM groups WHERE deleted_at IS NOT NULL AND user_id IS ?1",
        params![user_id],
    )?;
    tx.commit()
}

/// 全部数据导出为 JSON 文档（仅该用户的数据）
pub fn export_all(conn: &Connection, user_id: Option<i64>) -> SqlResult<ExportDoc> {
    let groups = list_groups(conn, user_id)?;
    let mut out = Vec::with_capacity(groups.len());
    for g in &groups {
        let tasks = list_tasks(conn, user_id, Some(g.id))?;
        out.push(ExportGroup {
            name: g.name.clone(),
            tasks: tasks
                .into_iter()
                .map(|t| ExportTask {
                    title: t.title,
                    description: t.description,
                    status: t.status,
                    due_at: t.due_at,
                })
                .collect(),
        });
    }
    Ok(ExportDoc {
        version: 1,
        exported_at: now(),
        groups: out,
    })
}

// ---------- 导入 ----------

/// 导入结果统计
#[derive(Debug, Clone, Serialize)]
pub struct ImportResult {
    pub groups_created: usize,
    pub groups_merged: usize,
    pub tasks_imported: usize,
    pub tasks_skipped: usize,
}

/// 导入导出文档（仅导入到该用户）：同名分组并入（任务追加），新分组新建；任务全部新增
pub fn import_doc(conn: &Connection, user_id: Option<i64>, doc: &ExportDoc) -> SqlResult<ImportResult> {
    let mut result = ImportResult {
        groups_created: 0,
        groups_merged: 0,
        tasks_imported: 0,
        tasks_skipped: 0,
    };

    // 先收集现有分组名（含回收站中同名的也视为占用，避免 UNIQUE 冲突）
    let groups = list_groups(conn, user_id)?;
    let mut name_map: std::collections::HashMap<String, i64> = groups
        .iter()
        .map(|g| (g.name.clone(), g.id))
        .collect();

    for g in &doc.groups {
        let name = g.name.trim();
        if name.is_empty() {
            continue;
        }
        let group_id = match name_map.get(name) {
            Some(id) => {
                result.groups_merged += 1;
                *id
            }
            None => {
                let group = create_group(conn, user_id, name)?;
                result.groups_created += 1;
                name_map.insert(name.to_string(), group.id);
                group.id
            }
        };

        for t in &g.tasks {
            let title = t.title.trim();
            if title.is_empty() {
                result.tasks_skipped += 1;
                continue;
            }
            // 已完成任务导入后保持完成状态
            let task =
                create_task(conn, user_id, group_id, title, t.description.trim(), t.due_at.as_deref())?;
            if t.status == "done" {
                let _ = update_task(
                    conn,
                    user_id,
                    task.id,
                    &TaskUpdate {
                        status: Some("done".to_string()),
                        ..Default::default()
                    },
                );
            }
            result.tasks_imported += 1;
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_conn() -> Connection {
        open(Path::new(":memory:")).expect("open memory db")
    }

    #[test]
    fn seeds_default_group() {
        let c = test_conn();
        let groups = list_groups(&c, None).unwrap();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].name, DEFAULT_GROUP);
    }

    #[test]
    fn group_crud_and_unique() {
        let c = test_conn();
        let g = create_group(&c, None, "工作").unwrap();
        assert_eq!(g.id, 2); // 默认分组占 id=1
        let dup = create_group(&c, None, "工作").unwrap_err();
        assert!(is_unique_violation(&dup));
        assert!(rename_group(&c, None, g.id, "生活").unwrap().is_some());
        assert!(list_groups(&c, None).unwrap().iter().any(|x| x.name == "生活"));
        assert!(delete_group(&c, None, g.id).unwrap());
        assert!(rename_group(&c, None, 999, "x").unwrap().is_none());
        assert!(!delete_group(&c, None, g.id).unwrap());
    }

    #[test]
    fn task_crud() {
        let c = test_conn();
        let gid = list_groups(&c, None).unwrap()[0].id;
        let t = create_task(&c, None, gid, "写文档", "详细说明", Some("2026-09-01T00:00:00Z")).unwrap();
        assert_eq!(t.status, "pending");
        assert_eq!(list_tasks(&c, None, Some(gid)).unwrap().len(), 1);

        let t2 = update_task(
            &c,
            None,
            t.id,
            &TaskUpdate {
                title: Some("改标题".into()),
                status: Some("done".into()),
                ..Default::default()
            },
        )
        .unwrap()
        .unwrap();
        assert_eq!(t2.title, "改标题");
        assert_eq!(t2.status, "done");

        let cleared = update_task(
            &c,
            None,
            t.id,
            &TaskUpdate {
                due_at: Some(None),
                ..Default::default()
            },
        )
        .unwrap()
        .unwrap();
        assert!(cleared.due_at.is_none());

        assert!(update_task(&c, None, 999, &TaskUpdate::default()).unwrap().is_none());
        assert!(delete_task(&c, None, t.id).unwrap());
        assert!(!delete_task(&c, None, t.id).unwrap());
    }

    #[test]
    fn cascade_delete_group() {
        let c = test_conn();
        let g = create_group(&c, None, "临时").unwrap();
        create_task(&c, None, g.id, "任务", "", None).unwrap();
        assert_eq!(list_tasks(&c, None, None).unwrap().len(), 1);
        delete_group(&c, None, g.id).unwrap();
        assert_eq!(list_tasks(&c, None, None).unwrap().len(), 0);
    }

    #[test]
    fn reorder_tasks_order() {
        let c = test_conn();
        let gid = list_groups(&c, None).unwrap()[0].id;
        let t1 = create_task(&c, None, gid, "A", "", None).unwrap();
        let t2 = create_task(&c, None, gid, "B", "", None).unwrap();
        let t3 = create_task(&c, None, gid, "C", "", None).unwrap();
        assert_eq!(list_tasks(&c, None, Some(gid)).unwrap().len(), 3);

        reorder_tasks(&c, None, gid, &[t3.id, t1.id, t2.id]).unwrap();
        let ids: Vec<i64> = list_tasks(&c, None, Some(gid))
            .unwrap()
            .iter()
            .map(|t| t.id)
            .collect();
        assert_eq!(ids, vec![t3.id, t1.id, t2.id]);

        // 不影响其他分组
        reorder_tasks(&c, None, gid, &[t2.id, t3.id, t1.id]).unwrap();
        let ids: Vec<i64> = list_tasks(&c, None, Some(gid))
            .unwrap()
            .iter()
            .map(|t| t.id)
            .collect();
        assert_eq!(ids, vec![t2.id, t3.id, t1.id]);
    }

    #[test]
    fn reorder_groups_order() {
        let c = test_conn();
        let g1 = create_group(&c, None, "甲").unwrap();
        let g2 = create_group(&c, None, "乙").unwrap();
        let g3 = create_group(&c, None, "丙").unwrap();

        reorder_groups(&c, None, &[g3.id, g1.id, g2.id]).unwrap();
        let ids: Vec<i64> = list_groups(&c, None).unwrap().iter().map(|g| g.id).collect();
        // 默认分组（快速清单）在最前，其后依次为 丙、甲、乙
        assert_eq!(ids, vec![1, g3.id, g1.id, g2.id]);
    }

    #[test]
    fn trash_flow() {
        let c = test_conn();
        let gid = list_groups(&c, None).unwrap()[0].id;
        let t = create_task(&c, None, gid, "待删任务", "", None).unwrap();

        // 删除任务 → 回收站可见、列表与导出不可见
        assert!(delete_task(&c, None, t.id).unwrap());
        assert!(list_tasks(&c, None, None).unwrap().is_empty());
        let (_, tasks) = list_trash(&c, None).unwrap();
        assert_eq!(tasks.len(), 1);
        assert!(tasks[0].deleted_at.is_some());
        let doc = export_all(&c, None).unwrap();
        assert!(doc.groups[0].tasks.is_empty());

        // 恢复 → 回到列表
        assert!(restore_task(&c, None, t.id).unwrap());
        assert_eq!(list_tasks(&c, None, None).unwrap().len(), 1);
        assert!(list_trash(&c, None).unwrap().1.is_empty());

        // 再删 → 彻底删除
        delete_task(&c, None, t.id).unwrap();
        assert!(purge_task(&c, None, t.id).unwrap());
        assert!(list_trash(&c, None).unwrap().1.is_empty());

        // 分组软删级联 + 恢复
        let g = create_group(&c, None, "回收组").unwrap();
        create_task(&c, None, g.id, "组内任务", "", None).unwrap();
        assert!(delete_group(&c, None, g.id).unwrap());
        let (groups, tasks) = list_trash(&c, None).unwrap();
        assert_eq!(groups.len(), 1);
        assert_eq!(tasks.len(), 1);
        assert!(restore_group(&c, None, g.id).unwrap());
        assert_eq!(list_groups(&c, None).unwrap().len(), 2);
        assert_eq!(list_tasks(&c, None, Some(g.id)).unwrap().len(), 1);

        // 分组在回收站时其任务再删除会怎样：恢复后任务仍在
        let g2 = create_group(&c, None, "再删组").unwrap();
        delete_group(&c, None, g2.id).unwrap();
        assert!(purge_group(&c, None, g2.id).unwrap());
        assert!(list_trash(&c, None).unwrap().0.iter().all(|x| x.id != g2.id));

        // 清空回收站
        delete_group(&c, None, g.id).unwrap();
        empty_trash(&c, None).unwrap();
        let (groups, tasks) = list_trash(&c, None).unwrap();
        assert!(groups.is_empty());
        assert!(tasks.is_empty());
    }

    #[test]
    fn settings_roundtrip() {
        let c = test_conn();
        assert_eq!(get_port_setting(&c).unwrap(), DEFAULT_PORT);
        set_setting(&c, SETTINGS_PORT_KEY, "8080").unwrap();
        assert_eq!(get_port_setting(&c).unwrap(), 8080);
        // 覆盖更新
        set_setting(&c, SETTINGS_PORT_KEY, "9001").unwrap();
        assert_eq!(get_port_setting(&c).unwrap(), 9001);
    }

    #[test]
    fn import_doc_merge() {
        let c = test_conn();
        // 预置数据：快速清单已存在（默认播种）
        let gid = list_groups(&c, None).unwrap()[0].id;
        create_task(&c, None, gid, "原有任务", "", None).unwrap();

        let doc = ExportDoc {
            version: 1,
            exported_at: "2026-08-22T00:00:00Z".to_string(),
            groups: vec![
                ExportGroup {
                    name: DEFAULT_GROUP.to_string(), // 同名 → 并入快速清单
                    tasks: vec![
                        ExportTask {
                            title: "导入任务1".to_string(),
                            description: "说明".to_string(),
                            status: "done".to_string(),
                            due_at: Some("2026-09-01T00:00:00Z".to_string()),
                        },
                        ExportTask {
                            title: "导入任务2".to_string(),
                            description: String::new(),
                            status: "pending".to_string(),
                            due_at: None,
                        },
                        ExportTask {
                            title: "  ".to_string(), // 空标题 → 跳过
                            description: String::new(),
                            status: "pending".to_string(),
                            due_at: None,
                        },
                    ],
                },
                ExportGroup {
                    name: "新分组".to_string(), // 新名字 → 新建
                    tasks: vec![ExportTask {
                        title: "新组任务".to_string(),
                        description: String::new(),
                        status: "pending".to_string(),
                        due_at: None,
                    }],
                },
            ],
        };

        let r = import_doc(&c, None, &doc).unwrap();
        assert_eq!(r.groups_created, 1);
        assert_eq!(r.groups_merged, 1);
        assert_eq!(r.tasks_imported, 3);
        assert_eq!(r.tasks_skipped, 1);

        // 快速清单：原有 + 2 个导入任务，且导入任务保持 done 状态
        let tasks = list_tasks(&c, None, Some(gid)).unwrap();
        assert_eq!(tasks.len(), 3);
        assert!(tasks.iter().any(|t| t.title == "导入任务1" && t.status == "done"));

        // 新分组
        let groups = list_groups(&c, None).unwrap();
        let new_g = groups.iter().find(|g| g.name == "新分组").unwrap();
        assert_eq!(list_tasks(&c, None, Some(new_g.id)).unwrap().len(), 1);
    }

#[test]
    fn multi_user_isolation() {
        let c = test_conn();
        // 本地模式遗留数据
        let gid = list_groups(&c, None).unwrap()[0].id;
        create_task(&c, None, gid, "本地任务", "", None).unwrap();

        // 创建首个用户 → 接管本地数据
        let u1 = create_user(&c, "alice", "pass1234").unwrap();
        let g1 = list_groups(&c, Some(u1.id)).unwrap();
        assert_eq!(g1.len(), 1);
        assert_eq!(list_groups(&c, None).unwrap().len(), 0);
        let t1 = list_tasks(&c, Some(u1.id), None).unwrap();
        assert_eq!(t1.len(), 1);
        assert_eq!(t1[0].title, "本地任务");

        // 第二个用户：全新数据空间
        let u2 = create_user(&c, "bob", "pass1234").unwrap();
        assert_eq!(list_groups(&c, Some(u2.id)).unwrap().len(), 0);

        // 用户1 的分组用户2 不可见/不可操作
        let g = &g1[0];
        assert!(rename_group(&c, Some(u2.id), g.id, "抢注").unwrap().is_none());
        assert_eq!(list_tasks(&c, Some(u2.id), Some(g.id)).unwrap().len(), 0);
        assert!(!delete_task(&c, Some(u2.id), t1[0].id).unwrap());

        // 密码校验
        assert!(verify_user(&c, "alice", "pass1234").unwrap().is_some());
        assert!(verify_user(&c, "alice", "wrong").unwrap().is_none());
    }

    #[test]
    fn export_shape() {
        let c = test_conn();
        let gid = list_groups(&c, None).unwrap()[0].id;
        create_task(&c, None, gid, "A", "B", None).unwrap();
        let doc = export_all(&c, None).unwrap();
        assert_eq!(doc.version, 1);
        assert_eq!(doc.groups.len(), 1);
        assert_eq!(doc.groups[0].name, DEFAULT_GROUP);
        assert_eq!(doc.groups[0].tasks.len(), 1);
        assert_eq!(doc.groups[0].tasks[0].title, "A");
    }
}