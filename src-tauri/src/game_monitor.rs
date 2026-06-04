use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::collections::HashMap;
use sysinfo::System;
use tauri::{AppHandle, Emitter};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{EnumWindows, GetWindowTextW, GetWindowThreadProcessId};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameInstance {
    pub account_id: String,
    pub account_name: String,
    pub region: String,
    pub process_id: u32,
    pub window_title: String,
    pub start_time: std::time::SystemTime,
    pub total_seconds: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct GameStatus {
    pub running_instances: Vec<GameInstance>,
    pub total_running: usize,
}

pub struct GameMonitor {
    instances: Arc<Mutex<Vec<GameInstance>>>,
    start_times: Arc<Mutex<HashMap<u32, std::time::Instant>>>,
    app_handle: AppHandle,
    monitoring: Arc<Mutex<bool>>,
}

impl GameMonitor {
    pub fn new(app_handle: AppHandle) -> Self {
        Self {
            instances: Arc::new(Mutex::new(Vec::new())),
            start_times: Arc::new(Mutex::new(HashMap::new())),
            app_handle,
            monitoring: Arc::new(Mutex::new(false)),
        }
    }

    /// 启动游戏监控（每 2 秒检查一次）
    pub fn start_monitoring(&self) {
        let instances = Arc::clone(&self.instances);
        let start_times = Arc::clone(&self.start_times);
        let app_handle = self.app_handle.clone();
        let monitoring = Arc::clone(&self.monitoring);

        // 设置监控标志
        *monitoring.lock().unwrap() = true;
        let monitoring_clone = Arc::clone(&monitoring);

        std::thread::spawn(move || {
            println!("🎮 游戏监控线程已启动（检测间隔: 2秒）");
            
            // 立即执行一次检查
            Self::check_and_update(&instances, &start_times, &app_handle);

            while *monitoring_clone.lock().unwrap() {
                // 每 2 秒检查一次，快速响应状态变化
                std::thread::sleep(Duration::from_secs(2));
                Self::check_and_update(&instances, &start_times, &app_handle);
            }

            println!("🛑 游戏监控线程已停止");
        });
    }
    
    fn check_and_update(
        instances: &Arc<Mutex<Vec<GameInstance>>>,
        start_times: &Arc<Mutex<HashMap<u32, std::time::Instant>>>,
        app_handle: &AppHandle,
    ) {
        if let Ok(running_games) = Self::check_running_games() {
            println!("🔍 当前检测到 {} 个运行中的游戏", running_games.len());
            let mut times_guard = start_times.lock().unwrap();
            let now = std::time::Instant::now();
            
            // 更新运行时长
            let games_with_time: Vec<GameInstance> = running_games.into_iter().map(|mut game| {
                let start = times_guard.entry(game.process_id).or_insert(now);
                game.total_seconds = start.elapsed().as_secs();
                game
            }).collect();
            
            // 清理已经不存在的进程
            let current_pids: Vec<u32> = games_with_time.iter().map(|g| g.process_id).collect();
            times_guard.retain(|pid, _| current_pids.contains(pid));
            drop(times_guard);
            
            // 更新实例列表
            let mut instances_guard = instances.lock().unwrap();
            *instances_guard = games_with_time.clone();
            drop(instances_guard);

            // 发送状态更新事件到前端
            let status = GameStatus {
                total_running: games_with_time.len(),
                running_instances: games_with_time,
            };

            let _ = app_handle.emit("game_status_update", &status);
        }
    }

    /// 停止监控
    pub fn stop_monitoring(&self) {
        *self.monitoring.lock().unwrap() = false;
    }

    /// 检查当前运行的游戏实例
    fn check_running_games() -> Result<Vec<GameInstance>, String> {
        let mut system = System::new();
        system.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

        let mut instances = Vec::new();
        let mut windows_map: std::collections::HashMap<u32, String> = std::collections::HashMap::new();

        // 获取所有 D2R 窗口
        unsafe {
            let _ = EnumWindows(
                Some(enum_window_callback),
                windows::Win32::Foundation::LPARAM(&mut windows_map as *mut _ as isize),
            );
        }

        // 查找所有 D2R.exe 进程
        for process in system.processes_by_name(std::ffi::OsStr::new("D2R.exe")) {
            let pid = process.pid().as_u32();

            // 检查窗口标题
            if let Some(window_title) = windows_map.get(&pid) {
                // 窗口标题格式: "{ID} - {Name} ({Region}) - Diablo II: Resurrected"
                if window_title.ends_with("- Diablo II: Resurrected") {
                    if let Some(parsed) = Self::parse_window_title(window_title) {
                        instances.push(GameInstance {
                            account_id: parsed.0,
                            account_name: parsed.1,
                            region: parsed.2,
                            process_id: pid,
                            window_title: window_title.clone(),
                            start_time: std::time::SystemTime::now(), // 简化版，实际应该记录真实启动时间
                            total_seconds: 0,
                        });
                    }
                }
            }
        }

        Ok(instances)
    }

    /// 解析窗口标题，提取账号信息
    /// 格式: "[UUID] MyAccount (cn) - Diablo II: Resurrected"
    fn parse_window_title(title: &str) -> Option<(String, String, String)> {
        if !title.ends_with("- Diablo II: Resurrected") {
            return None;
        }

        // 移除 "- Diablo II: Resurrected" 后缀
        let main_part = title.trim_end_matches("- Diablo II: Resurrected").trim().trim_end_matches('-').trim();

        // 解析格式: "[ID] Name (Region)"
        // 提取 [ID]
        if !main_part.starts_with('[') {
            return None;
        }
        
        let close_bracket = main_part.find(']')?;
        let account_id = main_part[1..close_bracket].to_string();
        let remainder = main_part[close_bracket + 1..].trim();

        // 提取 Name 和 Region: "Name (Region)"
        if let Some(paren_pos) = remainder.rfind('(') {
            let account_name = remainder[..paren_pos].trim().to_string();
            let region_part = &remainder[paren_pos + 1..];
            if let Some(close_paren) = region_part.find(')') {
                let region = region_part[..close_paren].trim().to_string();
                println!("✅ 解析窗口标题成功: ID={}, Name={}, Region={}", account_id, account_name, region);
                return Some((account_id, account_name, region));
            }
        }
        
        println!("❌ 解析窗口标题失败: {}", title);
        None
    }

    /// 获取当前运行的游戏实例
    pub fn get_running_instances(&self) -> Vec<GameInstance> {
        self.instances.lock().unwrap().clone()
    }
}

// Windows API 回调函数：枚举所有窗口
unsafe extern "system" fn enum_window_callback(
    hwnd: HWND,
    lparam: windows::Win32::Foundation::LPARAM,
) -> windows::Win32::Foundation::BOOL {
    let map = &mut *(lparam.0 as *mut std::collections::HashMap<u32, String>);

    // 获取窗口进程 ID
    let mut process_id: u32 = 0;
    GetWindowThreadProcessId(hwnd, Some(&mut process_id));

    // 获取窗口标题
    let mut title: [u16; 512] = [0; 512];
    let len = GetWindowTextW(hwnd, &mut title);

    if len > 0 {
        let window_title = String::from_utf16_lossy(&title[..len as usize]);
        
        // 只记录 D2R 相关窗口
        if window_title.contains("Diablo II: Resurrected") {
            println!("🎮 检测到 D2R 窗口 (PID {}): {}", process_id, window_title);
            map.insert(process_id, window_title);
        }
    }

    windows::Win32::Foundation::BOOL(1) // 继续枚举
}

/// Tauri 命令：获取当前运行的游戏状态
#[tauri::command]
pub fn get_game_status(
    app: tauri::State<'_, Arc<Mutex<Option<GameMonitor>>>>,
) -> Result<GameStatus, String> {
    let monitor_guard = app.lock().map_err(|e| e.to_string())?;

    if let Some(monitor) = monitor_guard.as_ref() {
        let instances = monitor.get_running_instances();
        Ok(GameStatus {
            total_running: instances.len(),
            running_instances: instances,
        })
    } else {
        Ok(GameStatus {
            total_running: 0,
            running_instances: Vec::new(),
        })
    }
}

/// Tauri 命令：启动游戏监控
#[tauri::command]
pub fn start_game_monitoring(
    app: tauri::State<'_, Arc<Mutex<Option<GameMonitor>>>>,
) -> Result<(), String> {
    let mut monitor_guard = app.lock().map_err(|e| e.to_string())?;

    if monitor_guard.is_none() {
        return Err("游戏监控器未初始化".to_string());
    }

    if let Some(monitor) = monitor_guard.as_mut() {
        monitor.start_monitoring();
        Ok(())
    } else {
        Err("无法启动监控".to_string())
    }
}

/// Tauri 命令：停止游戏监控
#[tauri::command]
pub fn stop_game_monitoring(
    app: tauri::State<'_, Arc<Mutex<Option<GameMonitor>>>>,
) -> Result<(), String> {
    let monitor_guard = app.lock().map_err(|e| e.to_string())?;

    if let Some(monitor) = monitor_guard.as_ref() {
        monitor.stop_monitoring();
        Ok(())
    } else {
        Err("游戏监控器未初始化".to_string())
    }
}

/// Tauri 命令：关闭指定进程
#[tauri::command]
pub fn kill_game_process(process_id: u32) -> Result<(), String> {
    use sysinfo::{Pid, System};
    
    let mut system = System::new();
    system.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
    
    let pid = Pid::from_u32(process_id);
    
    if let Some(process) = system.process(pid) {
        if process.kill() {
            Ok(())
        } else {
            Err("无法终止进程".to_string())
        }
    } else {
        Err("进程不存在".to_string())
    }
}
