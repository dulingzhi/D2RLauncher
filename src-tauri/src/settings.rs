use std::fs;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use tauri::Manager;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub game_path: String,
    pub handle_path: String,
    pub launch_delay_secs: u64,
    /// 是否等待登录完成后再启动下一个账号
    #[serde(default = "default_wait_for_login")]
    pub wait_for_login: bool,
    /// 登录检测超时时间（秒）
    #[serde(default = "default_login_timeout")]
    pub login_timeout_secs: u64,
}

fn default_wait_for_login() -> bool {
    true
}

fn default_login_timeout() -> u64 {
    60 // 60秒超时
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            game_path: r"C:\Program Files (x86)\Battle.net\Games\Diablo II Resurrected".to_string(),
            handle_path: String::new(),
            launch_delay_secs: 5,
            wait_for_login: true,
            login_timeout_secs: 60,
        }
    }
}

fn settings_path(app: &tauri::AppHandle) -> PathBuf {
    app.path()
        .app_data_dir()
        .expect("failed to get app data dir")
        .join("settings.json")
}

#[tauri::command]
pub fn get_settings(app: tauri::AppHandle) -> Settings {
    let path = settings_path(&app);
    if !path.exists() {
        return Settings::default();
    }
    let content = fs::read_to_string(&path).unwrap_or_default();
    serde_json::from_str(&content).unwrap_or_default()
}

#[tauri::command]
pub fn save_settings(app: tauri::AppHandle, settings: Settings) -> Result<(), String> {
    let path = settings_path(&app);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
    fs::write(&path, json).map_err(|e| e.to_string())?;
    Ok(())
}
