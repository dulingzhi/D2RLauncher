/// 使用本地 HTTP 服务器自动捕获 Battle.net token
use tauri::{AppHandle, Emitter, Manager, WebviewWindowBuilder, WebviewUrl};
use std::net::TcpListener;
use std::io::{Read, Write};
use std::thread;

const LOGIN_HTML: &str = include_str!("../login-proxy.html");
const SERVER_PORT: u16 = 31888;

/// 打开 token 提取页面并启动本地服务器
#[tauri::command]
pub async fn open_login_with_server(app: AppHandle, account_id: String) -> Result<(), String> {
    // 启动本地 HTTP 服务器
    let listener = TcpListener::bind(format!("127.0.0.1:{}", SERVER_PORT))
        .map_err(|e| format!("无法启动本地服务器: {} (端口 {} 可能被占用)", e, SERVER_PORT))?;
    
    println!("✅ 本地服务器已启动: http://localhost:{}", SERVER_PORT);
    
    let app_clone = app.clone();
    let aid = account_id.clone();
    
    // 在后台线程中处理 HTTP 请求
    thread::spawn(move || {
        loop {
            match listener.accept() {
                Ok((mut stream, addr)) => {
                    println!("📥 收到连接: {}", addr);
                    
                    let mut buffer = [0; 8192];
                    if let Ok(size) = stream.read(&mut buffer) {
                        let request = String::from_utf8_lossy(&buffer[..size]);
                        let first_line = request.lines().next().unwrap_or("");
                        println!("📨 请求: {}", first_line);
                        
                        // 路由处理
                        if first_line.contains("GET / ") || first_line.contains("GET /index") {
                            // 返回 HTML 页面
                            let response = format!(
                                "HTTP/1.1 200 OK\r\n\
                                 Content-Type: text/html; charset=utf-8\r\n\
                                 Content-Length: {}\r\n\
                                 Connection: close\r\n\
                                 \r\n\
                                 {}",
                                LOGIN_HTML.len(),
                                LOGIN_HTML
                            );
                            let _ = stream.write_all(response.as_bytes());
                            
                        } else if first_line.contains("POST /submit-token") {
                            // 提取请求体
                            if let Some(body_start) = request.find("\r\n\r\n") {
                                let body = &request[body_start + 4..];
                                
                                // 解析 JSON 获取 token
                                if let Ok(json) = serde_json::from_str::<serde_json::Value>(body) {
                                    if let Some(token) = json.get("token").and_then(|t| t.as_str()) {
                                        println!("🎉 收到 token: {}", token);
                                        
                                        // 发送事件给前端
                                        let _ = app_clone.emit(
                                            "token_captured",
                                            serde_json::json!({
                                                "account_id": aid.clone(),
                                                "token": token
                                            }),
                                        );
                                        
                                        // 返回成功响应
                                        let response_json = r#"{"success":true}"#;
                                        let response = format!(
                                            "HTTP/1.1 200 OK\r\n\
                                             Content-Type: application/json\r\n\
                                             Access-Control-Allow-Origin: *\r\n\
                                             Content-Length: {}\r\n\
                                             Connection: close\r\n\
                                             \r\n\
                                             {}",
                                            response_json.len(),
                                            response_json
                                        );
                                        let _ = stream.write_all(response.as_bytes());
                                        
                                        // 可以选择停止服务器（此处继续运行以便重用）
                                        // break;
                                        continue;
                                    }
                                }
                            }
                            
                            // 返回错误响应
                            let error_json = r#"{"success":false,"error":"Invalid request"}"#;
                            let response = format!(
                                "HTTP/1.1 400 Bad Request\r\n\
                                 Content-Type: application/json\r\n\
                                 Content-Length: {}\r\n\
                                 Connection: close\r\n\
                                 \r\n\
                                 {}",
                                error_json.len(),
                                error_json
                            );
                            let _ = stream.write_all(response.as_bytes());
                            
                        } else if first_line.contains("OPTIONS") {
                            // CORS 预检请求
                            let response = "HTTP/1.1 200 OK\r\n\
                                           Access-Control-Allow-Origin: *\r\n\
                                           Access-Control-Allow-Methods: GET, POST, OPTIONS\r\n\
                                           Access-Control-Allow-Headers: Content-Type\r\n\
                                           Connection: close\r\n\
                                           \r\n";
                            let _ = stream.write_all(response.as_bytes());
                        } else {
                            // 404
                            let response = "HTTP/1.1 404 Not Found\r\n\
                                           Connection: close\r\n\
                                           \r\n";
                            let _ = stream.write_all(response.as_bytes());
                        }
                    }
                }
                Err(e) => {
                    eprintln!("❌ 接受连接失败: {}", e);
                    break;
                }
            }
        }
    });
    
    // 在 Tauri 窗口中打开本地页面
    let local_url = format!("http://localhost:{}", SERVER_PORT);
    
    WebviewWindowBuilder::new(
        &app,
        "token-extractor",
        WebviewUrl::External(local_url.parse().unwrap()),
    )
    .title("Token 提取工具")
    .inner_size(650.0, 700.0)
    .center()
    .resizable(true)
    .build()
    .map_err(|e| format!("无法创建窗口: {}", e))?;
    
    println!("🌐 Token 提取页面已打开");
    
    Ok(())
}

/// 打开 Battle.net 登录页面，并自动监听 URL 变化捕获 token
/// game 为账号所属游戏 uid（"osic" / "wow"），决定登录页的 app= 参数
#[tauri::command]
pub async fn open_login_with_navigation(
    app: AppHandle,
    account_id: String,
    game: Option<String>,
) -> Result<(), String> {
    let cfg = crate::games::get_game(&game.unwrap_or_else(|| "osic".to_string()))
        .ok_or_else(|| "不支持的游戏 uid".to_string())?;
    let login_url = cfg.login_url();
    
    let account_id_for_nav = account_id.clone();
    let account_id_for_close = account_id.clone();
    let app_for_nav = app.clone();
    let app_for_close = app.clone();
    
    let window = WebviewWindowBuilder::new(
        &app,
        "battle-net-login",
        WebviewUrl::External(login_url.parse().unwrap()),
    )
    .title("Battle.net CN 登录")
    .inner_size(960.0, 720.0)
    .center()
    .resizable(true)
    .incognito(true)
    .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
    .on_navigation(move |url| {
        let url_str = url.as_str();
        println!("🔗 导航到: {}", url_str);
        
        // 检测是否跳转到 localhost:0（表示登录成功）
        if url_str.starts_with("http://localhost:0/") || url_str.starts_with("http://127.0.0.1:0/") {
            println!("✅ 检测到登录成功，提取 token...");
            
            // 从 URL 中提取 ST 参数（token）
            if let Ok(parsed_url) = url::Url::parse(url_str) {
                if let Some(token) = parsed_url.query_pairs().find(|(key, _)| key == "ST").map(|(_, value)| value.to_string()) {
                    println!("🎉 成功提取 token: {}", token);
                    
                    // 发送事件到前端
                    let _ = app_for_nav.emit(
                        "token_captured",
                        serde_json::json!({
                            "account_id": account_id_for_nav.clone(),
                            "token": token
                        }),
                    );
                    
                    // 关闭登录窗口
                    if let Some(window) = app_for_nav.get_webview_window("battle-net-login") {
                        let _ = window.close();
                        println!("🔒 登录窗口已关闭");
                    }
                } else {
                    println!("⚠️ 未在 URL 中找到 ST 参数");
                }
            }
            
            // 阻止导航到 localhost:0（避免错误页面）
            return false;
        }
        
        // 允许其他导航
        true
    })
    .build()
    .map_err(|e| format!("无法创建登录窗口: {}", e))?;
    
    // 监听窗口关闭事件
    window.on_window_event(move |event| {
        use tauri::WindowEvent;
        
        if let WindowEvent::Destroyed = event {
            println!("🚪 登录窗口已关闭");
            // 发送取消事件到前端
            let _ = app_for_close.emit(
                "login_cancelled",
                serde_json::json!({
                    "account_id": account_id_for_close.clone()
                }),
            );
        }
    });
    
    println!("🌐 Battle.net 登录窗口已打开（自动捕获模式）");
    
    Ok(())
}

