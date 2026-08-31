use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub id: String,
    pub label: String,
    /// base64 编码的 DPAPI 加密 token
    pub encrypted_token: Option<String>,
    /// token 获取时间 (Unix timestamp seconds)
    pub token_set_at: Option<i64>,
    /// 所属游戏 uid，见 games::get_game（旧数据默认 "osic"）
    #[serde(default = "default_game")]
    pub game: String,
    /// WoW 客户端分支：retail / classic / classic_era（仅 wow 使用）
    #[serde(default = "default_flavor")]
    pub flavor: String,
    pub custom_args: String,
    pub window_x: Option<i32>,
    pub window_y: Option<i32>,
    #[serde(default)]
    pub window_width: Option<i32>,
    #[serde(default)]
    pub window_height: Option<i32>,
}

fn default_game() -> String {
    "osic".to_string()
}

fn default_flavor() -> String {
    "retail".to_string()
}
