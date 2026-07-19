use std::sync::Arc;
use tauri::menu::{MenuBuilder, MenuItemBuilder, SubmenuBuilder};
use tauri::{AppHandle, Manager};

use crate::state::AppState;
use crate::{process, windows};

pub fn build_and_attach(app: &AppHandle) -> Result<(), String> {
    let start_restart = MenuItemBuilder::with_id("start_restart", "启动/重启").build(app).map_err(|e| e.to_string())?;
    let reload_page = MenuItemBuilder::with_id("reload_page", "刷新页面").build(app).map_err(|e| e.to_string())?;
    let manage_nodes = MenuItemBuilder::with_id("manage_nodes", "自定义节点管理").build(app).map_err(|e| e.to_string())?;
    let open_nodes_dir = MenuItemBuilder::with_id("open_nodes_dir", "打开自定义节点目录").build(app).map_err(|e| e.to_string())?;
    let open_output_dir = MenuItemBuilder::with_id("open_output_dir", "打开输出目录").build(app).map_err(|e| e.to_string())?;
    let open_models_dir = MenuItemBuilder::with_id("open_models_dir", "打开模型目录").build(app).map_err(|e| e.to_string())?;
    let settings = MenuItemBuilder::with_id("settings", "设置").build(app).map_err(|e| e.to_string())?;

    let file_menu = SubmenuBuilder::new(app, "File")
        .item(&start_restart)
        .item(&reload_page)
        .separator()
        .item(&manage_nodes)
        .item(&open_nodes_dir)
        .item(&open_output_dir)
        .item(&open_models_dir)
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
            "manage_nodes" => {
                let _ = windows::open_nodes_window(app);
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
