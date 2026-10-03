mod commands;
mod config;
mod menu;
mod logs;
mod nodes;
mod process;
mod state;
mod windows;

use std::sync::Arc;
use tauri::webview::{NewWindowResponse, WebviewWindowBuilder};
use tauri::Manager;

use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        // Let the webview handle links and downloads like a normal browser.
        // The opener plugin's injected click handler prevents `_blank` links
        // before navigation, which breaks pages loaded from ComfyUI's HTTP
        // origin where Tauri IPC is unavailable.
        .plugin(
            tauri_plugin_opener::Builder::new()
                .open_js_links_on_click(false)
                .build(),
        )
        .plugin(tauri_plugin_dialog::init())
        .on_window_event(|window, event| {
            if window.label() == "main" && matches!(event, tauri::WindowEvent::CloseRequested { .. }) {
                window.app_handle().exit(0);
            }
        })
        .setup(|app| {
            // The main window is created here instead of automatically from
            // tauri.conf.json so we can restore WebView2's default handling
            // for `target="_blank"`, window.open(), downloads, and custom URL
            // schemes. Wry otherwise suppresses every new-window request when
            // no handler is registered.
            let main_config = app
                .config()
                .app
                .windows
                .first()
                .ok_or_else(|| std::io::Error::other("main window config is missing"))?;
            WebviewWindowBuilder::from_config(app.handle(), main_config)?
                .initialization_script(include_str!("launcher.js"))
                // ComfyUI uses the Clipboard API for copying and pasting
                // workflow data. Wry leaves clipboard-read access disabled by
                // default even though regular browsers expose it to localhost.
                .enable_clipboard_access()
                .on_new_window(|_, _| NewWindowResponse::Allow)
                .build()?;

            let handle = app.handle().clone();
            let cfg = config::load(&handle);
            let app_state = Arc::new(AppState::new(cfg));
            app.manage(app_state.clone());

            // Explicitly apply the embedded transparent icon to the native
            // window. Windows can otherwise keep showing a stale cached icon
            // for the stable dev executable/App ID after icon assets change.
            if let (Some(window), Some(icon)) =
                (app.get_webview_window("main"), app.default_window_icon())
            {
                window.set_icon(icon.clone())?;
            }

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
            commands::launcher_pick_directory,
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
