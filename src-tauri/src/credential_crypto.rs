// ── 凭据加密（F3）：AES-256-GCM + 本地随机密钥 ──
// 密钥存 ~/.jc9/key.bin（0600）；敏感字段加密为 `enc$<iv_b64>.<ct_b64>`，明文不落盘。
// 设计见 docs/plans §6：v1 本地加密；v2 可升级系统钥匙串 / DPAPI。

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use rand::RngCore;
use std::fs;
use std::path::PathBuf;

const ENC_PREFIX: &str = "enc$";
const KEY_FILE: &str = "key.bin";

fn key_path() -> PathBuf {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join(".jc9").join(KEY_FILE)
}

/// 读取或首次生成 AES-256 密钥（0600 权限）
pub fn load_or_create_key() -> Result<[u8; 32], String> {
    let path = key_path();
    if let Ok(content) = fs::read(&path) {
        if content.len() == 32 {
            let mut k = [0u8; 32];
            k.copy_from_slice(&content);
            return Ok(k);
        }
    }
    let mut key = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut key);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("创建密钥目录失败: {e}"))?;
    }
    fs::write(&path, key).map_err(|e| format!("写入密钥失败: {e}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o600));
    }
    Ok(key)
}

pub fn is_encrypted(v: &str) -> bool {
    v.starts_with(ENC_PREFIX)
}

/// 加密敏感字段 → `enc$<iv_b64>.<ct_b64>`
pub fn encrypt_field(key: &[u8; 32], plain: &str) -> Result<String, String> {
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    let mut iv = [0u8; 12];
    rand::rngs::OsRng.fill_bytes(&mut iv);
    let ct = cipher
        .encrypt(Nonce::from_slice(&iv), plain.as_bytes())
        .map_err(|e| format!("加密失败: {e}"))?;
    Ok(format!(
        "{}{}.{}",
        ENC_PREFIX,
        B64.encode(iv),
        B64.encode(ct)
    ))
}

/// 解密字段（非 enc$ 前缀则原样返回，兼容旧明文数据）
pub fn decrypt_field(key: &[u8; 32], val: &str) -> Result<String, String> {
    if !is_encrypted(val) {
        return Ok(val.to_string());
    }
    let body = val.trim_start_matches(ENC_PREFIX);
    let (iv_b64, ct_b64) = body
        .split_once('.')
        .ok_or_else(|| "密文格式错误".to_string())?;
    let iv = B64.decode(iv_b64).map_err(|e| format!("解析 IV 失败: {e}"))?;
    let ct = B64.decode(ct_b64).map_err(|e| format!("解析密文失败: {e}"))?;
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    let plain = cipher
        .decrypt(Nonce::from_slice(&iv), ct.as_ref())
        .map_err(|e| format!("解密失败: {e}"))?;
    String::from_utf8(plain).map_err(|e| format!("解密结果非法: {e}"))
}

/// 需要加密的敏感字段（其余如 username/url 明文）
pub fn sensitive_fields() -> &'static [&'static str] {
    &["password", "token", "kubeconfig"]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip() {
        let key = [7u8; 32];
        let enc = encrypt_field(&key, "secret-token-123").unwrap();
        assert!(is_encrypted(&enc));
        assert_eq!(decrypt_field(&key, &enc).unwrap(), "secret-token-123");
        // 每次加密随机 IV → 两次密文不同
        let enc2 = encrypt_field(&key, "secret-token-123").unwrap();
        assert_ne!(enc, enc2);
        // 明文兼容（旧数据）
        assert_eq!(decrypt_field(&key, "plain").unwrap(), "plain");
        // 错误密钥解不开
        let other = [9u8; 32];
        assert!(decrypt_field(&other, &enc).is_err());
    }
}
