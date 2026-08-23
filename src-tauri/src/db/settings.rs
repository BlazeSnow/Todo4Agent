use rusqlite::{params, Connection, OptionalExtension, Result as SqlResult};

pub const SETTINGS_PORT_KEY: &str = "port";
pub const SETTINGS_WEBUI_LAN_KEY: &str = "webui_lan";
pub const SETTINGS_ALLOW_REGISTER_KEY: &str = "allow_register";
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

/// 读取布尔设置；未设置或非法时回退默认值（默认开）
fn get_bool_setting(conn: &Connection, key: &str, default: bool) -> SqlResult<bool> {
    Ok(match get_setting(conn, key)? {
        Some(v) => v.trim().parse::<u8>().map(|n| n != 0).unwrap_or(default),
        None => default,
    })
}

/// 是否对外（0.0.0.0）开放 WebUI；关闭时仅监听 127.0.0.1
pub fn get_webui_lan(conn: &Connection) -> SqlResult<bool> {
    get_bool_setting(conn, SETTINGS_WEBUI_LAN_KEY, true)
}

/// 是否允许注册新账号
pub fn get_allow_register(conn: &Connection) -> SqlResult<bool> {
    get_bool_setting(conn, SETTINGS_ALLOW_REGISTER_KEY, true)
}

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

    #[test]
    fn settings_bool_defaults_and_roundtrip() {
        let (c, _admin) = test_conn();
        // 默认开启
        assert!(get_webui_lan(&c).unwrap());
        assert!(get_allow_register(&c).unwrap());
        // 写入关闭并读回
        set_setting(&c, SETTINGS_WEBUI_LAN_KEY, "0").unwrap();
        set_setting(&c, SETTINGS_ALLOW_REGISTER_KEY, "0").unwrap();
        assert!(!get_webui_lan(&c).unwrap());
        assert!(!get_allow_register(&c).unwrap());
        // 重新开启
        set_setting(&c, SETTINGS_WEBUI_LAN_KEY, "1").unwrap();
        assert!(get_webui_lan(&c).unwrap());
    }
}
