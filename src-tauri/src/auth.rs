//! 本地认证基础组件：密码哈希（PBKDF2-HMAC-SHA256 加盐）与随机 token 生成。
//! 会话（token → user）由 db::sessions 直接持久化于 SQLite，多进程实时一致。

use rand::Rng;
use sha2::{Digest, Sha256};

// ---------- 密码哈希 ----------

/// PBKDF2 迭代次数：正式版 600,000（OWASP 对 PBKDF2-HMAC-SHA256 的建议下限）。
/// 测试构建降低为 1,000，避免单测里每次建用户/校验都做数十万轮哈希
#[cfg(not(test))]
const PBKDF2_ITERATIONS: u32 = 600_000;
#[cfg(test)]
const PBKDF2_ITERATIONS: u32 = 1_000;

/// 新格式存储前缀：`pbkdf2$<迭代次数>$<盐hex>$<哈希hex>`；无前缀 = 旧版单轮 SHA-256
const PBKDF2_PREFIX: &str = "pbkdf2$";

/// HMAC-SHA256（RFC 2104）。手工实现以免引入与 sha2 0.11 的 digest 版本不兼容的 hmac 依赖
fn hmac_sha256(key: &[u8], msg: &[u8]) -> [u8; 32] {
    const BLOCK: usize = 64; // SHA-256 分组长度
    let mut key = key.to_vec();
    if key.len() > BLOCK {
        let mut h = Sha256::new();
        h.update(&key);
        key = h.finalize().to_vec();
    }
    key.resize(BLOCK, 0);
    let (mut ipad, mut opad) = ([0x36u8; BLOCK], [0x5cu8; BLOCK]);
    for i in 0..BLOCK {
        ipad[i] ^= key[i];
        opad[i] ^= key[i];
    }
    let mut inner = Sha256::new();
    inner.update(&ipad);
    inner.update(msg);
    let mut outer = Sha256::new();
    outer.update(&opad);
    outer.update(&inner.finalize());
    let mut out = [0u8; 32];
    out.copy_from_slice(&outer.finalize());
    out
}

/// PBKDF2-HMAC-SHA256（RFC 2898）；输出 32 字节（dkLen=32 仅一块，无需多块编号）
fn pbkdf2_sha256(password: &[u8], salt: &[u8], iterations: u32) -> [u8; 32] {
    let mut msg = Vec::with_capacity(salt.len() + 4);
    msg.extend_from_slice(salt);
    msg.extend_from_slice(&1u32.to_be_bytes());
    let mut u = hmac_sha256(password, &msg);
    let mut dk = u;
    for _ in 1..iterations {
        u = hmac_sha256(password, &u);
        for i in 0..32 {
            dk[i] ^= u[i];
        }
    }
    dk
}

/// 生成新密码存储串（随机盐 + PBKDF2）
pub fn new_password_hash(password: &str) -> String {
    let salt = new_salt();
    let dk = pbkdf2_sha256(password.as_bytes(), salt.as_bytes(), PBKDF2_ITERATIONS);
    format!(
        "{PBKDF2_PREFIX}{}${}${}",
        PBKDF2_ITERATIONS,
        salt,
        format_hex(&dk)
    )
}

/// 取出存储串内嵌的盐（写入 users.salt 列保持兼容；校验以内嵌盐为准）
pub fn salt_of_hash(stored: &str) -> &str {
    stored.split('$').nth(2).unwrap_or("")
}

/// 校验密码：新格式（pbkdf2$ 前缀）与旧格式（单轮加盐 SHA-256）均可
pub fn verify_password(password: &str, legacy_salt: &str, stored: &str) -> bool {
    if let Some(rest) = stored.strip_prefix(PBKDF2_PREFIX) {
        let mut parts = rest.split('$');
        let (Some(iters), Some(salt), Some(dk_hex), None) =
            (parts.next(), parts.next(), parts.next(), parts.next())
        else {
            return false;
        };
        let (Some(iters), Some(dk)) = (iters.parse::<u32>().ok(), parse_hex(dk_hex)) else {
            return false;
        };
        let computed = pbkdf2_sha256(password.as_bytes(), salt.as_bytes(), iters);
        computed.as_slice() == dk.as_slice()
    } else {
        hash_password(password, legacy_salt) == stored
    }
}

/// 是否为旧格式（单轮 SHA-256）存储；登录成功后应透明升级
pub fn is_legacy_password_hash(stored: &str) -> bool {
    !stored.starts_with(PBKDF2_PREFIX)
}

fn parse_hex(s: &str) -> Option<Vec<u8>> {
    if s.len() % 2 != 0 {
        return None;
    }
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len() / 2);
    for i in (0..b.len()).step_by(2) {
        let hi = (b[i] as char).to_digit(16)?;
        let lo = (b[i + 1] as char).to_digit(16)?;
        out.push((hi * 16 + lo) as u8);
    }
    Some(out)
}

/// 旧版格式（单轮加盐 SHA-256），仅用于校验历史存量数据，勿用于新存储
pub fn hash_password(password: &str, salt: &str) -> String {
    let mut h = Sha256::new();
    h.update(salt.as_bytes());
    h.update(password.as_bytes());
    format_hex(&h.finalize())
}

pub fn new_salt() -> String {
    random_hex(16)
}

pub fn random_hex(bytes: usize) -> String {
    let mut buf = vec![0u8; bytes];
    rand::rng().fill_bytes(&mut buf);
    format_hex(&buf)
}

fn format_hex(data: &[u8]) -> String {
    data.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_hash_and_verify() {
        // 旧版单轮 SHA-256：仅用于校验存量数据
        let salt = new_salt();
        let h1 = hash_password("secret123", &salt);
        let h2 = hash_password("secret123", &salt);
        assert_eq!(h1, h2);
        assert_ne!(h1, hash_password("secret124", &salt));
        assert_ne!(h1, hash_password("secret123", &new_salt()));
    }

    #[test]
    fn hmac_sha256_rfc4231_vector() {
        // RFC 4231 测试向量 1：key = 0x0b×20，data = "Hi There"
        let key = [0x0bu8; 20];
        let out = hmac_sha256(&key, b"Hi There");
        let hex: String = out.iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(
            hex,
            "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"
        );
    }

    #[test]
    fn pbkdf2_sha256_vectors() {
        let hex = |dk: [u8; 32]| -> String { dk.iter().map(|b| format!("{b:02x}")).collect() };
        // 常用 PBKDF2-HMAC-SHA256 测试向量（P="password"，S="salt"）
        assert_eq!(
            hex(pbkdf2_sha256(b"password", b"salt", 1)),
            "120fb6cffcf8b32c43e7225256c4f837a86548c92ccc35480805987cb70be17b"
        );
        assert_eq!(
            hex(pbkdf2_sha256(b"password", b"salt", 2)),
            "ae4d0c95af6b46d32d0adff928f06dd02a303f8ef3c251dfd6e2d85a95474c43"
        );
        assert_eq!(
            hex(pbkdf2_sha256(b"password", b"salt", 4096)),
            "c5e478d59288c841aa530db6845c4c8d962893a001ce4e11a4963873aa98134a"
        );
    }

    #[test]
    fn password_hash_roundtrip_and_verify() {
        let stored = new_password_hash("secret123");
        assert!(stored.starts_with("pbkdf2$"));
        assert!(!is_legacy_password_hash(&stored));
        let salt = salt_of_hash(&stored);
        assert!(!salt.is_empty());
        assert!(verify_password("secret123", salt, &stored));
        assert!(!verify_password("secret124", salt, &stored));

        // 旧格式仍可校验，且与同密码的新格式不同值
        let legacy_salt = new_salt();
        let legacy = hash_password("secret123", &legacy_salt);
        assert!(is_legacy_password_hash(&legacy));
        assert!(verify_password("secret123", &legacy_salt, &legacy));
        assert!(!verify_password("wrong", &legacy_salt, &legacy));

        // 损坏的存储串按校验失败处理，不 panic
        assert!(!verify_password("x", "y", "pbkdf2$bad"));
    }

    #[test]
    fn parse_hex_shape() {
        assert_eq!(parse_hex("00ff10").unwrap(), vec![0x00, 0xff, 0x10]);
        assert!(parse_hex("1").is_none());
        assert!(parse_hex("zz").is_none());
    }

    #[test]
    fn random_hex_shape() {
        let a = random_hex(16);
        let b = random_hex(16);
        assert_eq!(a.len(), 32);
        assert_ne!(a, b);
    }
}