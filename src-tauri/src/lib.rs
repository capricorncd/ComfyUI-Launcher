mod commands;
mod config;
mod menu;
mod nodes;
mod process;
mod state;
mod windows;

use std::sync::Arc;
use tauri::Manager;

use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let handle = app.handle().clone();
            let cfg = config::load(&handle);
            let app_state = Arc::new(AppState::new(cfg));
            app.manage(app_state.clone());

            if let Some(window) = app.get_webview_window("main") {
                if let Ok(home) = window.url() {
                    *app_state.home_url.lock().unwrap() = Some(home);
                }
            }

            menu::build_and_attach(&handle)?;

            let handle2 = handle.clone();
            tauri::async_runtime::spawn(async move {
                let state = handle2.state::<Arc<AppState>>().inner().clone();
                let _ = process::start_or_restart(handle2.clone(), state).await;
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_status,
            commands::start_or_restart,
            commands::get_config,
            commands::save_config,
            commands::list_custom_nodes,
            commands::pull_node,
            commands::clone_node,
            commands::check_node_updates,
            commands::open_folder,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
