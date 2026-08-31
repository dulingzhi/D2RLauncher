/// 账号数据加密导出 / 导入
///
/// 导出流程：加载账号 → DPAPI 解密 token → 明文载荷用密码加密（Argon2id + AES-256-GCM）
/// 导入流程：密码解密 → 明文 token 在本机重新 DPAPI 加密 → 按模式合并

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use argon2::Argon2;
use base64::Engine;
use rand::RngCore;
use serde::{Deserialize, Serialize};

use crate::models::Account;

const FORMAT: &str = "d2r-launcher-backup";
const VERSION: u32 = 1;
const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 12;
const KEY_LEN: usize = 32;

/// 备份文件外层结构（明文 JSON，ciphertext 才是加密内容）
#[derive(Serialize, Deserialize)]
struct BackupEnvelope {
    format: String,
    version: u32,
    exported_at: i64,
    kdf: KdfInfo,
    cipher: CipherInfo,
}

#[derive(Serialize, Deserialize)]
struct KdfInfo {
    algorithm: String,
    /// base64 编码的盐
    salt: String,
}

#[derive(Serialize, Deserialize)]
struct CipherInfo {
    algorithm: String,
    /// base64 编码的 nonce
    nonce: String,
    /// base64 编码的密文（含 GCM 认证标签）
    ciphertext: String,
}

fn b64(data: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(data)
}

fn unb64(s: &str) -> Result<Vec<u8>, String> {
    base64::engine::general_purpose::STANDARD
        .decode(s)
        .map_err(|_| "文件格式无效，不是合法的备份文件".to_string())
}

/// 用 Argon2id 从密码派生 256 位密钥
fn derive_key(password: &str, salt: &[u8]) -> Result<[u8; KEY_LEN], String> {
    let mut key = [0u8; KEY_LEN];
    Argon2::default()
        .hash_password_into(password.as_bytes(), salt, &mut key)
        .map_err(|e| format!("密钥派生失败: {}", e))?;
    Ok(key)
}

/// 加密明文载荷，返回完整备份文件的 JSON 字符串
pub fn encrypt_payload(plaintext: &str, password: &str) -> String {
    let mut salt = [0u8; SALT_LEN];
    let mut nonce = [0u8; NONCE_LEN];
    let mut rng = rand::thread_rng();
    rng.fill_bytes(&mut salt);
    rng.fill_bytes(&mut nonce);

    let key = derive_key(password, &salt).expect("固定输出长度的 Argon2id 不会失败");
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key));
    let ciphertext = cipher
        .encrypt(Nonce::from_slice(&nonce), plaintext.as_bytes())
        .expect("合法密钥/nonce 下 AES-256-GCM 加密不会失败");

    let envelope = BackupEnvelope {
        format: FORMAT.to_string(),
        version: VERSION,
        exported_at: chrono::Utc::now().timestamp(),
        kdf: KdfInfo {
            algorithm: "argon2id".to_string(),
            salt: b64(&salt),
        },
        cipher: CipherInfo {
            algorithm: "aes-256-gcm".to_string(),
            nonce: b64(&nonce),
            ciphertext: b64(&ciphertext),
        },
    };
    serde_json::to_string_pretty(&envelope).expect("备份文件序列化不会失败")
}

/// 解密备份文件，返回明文载荷
pub fn decrypt_payload(envelope_json: &str, password: &str) -> Result<String, String> {
    // 先做格式预检，让任何非本程序导出的 JSON 得到明确的错误信息
    let value: serde_json::Value = serde_json::from_str(envelope_json)
        .map_err(|_| "文件格式无效，不是合法的备份文件".to_string())?;
    if value.get("format").and_then(|v| v.as_str()) != Some(FORMAT) {
        return Err("文件格式无效，不是 D2R Launcher 备份文件".to_string());
    }
    let envelope: BackupEnvelope = serde_json::from_value(value)
        .map_err(|_| "文件格式无效，不是合法的备份文件".to_string())?;
    if envelope.version != VERSION {
        return Err(format!(
            "不支持的备份版本 v{}（当前支持 v{}）",
            envelope.version, VERSION
        ));
    }

    let salt = unb64(&envelope.kdf.salt)?;
    let nonce = unb64(&envelope.cipher.nonce)?;
    let ciphertext = unb64(&envelope.cipher.ciphertext)?;

    let key = derive_key(password, &salt)?;
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key));
    let plaintext = cipher
        .decrypt(Nonce::from_slice(&nonce), ciphertext.as_slice())
        .map_err(|_| "密码错误或文件已损坏".to_string())?;

    String::from_utf8(plaintext).map_err(|_| "密码错误或文件已损坏".to_string())
}

/// 导入模式
#[derive(Debug, Clone, Copy, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportMode {
    /// 覆盖更新：同 id 用导入数据替换，新 id 追加
    Overwrite,
    /// 只导入新账号：同 id 跳过，保留本地版本
    SkipExisting,
    /// 全部替换：清空现有账号后导入
    ReplaceAll,
}

/// 导入结果统计
#[derive(Debug, Serialize)]
pub struct ImportStats {
    /// 新增账号数
    pub imported: usize,
    /// 覆盖更新账号数
    pub updated: usize,
    /// 跳过账号数
    pub skipped: usize,
}

/// 按模式将导入账号合并进现有列表（纯函数，便于测试）
fn merge_accounts(
    existing: Vec<Account>,
    incoming: Vec<Account>,
    mode: ImportMode,
) -> (Vec<Account>, ImportStats) {
    let mut stats = ImportStats { imported: 0, updated: 0, skipped: 0 };

    let mut merged = if matches!(mode, ImportMode::ReplaceAll) {
        stats.imported = incoming.len();
        return (incoming, stats);
    } else {
        existing
    };

    for inc in incoming {
        match merged.iter_mut().find(|a| a.id == inc.id) {
            Some(slot) => match mode {
                ImportMode::Overwrite => {
                    *slot = inc;
                    stats.updated += 1;
                }
                ImportMode::SkipExisting => stats.skipped += 1,
                ImportMode::ReplaceAll => unreachable!(),
            },
            None => {
                merged.push(inc);
                stats.imported += 1;
            }
        }
    }

    (merged, stats)
}

/// 备份载荷内的账号（token 为明文，整个载荷已被密码加密）
#[derive(Serialize, Deserialize)]
struct BackupAccount {
    id: String,
    label: String,
    plain_token: Option<String>,
    token_set_at: Option<i64>,
    /// 旧备份缺省为 osic / retail
    #[serde(default = "default_backup_game")]
    game: String,
    #[serde(default = "default_backup_flavor")]
    flavor: String,
    custom_args: String,
    window_x: Option<i32>,
    window_y: Option<i32>,
    window_width: Option<i32>,
    window_height: Option<i32>,
}

/// 解密后的载荷内容
#[derive(Serialize, Deserialize)]
struct BackupPayload {
    accounts: Vec<BackupAccount>,
}

fn default_backup_game() -> String {
    "osic".to_string()
}

fn default_backup_flavor() -> String {
    "retail".to_string()
}

/// 导出全部账号（含认证 token）到密码加密的备份文件
#[tauri::command]
pub fn export_accounts(
    app: tauri::AppHandle,
    path: String,
    password: String,
) -> Result<usize, String> {
    let accounts = crate::accounts::load_accounts(&app);
    if accounts.is_empty() {
        return Err("没有账号可导出".to_string());
    }

    let mut backup_accounts = Vec::with_capacity(accounts.len());
    for acc in &accounts {
        // DPAPI 与当前 Windows 用户/机器绑定，必须先解密成明文才能跨机器迁移
        let plain_token = match &acc.encrypted_token {
            Some(encoded) => {
                let bytes = base64::engine::general_purpose::STANDARD
                    .decode(encoded)
                    .map_err(|e| format!("账号 \"{}\" 的 token 编码无效: {}", acc.label, e))?;
                Some(crate::token::dpapi_decrypt_token(&bytes).map_err(|e| {
                    format!("账号 \"{}\" 的 token 解密失败: {}", acc.label, e)
                })?)
            }
            None => None,
        };
        backup_accounts.push(BackupAccount {
            id: acc.id.clone(),
            label: acc.label.clone(),
            plain_token,
            token_set_at: acc.token_set_at,
            game: acc.game.clone(),
            flavor: acc.flavor.clone(),
            custom_args: acc.custom_args.clone(),
            window_x: acc.window_x,
            window_y: acc.window_y,
            window_width: acc.window_width,
            window_height: acc.window_height,
        });
    }

    let payload = BackupPayload {
        accounts: backup_accounts,
    };
    let json = serde_json::to_string(&payload).map_err(|e| e.to_string())?;
    let envelope = encrypt_payload(&json, &password);
    std::fs::write(&path, envelope).map_err(|e| format!("写入备份文件失败: {}", e))?;
    Ok(accounts.len())
}

/// 从密码加密的备份文件导入账号，token 在本机重新 DPAPI 加密
#[tauri::command]
pub fn import_accounts(
    app: tauri::AppHandle,
    path: String,
    password: String,
    mode: ImportMode,
) -> Result<ImportStats, String> {
    let content =
        std::fs::read_to_string(&path).map_err(|e| format!("读取备份文件失败: {}", e))?;
    let payload_json = decrypt_payload(&content, &password)?;
    let payload: BackupPayload = serde_json::from_str(&payload_json)
        .map_err(|_| "备份内容无效（文件可能已损坏）".to_string())?;

    let mut incoming = Vec::with_capacity(payload.accounts.len());
    for b in payload.accounts {
        let encrypted_token = match &b.plain_token {
            Some(plain) => {
                let encrypted = crate::token::dpapi_encrypt_token(plain).map_err(|e| {
                    format!("账号 \"{}\" 的 token 加密失败: {}", b.label, e)
                })?;
                Some(b64(&encrypted))
            }
            None => None,
        };
        incoming.push(Account {
            id: b.id,
            label: b.label,
            encrypted_token,
            token_set_at: b.token_set_at,
            game: b.game,
            flavor: b.flavor,
            custom_args: b.custom_args,
            window_x: b.window_x,
            window_y: b.window_y,
            window_width: b.window_width,
            window_height: b.window_height,
        });
    }

    let existing = crate::accounts::load_accounts(&app);
    let (merged, stats) = merge_accounts(existing, incoming, mode);
    crate::accounts::save_accounts(&app, &merged)?;
    Ok(stats)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encrypt_decrypt_round_trip() {
        let plaintext = r#"{"accounts":[{"label":"acc1"}]}"#;
        let envelope = encrypt_payload(plaintext, "correct horse battery");
        let decrypted = decrypt_payload(&envelope, "correct horse battery").unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn wrong_password_returns_friendly_error() {
        let envelope = encrypt_payload("secret payload", "right password");
        let err = decrypt_payload(&envelope, "wrong password").unwrap_err();
        assert_eq!(err, "密码错误或文件已损坏");
    }

    #[test]
    fn non_backup_file_returns_format_error() {
        let err = decrypt_payload("not a json at all", "any").unwrap_err();
        assert_eq!(err, "文件格式无效，不是合法的备份文件");

        let foreign = r#"{"format":"something-else","version":1,"exported_at":0,"kdf":{},"cipher":{}}"#;
        let err = decrypt_payload(foreign, "any").unwrap_err();
        assert_eq!(err, "文件格式无效，不是 D2R Launcher 备份文件");
    }

    fn acc(id: &str, label: &str) -> Account {
        Account {
            id: id.to_string(),
            label: label.to_string(),
            encrypted_token: None,
            token_set_at: None,
            game: "osic".to_string(),
            flavor: "retail".to_string(),
            custom_args: String::new(),
            window_x: None,
            window_y: None,
            window_width: None,
            window_height: None,
        }
    }

    #[test]
    fn overwrite_updates_existing_and_adds_new() {
        let existing = vec![acc("A", "旧标签")];
        let incoming = vec![acc("A", "新标签"), acc("B", "新账号")];

        let (merged, stats) = merge_accounts(existing, incoming, ImportMode::Overwrite);

        assert_eq!(merged.len(), 2);
        assert_eq!(merged.iter().find(|a| a.id == "A").unwrap().label, "新标签");
        assert_eq!(stats.imported, 1);
        assert_eq!(stats.updated, 1);
        assert_eq!(stats.skipped, 0);
    }

    #[test]
    fn skip_existing_keeps_local_version() {
        let existing = vec![acc("A", "本地标签")];
        let incoming = vec![acc("A", "导入标签"), acc("B", "新账号")];

        let (merged, stats) = merge_accounts(existing, incoming, ImportMode::SkipExisting);

        assert_eq!(merged.len(), 2);
        assert_eq!(merged.iter().find(|a| a.id == "A").unwrap().label, "本地标签");
        assert_eq!(stats.imported, 1);
        assert_eq!(stats.updated, 0);
        assert_eq!(stats.skipped, 1);
    }

    #[test]
    fn replace_all_discards_existing() {
        let existing = vec![acc("A", "本地"), acc("C", "将被丢弃")];
        let incoming = vec![acc("A", "导入"), acc("B", "新账号")];

        let (merged, stats) = merge_accounts(existing, incoming, ImportMode::ReplaceAll);

        assert_eq!(merged.len(), 2);
        assert_eq!(merged.iter().find(|a| a.id == "A").unwrap().label, "导入");
        assert!(merged.iter().all(|a| a.id != "C"));
        assert_eq!(stats.imported, 2);
        assert_eq!(stats.updated, 0);
        assert_eq!(stats.skipped, 0);
    }
}

