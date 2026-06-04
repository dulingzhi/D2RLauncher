use std::fs;
use std::path::PathBuf;
use serde_json;
use tauri::Manager;
use uuid::Uuid;

use crate::models::Account;

fn accounts_path(app: &tauri::AppHandle) -> PathBuf {
    app.path()
        .app_data_dir()
        .expect("failed to get app data dir")
        .join("accounts.json")
}

fn load_accounts(app: &tauri::AppHandle) -> Vec<Account> {
    let path = accounts_path(app);
    if !path.exists() {
        return vec![];
    }
    let content = fs::read_to_string(&path).unwrap_or_default();
    serde_json::from_str(&content).unwrap_or_default()
}

fn save_accounts(app: &tauri::AppHandle, accounts: &Vec<Account>) -> Result<(), String> {
    let path = accounts_path(app);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(accounts).map_err(|e| e.to_string())?;
    fs::write(&path, json).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn get_accounts(app: tauri::AppHandle) -> Vec<Account> {
    load_accounts(&app)
}

#[tauri::command]
pub fn add_account(
    app: tauri::AppHandle,
    label: String,
    email: String,
    custom_args: String,
    window_x: Option<i32>,
    window_y: Option<i32>,
    window_width: Option<i32>,
    window_height: Option<i32>,
) -> Result<Account, String> {
    let mut accounts = load_accounts(&app);
    let account = Account {
        id: Uuid::new_v4().to_string(),
        label,
        email,
        encrypted_token: None,
        token_set_at: None,
        custom_args,
        window_x,
        window_y,
        window_width,
        window_height,
    };
    accounts.push(account.clone());
    save_accounts(&app, &accounts)?;
    Ok(account)
}

#[tauri::command]
pub fn update_account(app: tauri::AppHandle, account: Account) -> Result<(), String> {
    let mut accounts = load_accounts(&app);
    if let Some(pos) = accounts.iter().position(|a| a.id == account.id) {
        accounts[pos] = account;
        save_accounts(&app, &accounts)?;
        Ok(())
    } else {
        Err(format!("Account {} not found", account.id))
    }
}

#[tauri::command]
pub fn delete_account(app: tauri::AppHandle, id: String) -> Result<(), String> {
    let mut accounts = load_accounts(&app);
    accounts.retain(|a| a.id != id);
    save_accounts(&app, &accounts)?;
    Ok(())
}

/// 将捕获到的明文 token 加密后保存到账号
#[tauri::command]
pub fn save_token_for_account(
    app: tauri::AppHandle,
    id: String,
    plain_token: String,
) -> Result<(), String> {
    let encrypted = crate::token::dpapi_encrypt_token(&plain_token)?;
    let encoded = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        &encrypted,
    );
    let now = chrono::Utc::now().timestamp();

    let mut accounts = load_accounts(&app);
    if let Some(acc) = accounts.iter_mut().find(|a| a.id == id) {
        acc.encrypted_token = Some(encoded);
        acc.token_set_at = Some(now);
        save_accounts(&app, &accounts)?;
        Ok(())
    } else {
        Err(format!("Account {} not found", id))
    }
}
