use rusqlite::{params, Connection, OptionalExtension, Result as SqlResult};

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_conn;

    #[test]
    fn settings_roundtrip() {
        let (c, _admin) = test_conn();
        assert_eq!(get_port_setting(&c).unwrap(), DEFAULT_PORT);
        set_setting(&c, SETTINGS_PORT_KEY, "8080").unwrap();
        assert_eq!(get_port_setting(&c).unwrap(), 8080);
        // 覆盖更新
        set_setting(&c, SETTINGS_PORT_KEY, "9001").unwrap();
        assert_eq!(get_port_setting(&c).unwrap(), 9001);
    }
}
