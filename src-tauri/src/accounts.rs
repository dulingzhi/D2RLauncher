use std::fs;
use std::path::PathBuf;
use serde_json;
use tauri::Manager;
use uuid::Uuid;

use crate::models::Account;

#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};

fn accounts_path(app: &tauri::AppHandle) -> PathBuf {
    app.path()
        .app_data_dir()
        .expect("failed to get app data dir")
        .join("accounts.json")
}

pub(crate) fn load_accounts(app: &tauri::AppHandle) -> Vec<Account> {
    let path = accounts_path(app);
    if !path.exists() {
        return vec![];
    }
    let content = fs::read_to_string(&path).unwrap_or_default();
    serde_json::from_str(&content).unwrap_or_default()
}

pub(crate) fn save_accounts(app: &tauri::AppHandle, accounts: &Vec<Account>) -> Result<(), String> {
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

/// 窗口布局预设
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WindowLayout {
    /// 水平排列（所有窗口等高，按屏幕宽度平分）
    Horizontal,
    /// 垂直排列（所有窗口等宽，按屏幕高度平分）
    Vertical,
    /// 2x2 网格（最多4个窗口）
    Grid2x2,
    /// 2x3 网格（最多6个窗口）
    Grid2x3,
    /// 自定义行数（自动计算列数）
    CustomGrid { rows: u32 },
}

impl WindowLayout {
    /// 根据布局计算每个窗口的位置和大小
    fn calc_positions(&self, count: usize) -> Vec<(i32, i32, i32, i32)> {
        let screen_w = unsafe { GetSystemMetrics(SM_CXSCREEN) } as i32;
        let screen_h = unsafe { GetSystemMetrics(SM_CYSCREEN) } as i32;
        let gap = 4_i32;

        let (cols, rows): (i32, i32) = match self {
            WindowLayout::Horizontal => (count as i32, 1),
            WindowLayout::Vertical => (1, count as i32),
            WindowLayout::Grid2x2 => {
                let r = (count as f64).sqrt().ceil() as i32;
                (r, r)
            }
            WindowLayout::Grid2x3 => (
                count.min(3) as i32,
                ((count as f64) / 3.0).ceil() as i32,
            ),
            WindowLayout::CustomGrid { rows } => {
                let cols = ((count as f64) / (*rows as f64)).ceil() as i32;
                (cols, *rows as i32)
            }
        };

        let cell_w = (screen_w - gap * (cols - 1)) / cols;
        let cell_h = (screen_h - gap * (rows - 1)) / rows;

        (0..count)
            .map(|i| {
                let col = (i as i32) % cols;
                let row = (i as i32) / cols;
                (col * (cell_w + gap), row * (cell_h + gap), cell_w, cell_h)
            })
            .collect()
    }
}

/// 批量修改多个账号的窗口布局设置
/// 根据 layout 自动为每个账号分配不同位置
#[tauri::command]
pub fn batch_update_account_windows(
    app: tauri::AppHandle,
    account_ids: Vec<String>,
    layout: WindowLayout,
) -> Result<usize, String> {
    let mut accounts = load_accounts(&app);

    // 按 account_ids 的顺序收集有效账号索引
    let indices: Vec<usize> = account_ids
        .iter()
        .filter_map(|id| accounts.iter().position(|a| &a.id == id))
        .collect();

    if indices.is_empty() {
        return Err("未找到指定的账号".to_string());
    }

    let positions = layout.calc_positions(indices.len());

    for (i, &idx) in indices.iter().enumerate() {
        if i >= positions.len() {
            break;
        }
        let (x, y, w, h) = positions[i];
        accounts[idx].window_x = Some(x);
        accounts[idx].window_y = Some(y);
        accounts[idx].window_width = Some(w);
        accounts[idx].window_height = Some(h);
    }

    save_accounts(&app, &accounts)?;
    Ok(indices.len().min(positions.len()))
}

/// 重置所选账号的窗口布局设置（清空为 null）
#[tauri::command]
pub fn reset_account_windows(
    app: tauri::AppHandle,
    account_ids: Vec<String>,
) -> Result<usize, String> {
    let mut accounts = load_accounts(&app);
    let mut updated_count = 0;

    for account in &mut accounts {
        if account_ids.contains(&account.id) {
            account.window_x = None;
            account.window_y = None;
            account.window_width = None;
            account.window_height = None;
            updated_count += 1;
        }
    }

    save_accounts(&app, &accounts)?;
    Ok(updated_count)
}
