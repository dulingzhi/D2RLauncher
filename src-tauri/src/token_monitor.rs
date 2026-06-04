// Windows 注册表 Token 监控模块
// 用于检测 D2R 登录完成（到达角色选择界面）

use std::thread;
use std::time::Duration;

#[cfg(windows)]
use winreg::enums::*;
#[cfg(windows)]
use winreg::RegKey;

/// 监控注册表 WEB_TOKEN 变化，等待登录完成
/// 返回是否成功检测到登录完成
#[cfg(windows)]
pub fn wait_for_login_complete(timeout_seconds: u64) -> Result<bool, String> {
    println!("🔐 开始监控登录状态...");
    
    // 打开注册表路径: HKCU\SOFTWARE\Blizzard Entertainment\Battle.net\Launch Options\OSI
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let path = r"SOFTWARE\Blizzard Entertainment\Battle.net\Launch Options\OSI";
    
    let key = match hkcu.open_subkey(path) {
        Ok(k) => k,
        Err(e) => {
            return Err(format!("⚠️ 无法打开注册表路径: {}", e));
        }
    };
    
    // 获取初始 Token 值（尝试以字节形式读取）
    let initial_token: Vec<u8> = match key.get_raw_value("WEB_TOKEN") {
        Ok(val) => val.bytes,
        Err(e) => {
            return Err(format!("⚠️ 无法读取 WEB_TOKEN: {}", e));
        }
    };
    
    println!("📋 初始 Token 长度: {} 字节", initial_token.len());
    println!("⏳ 等待到达角色选择界面（Token 将会变化）...");
    println!("⚠️ 在此期间请勿关闭游戏或启动其他账号！");
    
    let mut token_change_count = 0;
    let start_time = std::time::Instant::now();
    let timeout = Duration::from_secs(timeout_seconds);
    
    loop {
        // 检查超时
        if start_time.elapsed() > timeout {
            println!("⏰ 登录检测超时（{}秒）", timeout_seconds);
            return Ok(false);
        }
        
        thread::sleep(Duration::from_millis(451)); // 使用 PS1 脚本相同的间隔
        
        // 读取当前 Token（以字节形式）
        let current_token: Vec<u8> = match key.get_raw_value("WEB_TOKEN") {
            Ok(val) => val.bytes,
            Err(e) => {
                println!("⚠️ 读取 Token 失败: {}", e);
                continue;
            }
        };
        
        // 检测 Token 变化
        if current_token != initial_token {
            token_change_count += 1;
            println!("🔄 检测到 Token 变化（第{}次）", token_change_count);
            
            // Token 变化 1 次 = 到达角色选择界面
            if token_change_count >= 1 {
                println!("✅ 登录完成！已到达角色选择界面");
                return Ok(true);
            }
            
            // 更新比较基准
            // initial_token = current_token; // 不需要更新，只等待第一次变化
        }
    }
}

#[cfg(not(windows))]
pub fn wait_for_login_complete(_timeout_seconds: u64) -> Result<bool, String> {
    Err("Token 监控仅支持 Windows 平台".to_string())
}

/// 获取当前 WEB_TOKEN 值（调试用）
#[cfg(windows)]
pub fn get_current_token() -> Result<String, String> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let path = r"SOFTWARE\Blizzard Entertainment\Battle.net\Launch Options\OSI";
    
    let key = match hkcu.open_subkey(path) {
        Ok(k) => k,
        Err(e) => {
            return Err(format!("无法打开注册表: {}", e));
        }
    };
    
    // 尝试读取原始字节数据
    let token_bytes: Vec<u8> = match key.get_raw_value("WEB_TOKEN") {
        Ok(val) => val.bytes,
        Err(e) => {
            return Err(format!("无法读取 WEB_TOKEN: {}", e));
        }
    };
    
    // 尝试转换为 UTF-8 字符串，如果失败则返回十六进制表示
    match String::from_utf8(token_bytes.clone()) {
        Ok(s) => Ok(s),
        Err(_) => {
            // 如果不是有效的 UTF-8，返回十六进制表示（仅前64字节）
            let hex_str: String = token_bytes
                .iter()
                .take(64)
                .map(|b| format!("{:02x}", b))
                .collect::<Vec<_>>()
                .join("");
            Ok(format!("binary:{}", hex_str))
        }
    }
}

#[cfg(not(windows))]
pub fn get_current_token() -> Result<String, String> {
    Err("仅支持 Windows 平台".to_string())
}
