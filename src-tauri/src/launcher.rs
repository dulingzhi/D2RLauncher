use std::process::Command;
use std::thread;
use std::time::Duration;
use tauri::Emitter;

use crate::token::{dpapi_decrypt_token, write_token_to_registry};
use crate::accounts;
use crate::settings;
use crate::token_monitor;
use crate::embedded_resources;

#[cfg(windows)]
use windows::Win32::Foundation::HWND;
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::{EnumWindows, GetWindowThreadProcessId, SetWindowTextW};
#[cfg(windows)]
use windows::core::PCWSTR;

/// 启动单个账号的 D2R 实例
/// 流程：解密 token → 写注册表 → 启动 D2R.exe → 等待后杀互斥锁句柄
#[tauri::command]
pub fn launch_account(
    app: tauri::AppHandle,
    account_id: String,
    game_path: String,
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
    
    println!("🎮 游戏进程已启动，PID: {}", pid);

    // 获取设置
    let settings = settings::get_settings(app.clone());
    let wait_for_login = settings.wait_for_login;
    let login_timeout = settings.login_timeout_secs;

    // 5. 获取内置的 handle64.exe 路径
    let handle_path = embedded_resources::get_handle64_path()
        .unwrap_or_else(|e| {
            eprintln!("⚠️ 获取内置 handle64.exe 失败: {}, 将跳过互斥锁处理", e);
            std::path::PathBuf::new()
        });
    
    // 6. 后台线程：重命名窗口、等待并杀互斥锁句柄、监控登录状态
    let account_id_clone = account.id.clone();
    let account_name_clone = account.label.clone();
    let region_clone = "cn".to_string();
    let app_clone = app.clone();
    
    thread::spawn(move || {
        // 等待游戏窗口出现（延长到 5 秒）
        println!("⏰ 等待游戏窗口出现...");
        thread::sleep(Duration::from_secs(5));
        
        // 重命名游戏窗口（不依赖 handle.exe）
        #[cfg(windows)]
        {
            let new_title = format!(
                "[{}] {} ({}) - Diablo II: Resurrected",
                account_id_clone, account_name_clone, region_clone
            );
            println!("🏷️  尝试重命名窗口为: {}", new_title);
            
            match rename_d2r_window(pid, &new_title) {
                Ok(_) => {
                    println!("✅ 窗口重命名成功");
                    // 立即触发一次游戏监控检查
                    thread::sleep(Duration::from_millis(500));
                    let _ = app_clone.emit("force_game_check", ());
                }
                Err(e) => {
                    eprintln!("❌ 重命名窗口失败: {}", e);
                }
            }
        }
        
        // 杀互斥锁句柄（使用内置的 handle64.exe）
        if handle_path.exists() {
            println!("🔓 尝试关闭互斥锁句柄...");
            let handle_path_str = handle_path.to_string_lossy().to_string();
            kill_d2r_mutex_handle(&handle_path_str, pid);
            println!("✅ 互斥锁处理完成");
        } else {
            println!("⚠️  内置 handle64.exe 不可用，跳过互斥锁处理");
        }
        
        // 监控登录状态（如果启用）
        if wait_for_login {
            println!("🔐 已启用登录完成检测");
            match token_monitor::wait_for_login_complete(login_timeout) {
                Ok(true) => {
                    println!("✅ {} 登录完成！", account_name_clone);
                    let _ = app_clone.emit("login_complete", serde_json::json!({
                        "account_id": account_id_clone,
                        "account_name": account_name_clone,
                        "success": true
                    }));
                }
                Ok(false) => {
                    println!("⏰ {} 登录检测超时", account_name_clone);
                    let _ = app_clone.emit("login_complete", serde_json::json!({
                        "account_id": account_id_clone,
                        "account_name": account_name_clone,
                        "success": false,
                        "timeout": true
                    }));
                }
                Err(e) => {
                    eprintln!("❌ 登录检测失败: {}", e);
                    let _ = app_clone.emit("login_complete", serde_json::json!({
                        "account_id": account_id_clone,
                        "account_name": account_name_clone,
                        "success": false,
                        "error": e
                    }));
                }
            }
        } else {
            println!("⚠️  已禁用登录完成检测");
        }
    });

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

/// Windows 下重命名 D2R 游戏窗口
#[cfg(windows)]
fn rename_d2r_window(target_pid: u32, new_title: &str) -> Result<(), String> {
    use std::sync::{Arc, Mutex};
    use windows::Win32::UI::WindowsAndMessaging::GetWindowTextW;
    
    let target_hwnd: Arc<Mutex<Option<HWND>>> = Arc::new(Mutex::new(None));
    let target_hwnd_clone = Arc::clone(&target_hwnd);

    println!("🔍 查找进程 PID {} 的窗口...", target_pid);

    // 查找目标进程的窗口句柄
    unsafe {
        let _ = EnumWindows(
            Some(find_window_callback),
            windows::Win32::Foundation::LPARAM(
                Box::into_raw(Box::new((target_pid, target_hwnd_clone))) as isize
            ),
        );
    }

    // 如果找到窗口，设置新标题
    let found_hwnd = {
        let guard = target_hwnd.lock().unwrap();
        *guard
    };
    
    if let Some(hwnd) = found_hwnd {
        // 先读取当前窗口标题
        let mut current_title: [u16; 512] = [0; 512];
        unsafe {
            let len = GetWindowTextW(hwnd, &mut current_title);
            if len > 0 {
                let old_title = String::from_utf16_lossy(&current_title[..len as usize]);
                println!("📝 当前窗口标题: {}", old_title);
            }
        }
        
        let mut wide_title: Vec<u16> = new_title.encode_utf16().chain(std::iter::once(0)).collect();
        
        unsafe {
            SetWindowTextW(hwnd, PCWSTR(wide_title.as_mut_ptr()))
                .map_err(|e| format!("SetWindowTextW 失败: {:?}", e))?;
        }
        
        println!("✅ 游戏窗口已重命名为: {}", new_title);
        Ok(())
    } else {
        Err(format!("未找到进程 {} 的游戏窗口", target_pid))
    }
}

/// Windows API 回调：查找指定进程的窗口
#[cfg(windows)]
unsafe extern "system" fn find_window_callback(
    hwnd: HWND,
    lparam: windows::Win32::Foundation::LPARAM,
) -> windows::Win32::Foundation::BOOL {
    let params = &*(lparam.0 as *const (u32, std::sync::Arc<std::sync::Mutex<Option<HWND>>>));
    let target_pid = params.0;
    let target_hwnd = &params.1;

    let mut process_id: u32 = 0;
    GetWindowThreadProcessId(hwnd, Some(&mut process_id));

    if process_id == target_pid {
        // 检查是否是可见窗口
        use windows::Win32::UI::WindowsAndMessaging::{GetWindowTextW, IsWindowVisible};
        
        if IsWindowVisible(hwnd).as_bool() {
            let mut title: [u16; 256] = [0; 256];
            let len = GetWindowTextW(hwnd, &mut title);
            
            if len > 0 {
                let window_title = String::from_utf16_lossy(&title[..len as usize]);
                println!("🔍 找到进程 {} 的窗口: {}", target_pid, window_title);
                
                *target_hwnd.lock().unwrap() = Some(hwnd);
                return windows::Win32::Foundation::BOOL(0); // 停止枚举
            }
        }
    }

    windows::Win32::Foundation::BOOL(1) // 继续枚举
}
