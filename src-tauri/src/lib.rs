pub mod api;
mod commands;
pub mod engine;
pub mod state;
pub mod store;

use std::sync::Arc;

use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let dir = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| std::env::temp_dir().join("ncm-cloud-drive"));
            std::fs::create_dir_all(&dir).ok();
            app.manage(Arc::new(state::AppState::new(dir)));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_state,
            commands::save_credentials_cmd,
            commands::get_profile,
            commands::login_start,
            commands::logout,
            commands::get_history,
            commands::clear_history,
            commands::pick_files,
            commands::pick_folder,
            commands::upload_files,
            commands::cancel_uploads,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
