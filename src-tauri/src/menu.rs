use std::sync::Arc;
use tauri::menu::{MenuBuilder, MenuItemBuilder, SubmenuBuilder};
use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};

use crate::state::AppState;
use crate::{nodes, process, windows};

pub fn build_and_attach(app: &AppHandle) -> Result<(), String> {
    let start_restart = MenuItemBuilder::with_id("start_restart", "启动/重启").build(app).map_err(|e| e.to_string())?;
    let reload_page = MenuItemBuilder::with_id("reload_page", "刷新页面").build(app).map_err(|e| e.to_string())?;
    let update_comfyui = MenuItemBuilder::with_id("update_comfyui", "更新 ComfyUI 代码").build(app).map_err(|e| e.to_string())?;
    let manage_nodes = MenuItemBuilder::with_id("manage_nodes", "自定义节点管理").build(app).map_err(|e| e.to_string())?;
    let open_comfyui_dir = MenuItemBuilder::with_id("open_comfyui_dir", "ComfyUI 目录").build(app).map_err(|e| e.to_string())?;
    let open_nodes_dir = MenuItemBuilder::with_id("open_nodes_dir", "自定义节点目录").build(app).map_err(|e| e.to_string())?;
    let open_output_dir = MenuItemBuilder::with_id("open_output_dir", "输出目录").build(app).map_err(|e| e.to_string())?;
    let open_models_dir = MenuItemBuilder::with_id("open_models_dir", "模型目录").build(app).map_err(|e| e.to_string())?;
    let settings = MenuItemBuilder::with_id("settings", "设置").build(app).map_err(|e| e.to_string())?;

    let open_submenu = SubmenuBuilder::new(app, "打开")
        .item(&open_comfyui_dir)
        .item(&open_nodes_dir)
        .item(&open_output_dir)
        .item(&open_models_dir)
        .build()
        .map_err(|e| e.to_string())?;

    let file_menu = SubmenuBuilder::new(app, "File")
        .item(&start_restart)
        .item(&reload_page)
        .item(&update_comfyui)
        .separator()
        .item(&manage_nodes)
        .item(&open_submenu)
        .separator()
        .item(&settings)
        .build()
        .map_err(|e| e.to_string())?;

    let menu = MenuBuilder::new(app).item(&file_menu).build().map_err(|e| e.to_string())?;

    if let Some(window) = app.get_webview_window("main") {
        window.set_menu(menu).map_err(|e| e.to_string())?;
    }

    let handle = app.clone();
    app.on_menu_event(move |app, event| {
        let state = app.state::<Arc<AppState>>().inner().clone();
        match event.id().as_ref() {
            "start_restart" => {
                let app2 = handle.clone();
                tauri::async_runtime::spawn(async move {
                    let _ = process::start_or_restart(app2, state).await;
                });
            }
            "reload_page" => {
                let _ = windows::reload_main_page(app);
            }
            "update_comfyui" => {
                let app2 = app.clone();
                tauri::async_runtime::spawn(async move {
                    let config = state.config.lock().unwrap().clone();
                    let (kind, message) = match nodes::update_comfyui(&config).await {
                        Ok(result) if result.success => (MessageDialogKind::Info, result.message),
                        Ok(result) => (MessageDialogKind::Error, result.message),
                        Err(e) => (MessageDialogKind::Error, e),
                    };
                    app2.dialog()
                        .message(message)
                        .title("更新 ComfyUI")
                        .kind(kind)
                        .show(|_| {});
                });
            }
            "manage_nodes" => {
                let _ = windows::open_nodes_window(app);
            }
            "open_comfyui_dir" => {
                let _ = windows::open_folder(app, &state, "comfyui");
            }
            "open_nodes_dir" => {
                let _ = windows::open_folder(app, &state, "custom_nodes");
            }
            "open_output_dir" => {
                let _ = windows::open_folder(app, &state, "output");
            }
            "open_models_dir" => {
                let _ = windows::open_folder(app, &state, "models");
            }
            "settings" => {
                let _ = windows::open_settings_window(app);
            }
            _ => {}
        }
    });

    Ok(())
}
