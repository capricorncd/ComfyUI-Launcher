use std::sync::Arc;
use tauri::{AppHandle, State};

use crate::config::{self, Config};
use crate::nodes::{self, ActionResult, CloneResult, NodeInfo, UpdateCheck};
use crate::state::{AppState, ComfyStatus};
use crate::{process, windows};

#[tauri::command]
pub fn get_status(state: State<Arc<AppState>>) -> ComfyStatus {
    process::get_status(&state)
}

#[tauri::command]
pub fn get_log_tail(state: State<Arc<AppState>>) -> Vec<String> {
    state.comfy.lock().unwrap().log_tail()
}

#[tauri::command]
pub async fn start_or_restart(app: AppHandle, state: State<'_, Arc<AppState>>) -> Result<(), String> {
    process::start_or_restart(app, state.inner().clone()).await
}

#[tauri::command]
pub fn get_config(state: State<Arc<AppState>>) -> Config {
    state.config.lock().unwrap().clone()
}

#[tauri::command]
pub fn save_config(app: AppHandle, state: State<Arc<AppState>>, root_path: String) -> Result<(), String> {
    let candidate = Config { root_path };
    candidate.validate()?;
    config::save(&app, &candidate)?;
    *state.config.lock().unwrap() = candidate;
    Ok(())
}

#[tauri::command]
pub async fn list_custom_nodes(state: State<'_, Arc<AppState>>) -> Result<Vec<NodeInfo>, String> {
    let config = state.config.lock().unwrap().clone();
    nodes::list_custom_nodes(&config).await
}

#[tauri::command]
pub async fn pull_node(state: State<'_, Arc<AppState>>, name: String) -> Result<ActionResult, String> {
    let config = state.config.lock().unwrap().clone();
    nodes::pull_node(&config, &name).await
}

#[tauri::command]
pub async fn clone_node(state: State<'_, Arc<AppState>>, url: String) -> Result<CloneResult, String> {
    let config = state.config.lock().unwrap().clone();
    nodes::clone_node(&config, &url).await
}

#[tauri::command]
pub async fn check_node_updates(state: State<'_, Arc<AppState>>) -> Result<Vec<UpdateCheck>, String> {
    let config = state.config.lock().unwrap().clone();
    nodes::check_updates(&config).await
}

#[tauri::command]
pub fn open_folder(app: AppHandle, state: State<Arc<AppState>>, kind: String) -> Result<(), String> {
    windows::open_folder(&app, &state, &kind)
}
