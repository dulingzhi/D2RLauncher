/// 自定义自更新模块
///
/// tauri-plugin-updater 在 Windows 上默认将下载的更新包当作 NSIS/MSI 安装程序执行，
/// 但我们发布的是独立便携 exe（非安装包），updater 下载后直接运行新 exe 不会替换旧 exe。
/// 本模块实现自定义更新流程：
/// 1. 获取 latest.json 对比版本
/// 2. 下载新 exe 到临时目录（带进度事件）
/// 3. 创建批处理脚本：等待当前进程退出 → 复制覆盖 → 重启
/// 4. 启动脚本后退出当前进程

use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;
use tauri::{AppHandle, Emitter};

/// Windows 创建进程标志：不创建控制台窗口
const CREATE_NO_WINDOW: u32 = 0x08000000;

/// 更新信息（返回给前端）
#[derive(Serialize, Deserialize, Clone)]
pub struct UpdateInfo {
    pub version: String,
    pub notes: Option<String>,
    pub pub_date: Option<String>,
    pub download_url: String,
}

/// latest.json 结构
#[derive(Deserialize)]
struct LatestJson {
    version: String,
    notes: Option<String>,
    pub_date: Option<String>,
    platforms: std::collections::HashMap<String, PlatformInfo>,
}

#[derive(Deserialize)]
struct PlatformInfo {
    #[allow(dead_code)]
    signature: String,
    url: String,
}

/// 下载进度事件载荷
#[derive(Serialize, Clone)]
struct DownloadProgress {
    downloaded: u64,
    total: u64,
}

/// 获取更新临时目录 (%TEMP%\d2r-launcher\)
fn update_temp_dir() -> PathBuf {
    std::env::temp_dir().join("d2r-launcher")
}

/// 检查更新
///
/// 从 GitHub releases 获取 latest.json，对比当前版本，
/// 如果有新版本返回 UpdateInfo，否则返回 None
#[tauri::command]
pub async fn check_update() -> Result<Option<UpdateInfo>, String> {
    let endpoint = "https://github.com/dulingzhi/D2RLauncher/releases/latest/download/latest.json";

    let client = reqwest::Client::new();
    let response = client
        .get(endpoint)
        .send()
        .await
        .map_err(|e| format!("检查更新失败: {e}"))?;

    if !response.status().is_success() {
        return Err(format!("服务器返回错误: HTTP {}", response.status()));
    }

    let latest: LatestJson = response
        .json()
        .await
        .map_err(|e| format!("解析更新信息失败: {e}"))?;

    // 当前版本（来自 Cargo.toml，不含 v 前缀）
    let current = env!("CARGO_PKG_VERSION");
    // trim() 清除可能的 BOM/空白字符，strip_prefix 去掉 v 前缀
    let latest_ver = latest.version.trim().trim_start_matches('v');

    if latest_ver == current {
        return Ok(None);
    }

    let platform = latest
        .platforms
        .get("windows-x86_64")
        .ok_or("更新信息中缺少 windows-x86_64 平台数据")?;

    Ok(Some(UpdateInfo {
        version: latest.version,
        notes: latest.notes,
        pub_date: latest.pub_date,
        download_url: platform.url.clone(),
    }))
}

/// 执行自更新
///
/// 1. 流式下载新 exe 到临时目录，通过事件发送进度
/// 2. 创建批处理脚本（等待当前进程退出 → 覆盖 exe → 重启）
/// 3. 启动批处理脚本（隐藏窗口）
/// 4. 退出当前应用
#[tauri::command]
pub async fn perform_self_update(app: AppHandle, download_url: String) -> Result<(), String> {
    let current_exe = std::env::current_exe()
        .map_err(|e| format!("获取当前路径失败: {e}"))?;

    // 准备临时目录
    let temp_dir = update_temp_dir();
    fs::create_dir_all(&temp_dir)
        .map_err(|e| format!("创建临时目录失败: {e}"))?;

    let temp_exe = temp_dir.join("d2r-launcher-update.exe");

    // 流式下载新 exe
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {e}"))?;

    let mut response = client
        .get(&download_url)
        .send()
        .await
        .map_err(|e| format!("下载更新失败: {e}"))?;

    if !response.status().is_success() {
        return Err(format!("下载失败，HTTP 状态: {}", response.status()));
    }

    let total_size = response.content_length().unwrap_or(0);
    let mut downloaded: u64 = 0;
    let mut last_emit: u64 = 0;

    let mut file = fs::File::create(&temp_exe)
        .map_err(|e| format!("创建临时文件失败: {e}"))?;

    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|e| format!("下载数据读取失败: {e}"))?
    {
        file.write_all(&chunk)
            .map_err(|e| format!("写入临时文件失败: {e}"))?;
        downloaded += chunk.len() as u64;

        // 每 100KB 或完成时发送进度事件
        if downloaded - last_emit >= 102_400 || downloaded >= total_size && total_size > 0 {
            let _ = app.emit(
                "update-download-progress",
                DownloadProgress {
                    downloaded,
                    total: total_size,
                },
            );
            last_emit = downloaded;
        }
    }

    // 确保最终进度已发送
    if downloaded != last_emit {
        let _ = app.emit(
            "update-download-progress",
            DownloadProgress {
                downloaded,
                total: total_size,
            },
        );
    }

    drop(file); // 关闭文件句柄

    // 创建更新批处理脚本
    let current_pid = std::process::id();
    let batch_path = temp_dir.join("d2r-launcher-update.bat");
    let exe_str = current_exe.to_str().ok_or("路径包含非法字符")?;
    let temp_str = temp_exe.to_str().ok_or("临时路径包含非法字符")?;

    // 批处理脚本逻辑：
    // 1. 等待当前进程 (PID) 退出
    // 2. 将新 exe 复制覆盖到原路径（失败则重试）
    // 3. 启动新版本
    // 4. 清理临时文件
    let batch = format!(
        "@echo off\r\n\
         chcp 65001 >nul\r\n\
         :wait\r\n\
         tasklist /FI \"PID eq {pid}\" 2>nul | find \"{pid}\" >nul\r\n\
         if %ERRORLEVEL%==0 (\r\n\
             timeout /t 1 /nobreak >nul\r\n\
             goto wait\r\n\
         )\r\n\
         :copy\r\n\
         copy /y \"{temp}\" \"{exe}\"\r\n\
         if %ERRORLEVEL% neq 0 (\r\n\
             timeout /t 2 /nobreak >nul\r\n\
             goto copy\r\n\
         )\r\n\
         start \"\" \"{exe}\"\r\n\
         del \"{temp}\"\r\n\
         exit",
        pid = current_pid,
        temp = temp_str,
        exe = exe_str,
    );

    fs::write(&batch_path, batch)
        .map_err(|e| format!("写入更新脚本失败: {e}"))?;

    // 启动批处理脚本（隐藏窗口）
    Command::new("cmd")
        .args(["/c", batch_path.to_str().ok_or("脚本路径非法")?])
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map_err(|e| format!("启动更新脚本失败: {e}"))?;

    // 退出当前应用
    app.exit(0);

    Ok(())
}

/// 清理更新残留的临时文件
///
/// 在应用启动时调用，删除上次更新可能遗留的临时文件
#[tauri::command]
pub fn cleanup_update() -> Result<(), String> {
    let temp_dir = update_temp_dir();
    let temp_exe = temp_dir.join("d2r-launcher-update.exe");
    let batch = temp_dir.join("d2r-launcher-update.bat");

    if temp_exe.exists() {
        let _ = fs::remove_file(&temp_exe);
    }
    if batch.exists() {
        let _ = fs::remove_file(&batch);
    }

    Ok(())
}
