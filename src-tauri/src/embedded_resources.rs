/// 内置资源管理模块
/// 将 handle64.exe 编译进二进制，运行时释放到临时目录

use std::fs;
use std::path::PathBuf;
use std::io::Write;

/// 嵌入的 handle64.exe 二进制数据
static HANDLE64_EXE: &[u8] = include_bytes!("../resources/handle64.exe");

/// 获取 handle64.exe 的路径，自动释放到临时目录
/// 
/// 每次调用都会检查文件是否存在，如果不存在则重新释放
pub fn get_handle64_path() -> Result<PathBuf, String> {
    // 使用应用临时目录
    let temp_dir = std::env::temp_dir().join("d2r-launcher");
    
    // 确保目录存在
    fs::create_dir_all(&temp_dir)
        .map_err(|e| format!("创建临时目录失败: {}", e))?;
    
    let handle_path = temp_dir.join("handle64.exe");
    
    // 检查文件是否已存在且大小正确
    let needs_extract = if handle_path.exists() {
        match fs::metadata(&handle_path) {
            Ok(metadata) => metadata.len() != HANDLE64_EXE.len() as u64,
            Err(_) => true,
        }
    } else {
        true
    };
    
    if needs_extract {
        println!("📦 正在释放内置 handle64.exe 到: {}", handle_path.display());
        
        let mut file = fs::File::create(&handle_path)
            .map_err(|e| format!("创建 handle64.exe 失败: {}", e))?;
        
        file.write_all(HANDLE64_EXE)
            .map_err(|e| format!("写入 handle64.exe 失败: {}", e))?;
        
        println!("✅ handle64.exe 释放成功 ({}字节)", HANDLE64_EXE.len());
    }
    
    Ok(handle_path)
}

/// 清理临时释放的 handle64.exe
pub fn cleanup_handle64() -> Result<(), String> {
    let temp_dir = std::env::temp_dir().join("d2r-launcher");
    let handle_path = temp_dir.join("handle64.exe");
    
    if handle_path.exists() {
        fs::remove_file(&handle_path)
            .map_err(|e| format!("删除临时文件失败: {}", e))?;
        println!("🗑️ 已清理 handle64.exe");
    }
    
    Ok(())
}
