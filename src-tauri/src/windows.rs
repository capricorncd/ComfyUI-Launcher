use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_opener::OpenerExt;

use crate::state::AppState;

fn open_or_focus(
    app: &AppHandle,
    label: &str,
    url_path: &str,
    title: &str,
    size: (f64, f64),
    center: bool,
) -> Result<(), String> {
    if let Some(win) = app.get_webview_window(label) {
        let _ = win.set_focus();
        return Ok(());
    }
    let mut builder = WebviewWindowBuilder::new(app, label, WebviewUrl::App(url_path.into()))
        .title(title)
        .inner_size(size.0, size.1);
    if center {
        builder = builder.center();
    }
    builder.build().map_err(|e| e.to_string())?;
    Ok(())
}

const DIALOG_WIDTH_PCT: f64 = 0.4;
const DIALOG_HEIGHT_PCT: f64 = 0.45;
const DIALOG_MAX_WIDTH: f64 = 1200.0;
const DIALOG_FALLBACK_SIZE: (f64, f64) = (720.0, 480.0);

/// A fixed percentage of the primary monitor's logical size, capped at
/// `DIALOG_MAX_WIDTH` so it doesn't balloon on very large/4K displays.
/// Shared by the nodes and settings dialogs so they're sized consistently.
fn dialog_window_size(app: &AppHandle) -> (f64, f64) {
    let Some(main) = app.get_webview_window("main") else {
        return DIALOG_FALLBACK_SIZE;
    };
    let Ok(Some(monitor)) = main.primary_monitor() else {
        return DIALOG_FALLBACK_SIZE;
    };
    let scale = monitor.scale_factor();
    let logical_width = monitor.size().width as f64 / scale;
    let logical_height = monitor.size().height as f64 / scale;
    let width = (logical_width * DIALOG_WIDTH_PCT).min(DIALOG_MAX_WIDTH);
    let height = logical_height * DIALOG_HEIGHT_PCT;
    (width, height)
}

pub fn open_nodes_window(app: &AppHandle) -> Result<(), String> {
    let size = dialog_window_size(app);
    open_or_focus(app, "nodes", "nodes.html", "自定义节点管理", size, true)
}

pub fn open_settings_window(app: &AppHandle) -> Result<(), String> {
    let size = dialog_window_size(app);
    open_or_focus(app, "settings", "settings.html", "设置", size, true)
}

/// Reloads whatever page is currently loaded in the main window (the
/// ComfyUI UI, or the loading screen) without touching the ComfyUI process
/// — for when only a custom node's JS/CSS changed and a full restart isn't
/// needed.
pub fn reload_main_page(app: &AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "主窗口不存在".to_string())?;
    window
        .eval("window.location.reload();")
        .map_err(|e| e.to_string())
}

pub fn open_folder(app: &AppHandle, state: &AppState, kind: &str) -> Result<(), String> {
    let config = state.config.lock().unwrap().clone();
    let path = match kind {
        "custom_nodes" => config.custom_nodes_dir(),
        "output" => config.output_dir(),
        "models" => config.models_dir(),
        other => return Err(format!("未知的目录类型: {other}")),
    };
    if !path.is_dir() {
        return Err(format!("目录不存在: {}", path.display()));
    }
    app.opener()
        .open_path(path.to_string_lossy().to_string(), None::<&str>)
        .map_err(|e| e.to_string())
}
