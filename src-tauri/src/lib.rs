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
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let handle = app.handle().clone();
            let cfg = config::load(&handle);
            let app_state = Arc::new(AppState::new(cfg));
            app.manage(app_state.clone());

            // The window's initial navigation to index.html hasn't necessarily
            // started yet at this point in setup() — `window.url()` can still
            // report "about:blank" here. Poll briefly until it reflects the
            // real target, so a later restart navigates "home" correctly
            // instead of to a blank page.
            if let Some(window) = app.get_webview_window("main") {
                let app_state2 = app_state.clone();
                tauri::async_runtime::spawn(async move {
                    for _ in 0..100 {
                        if let Ok(home) = window.url() {
                            if home.as_str() != "about:blank" {
                                *app_state2.home_url.lock().unwrap() = Some(home);
                                return;
                            }
                        }
                        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
                    }
                });
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
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|app_handle, event| {
        if matches!(event, tauri::RunEvent::ExitRequested { .. }) {
            let state = app_handle.state::<Arc<AppState>>().inner().clone();
            process::stop_tracked_process(&state);
        }
    });
}
