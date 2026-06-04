use std::process::Command;
use std::thread;
use std::time::Duration;
use tauri::Emitter;

use crate::token::{dpapi_decrypt_token, write_token_to_registry};
use crate::accounts;

/// 启动单个账号的 D2R 实例
/// 流程：解密 token → 写注册表 → 启动 D2R.exe → 等待后杀互斥锁句柄
#[tauri::command]
pub fn launch_account(
    app: tauri::AppHandle,
    account_id: String,
    game_path: String,
    handle_path: String,
) -> Result<(), String> {
    // 1. 获取账号信息
    let accounts = accounts::get_accounts(app.clone());
    let account = accounts
        .iter()
        .find(|a| a.id == account_id)
        .ok_or_else(|| format!("Account {} not found", account_id))?
        .clone();

    // 2. 解密并写入注册表
    if let Some(encoded) = &account.encrypted_token {
        let encrypted = base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            encoded,
        )
        .map_err(|e| format!("base64 decode failed: {}", e))?;

        let plain_token = dpapi_decrypt_token(&encrypted)?;
        write_token_to_registry(plain_token)?;
    } else {
        return Err("此账号尚未设置 Token，请先获取 Token".to_string());
    }

    // 3. 拼接启动参数
    let d2r_exe = format!("{}\\D2R.exe", game_path.trim_end_matches('\\'));
    let mut args: Vec<String> = vec!["-uid".to_string(), "osic".to_string()];

    // 窗口位置参数（可选）
    if let (Some(_x), Some(_y)) = (account.window_x, account.window_y) {
        // D2R 不支持直接位置参数，使用自定义参数传递
    }

    // 自定义参数
    if !account.custom_args.is_empty() {
        for arg in account.custom_args.split_whitespace() {
            args.push(arg.to_string());
        }
    }

    // 4. 启动 D2R.exe
    let child = Command::new(&d2r_exe)
        .args(&args)
        .spawn()
        .map_err(|e| format!("Failed to launch D2R.exe: {}", e))?;

    let pid = child.id();

    // 5. 后台线程等待并杀互斥锁句柄
    if !handle_path.is_empty() {
        let handle_exe = handle_path.clone();
        thread::spawn(move || {
            // 等待游戏窗口出现
            thread::sleep(Duration::from_secs(5));
            kill_d2r_mutex_handle(&handle_exe, pid);
        });
    }

    // 6. 记录窗口位置（如果配置了的话，启动后移动窗口）
    if let (Some(x), Some(y)) = (account.window_x, account.window_y) {
        let _ = app.emit("move_window", serde_json::json!({
            "pid": pid,
            "x": x,
            "y": y
        }));
    }

    Ok(())
}

/// 批量启动所有账号，每个账号间隔 delay_secs 秒
#[tauri::command]
pub fn launch_all_accounts(
    app: tauri::AppHandle,
    account_ids: Vec<String>,
    game_path: String,
    handle_path: String,
    delay_secs: u64,
) -> Result<(), String> {
    for (i, id) in account_ids.iter().enumerate() {
        if i > 0 {
            thread::sleep(Duration::from_secs(delay_secs));
        }
        launch_account(
            app.clone(),
            id.clone(),
            game_path.clone(),
            handle_path.clone(),
        )?;
    }
    Ok(())
}

/// 使用 Handle64.exe 杀掉 D2R 进程的互斥锁句柄，允许多开
fn kill_d2r_mutex_handle(handle_exe: &str, pid: u32) {
    // Handle64.exe -p <pid> -a <handle_name> -c
    // D2R 的互斥锁名称
    let mutex_name = "DiabloII Check For Other Instances";

    let output = Command::new(handle_exe)
        .args([
            "-accepteula",
            "-p",
            &pid.to_string(),
            mutex_name,
            "-c",   // close handle
            "-y",   // 跳过确认
        ])
        .output();

    if let Err(e) = output {
        eprintln!("Handle64 error: {}", e);
    }
}
