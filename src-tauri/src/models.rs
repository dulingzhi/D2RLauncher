use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub id: String,
    pub label: String,
    /// base64 编码的 DPAPI 加密 token
    pub encrypted_token: Option<String>,
    /// token 获取时间 (Unix timestamp seconds)
    pub token_set_at: Option<i64>,
    pub custom_args: String,
    pub window_x: Option<i32>,
    pub window_y: Option<i32>,
    #[serde(default)]
    pub window_width: Option<i32>,
    #[serde(default)]
    pub window_height: Option<i32>,
}
