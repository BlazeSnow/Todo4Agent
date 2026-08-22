#![allow(dead_code)]

use rusqlite::{Connection, Result as SqlResult};

use super::*;

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
