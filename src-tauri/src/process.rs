use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command as TokioCommand};
use url::Url;

use crate::state::{AppState, ComfyStatus};
use crate::windows;

const COMFY_ADDR: &str = "127.0.0.1:8188";
const COMFY_URL: &str = "http://127.0.0.1:8188/";

async fn is_port_open(addr: &str) -> bool {
    tokio::time::timeout(Duration::from_millis(300), tokio::net::TcpStream::connect(addr))
        .await
        .map(|r| r.is_ok())
        .unwrap_or(false)
}

async fn kill_process_tree(pid: u32) {
    let _ = TokioCommand::new("taskkill")
        .args(["/PID", &pid.to_string(), "/T", "/F"])
        .output()
        .await;
    tokio::time::sleep(Duration::from_millis(200)).await;
}

fn set_status(app: &AppHandle, state: &Arc<AppState>, status: ComfyStatus) {
    state.comfy.lock().unwrap().status = status.clone();
    let _ = app.emit("comfy-status", status);
}

fn navigate_home(app: &AppHandle, state: &Arc<AppState>) {
    if let Some(window) = app.get_webview_window("main") {
        if let Some(home) = state.home_url.lock().unwrap().clone() {
            let _ = window.navigate(home);
        }
    }
}

fn navigate_to_comfy(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        if let Ok(url) = Url::parse(COMFY_URL) {
            let _ = window.navigate(url);
        }
    }
}

/// Kills any tracked ComfyUI process, spawns a fresh one, and drives it
/// through Starting -> Ready/Failed, navigating the main window accordingly.
pub async fn start_or_restart(app: AppHandle, state: Arc<AppState>) -> Result<(), String> {
    let _guard = state.start_lock.lock().await;

    let old_pid = state.comfy.lock().unwrap().pid.take();
    if let Some(pid) = old_pid {
        kill_process_tree(pid).await;
    }

    let my_gen = {
        let mut comfy = state.comfy.lock().unwrap();
        comfy.generation += 1;
        comfy.log.clear();
        comfy.generation
    };

    navigate_home(&app, &state);
    set_status(&app, &state, ComfyStatus::Starting);

    let config = state.config.lock().unwrap().clone();
    if let Err(msg) = config.validate() {
        set_status(
            &app,
            &state,
            ComfyStatus::Failed {
                message: format!("尚未正确配置 ComfyUI 目录：{msg}"),
                log_tail: vec![],
            },
        );
        let _ = windows::open_settings_window(&app);
        return Ok(());
    }

    let mut command = TokioCommand::new(config.python_exe());
    command
        .current_dir(&config.root_path)
        .arg("-s")
        .arg(config.main_py())
        .arg("--windows-standalone-build")
        .arg("--disable-api-nodes")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);

    let mut child: Child = match command.spawn() {
        Ok(c) => c,
        Err(e) => {
            set_status(
                &app,
                &state,
                ComfyStatus::Failed {
                    message: format!("无法启动 ComfyUI 进程: {e}"),
                    log_tail: vec![],
                },
            );
            return Ok(());
        }
    };

    let pid = child.id();
    state.comfy.lock().unwrap().pid = pid;

    if let Some(stdout) = child.stdout.take() {
        spawn_log_reader(app.clone(), state.clone(), my_gen, stdout);
    }
    if let Some(stderr) = child.stderr.take() {
        spawn_log_reader(app.clone(), state.clone(), my_gen, stderr);
    }

    let app2 = app.clone();
    let state2 = state.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::select! {
                biased;
                exit = child.wait() => {
                    if state2.comfy.lock().unwrap().generation != my_gen {
                        return; // superseded by a newer restart
                    }
                    let code = exit.map(|s| s.code().unwrap_or(-1)).unwrap_or(-1);
                    let log_tail = state2.comfy.lock().unwrap().log_tail();
                    set_status(&app2, &state2, ComfyStatus::Failed {
                        message: format!("ComfyUI 进程已退出（退出码 {code}）"),
                        log_tail,
                    });
                    return;
                }
                _ = tokio::time::sleep(Duration::from_millis(500)) => {
                    if state2.comfy.lock().unwrap().generation != my_gen {
                        return; // superseded by a newer restart
                    }
                    if is_port_open(COMFY_ADDR).await {
                        if state2.comfy.lock().unwrap().generation != my_gen {
                            return;
                        }
                        set_status(&app2, &state2, ComfyStatus::Ready { url: COMFY_URL.to_string() });
                        navigate_to_comfy(&app2);
                        return;
                    }
                }
            }
        }
    });

    Ok(())
}

fn spawn_log_reader<R>(app: AppHandle, state: Arc<AppState>, my_gen: u64, reader: R)
where
    R: tokio::io::AsyncRead + Unpin + Send + 'static,
{
    tauri::async_runtime::spawn(async move {
        let mut lines = BufReader::new(reader).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let mut comfy = state.comfy.lock().unwrap();
            if comfy.generation != my_gen {
                return; // superseded by a newer restart
            }
            comfy.push_log(line.clone());
            drop(comfy);
            let _ = app.emit("comfy-log", line);
        }
    });
}

pub fn get_status(state: &AppState) -> ComfyStatus {
    state.comfy.lock().unwrap().status.clone()
}
