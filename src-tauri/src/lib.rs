pub mod models;
pub mod accounts;
pub mod token;
pub mod token_browser;
pub mod launcher;
pub mod settings;
pub mod game_monitor;
pub mod token_monitor;
pub mod embedded_resources;

use std::sync::{Arc, Mutex};
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // 初始化游戏监控器
            let monitor = game_monitor::GameMonitor::new(app.handle().clone());
            app.manage(Arc::new(Mutex::new(Some(monitor))));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // accounts
            accounts::get_accounts,
            accounts::add_account,
            accounts::update_account,
            accounts::delete_account,
            accounts::save_token_for_account,
            // token
            token::open_login_window,
            token::write_token_to_registry,
            token_browser::open_login_with_server,
            token_browser::open_login_with_navigation,
            // launcher
            launcher::launch_account,
            launcher::launch_all_accounts,
            // settings
            settings::get_settings,
            settings::save_settings,
            // game monitor
            game_monitor::get_game_status,
            game_monitor::start_game_monitoring,
            game_monitor::stop_game_monitoring,
            game_monitor::kill_game_process,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
