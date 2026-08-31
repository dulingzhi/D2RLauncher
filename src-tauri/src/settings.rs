use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use tauri::Manager;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// 兼容旧配置：D2R(osic) 安装目录，读取时归一到 game_paths
    pub game_path: String,
    /// uid → 安装根目录
    #[serde(default)]
    pub game_paths: HashMap<String, String>,
    pub handle_path: String,
    pub launch_delay_secs: u64,
    /// 是否等待登录完成后再启动下一个账号
    #[serde(default = "default_wait_for_login")]
    pub wait_for_login: bool,
    /// 登录检测超时时间（秒）
    #[serde(default = "default_login_timeout")]
    pub login_timeout_secs: u64,
    /// 是否修改游戏窗口标题为账号名称（方便识别）
    #[serde(default = "default_rename_window")]
    pub rename_window: bool,
}

/// 把旧版 game_path 字段归一为 game_paths["osic"]（不覆盖已有值）
pub fn normalize_game_paths(settings: &mut Settings) {
    let legacy = settings.game_path.trim().to_string();
    if !legacy.is_empty() {
        settings
            .game_paths
            .entry("osic".to_string())
            .or_insert(legacy);
    }
}

/// 取指定游戏的安装根目录（优先 game_paths，osic 回退旧字段）
pub fn game_path_for(settings: &Settings, uid: &str) -> String {
    if let Some(p) = settings.game_paths.get(uid) {
        if !p.trim().is_empty() {
            return p.clone();
        }
    }
    if uid == "osic" {
        return settings.game_path.clone();
    }
    String::new()
}

fn default_wait_for_login() -> bool {
    true
}

fn default_login_timeout() -> u64 {
    60 // 60秒超时
}

fn default_rename_window() -> bool {
    true
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            game_path: r"C:\Program Files (x86)\Battle.net\Games\Diablo II Resurrected".to_string(),
            game_paths: HashMap::new(),
            handle_path: String::new(),
            launch_delay_secs: 5,
            wait_for_login: true,
            login_timeout_secs: 60,
            rename_window: true,
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
    let mut settings: Settings = serde_json::from_str(&content).unwrap_or_default();
    normalize_game_paths(&mut settings);
    settings
}

#[tauri::command]
pub fn save_settings(app: tauri::AppHandle, mut settings: Settings) -> Result<(), String> {
    normalize_game_paths(&mut settings);
    let path = settings_path(&app);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
    fs::write(&path, json).map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_settings() -> Settings {
        Settings::default()
    }

    #[test]
    fn legacy_game_path_seeds_osic_map_entry() {
        let mut s = base_settings();
        s.game_path = r"C:\Games\D2R".to_string();
        normalize_game_paths(&mut s);
        assert_eq!(s.game_paths.get("osic").unwrap(), r"C:\Games\D2R");
    }

    #[test]
    fn normalize_keeps_existing_map_entry() {
        let mut s = base_settings();
        s.game_path = r"C:\Old\D2R".to_string();
        s.game_paths.insert("osic".to_string(), r"C:\New\D2R".to_string());
        normalize_game_paths(&mut s);
        assert_eq!(s.game_paths.get("osic").unwrap(), r"C:\New\D2R");
    }

    #[test]
    fn game_path_for_prefers_map_then_legacy_fallback() {
        let mut s = base_settings();
        s.game_path = r"C:\Legacy\D2R".to_string();
        normalize_game_paths(&mut s);
        s.game_paths.insert("wow".to_string(), r"C:\WoW".to_string());

        assert_eq!(game_path_for(&s, "osic"), r"C:\Legacy\D2R");
        assert_eq!(game_path_for(&s, "wow"), r"C:\WoW");
        // 未配置的游戏返回空串（调用方据此提示设置）
        assert_eq!(game_path_for(&s, "xyz"), "");
    }

    #[test]
    fn missing_game_paths_field_deserializes_from_legacy_json() {
        let json = r#"{"game_path":"C:\\D2R","handle_path":"","launch_delay_secs":5,"wait_for_login":true,"login_timeout_secs":60,"rename_window":true}"#;
        let s: Settings = serde_json::from_str(json).expect("旧配置应可反序列化");
        assert!(s.game_paths.is_empty());
        assert_eq!(s.game_path, r"C:\D2R");
    }
}
