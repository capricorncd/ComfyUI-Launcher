use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command as TokioCommand};
use url::Url;

use crate::state::{AppState, ComfyStatus};
use crate::windows;

const COMFY_URL: &str = "http://127.0.0.1:8188/";

/// Appends `dir` to the current process's PATH, mirroring what
/// `run_nvidia_gpu.bat` does (`set PATH=%PATH%;%~dp0ComfyUI\sox`).
fn append_to_path(dir: &std::path::Path) -> std::ffi::OsString {
    match std::env::var_os("PATH") {
        Some(existing) => {
            let mut path = existing;
            path.push(";");
            path.push(dir);
            path
        }
        None => dir.as_os_str().to_owned(),
    }
}

/// A real HTTP request, not just a TCP connect: on this kind of dev machine
/// (Hyper-V/WSL2/Docker virtual switches in play) a bare `TcpStream::connect`
/// to a closed loopback port can take several seconds to actually refuse,
/// and can even report a spurious success at the virtual-adapter level
/// before a real refusal propagates back. Only a genuine HTTP response
/// proves ComfyUI (not just "something") is listening.
async fn is_comfy_responding(url: &str) -> bool {
    let Ok(client) = reqwest::Client::builder().timeout(Duration::from_secs(2)).build() else {
        return false;
    };
    matches!(client.get(url).send().await, Ok(resp) if resp.status().is_success())
}

async fn kill_process_tree(pid: u32) {
    let _ = TokioCommand::new("taskkill")
        .args(["/PID", &pid.to_string(), "/T", "/F"])
        .output()
        .await;
    tokio::time::sleep(Duration::from_millis(200)).await;
}

/// Stops the ComfyUI process tree during application shutdown. This is
/// intentionally synchronous because the async runtime may already be
/// shutting down when Tauri delivers `ExitRequested`.
pub fn stop_tracked_process(state: &Arc<AppState>) {
    let pid = state.comfy.lock().unwrap().pid.take();
    if let Some(pid) = pid {
        let _ = std::process::Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .output();
    }
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

    let (my_gen, is_first_start) = {
        let mut comfy = state.comfy.lock().unwrap();
        let is_first_start = comfy.generation == 0;
        comfy.generation += 1;
        comfy.log.clear();
        (comfy.generation, is_first_start)
    };

    // The main window is already loading (or has already loaded) our
    // index.html as part of its own creation — re-navigating to that exact
    // URL here would race with that in-flight initial load and can leave
    // WebView2 showing a blank page. Only navigate "home" for an actual
    // restart, when the window may currently be showing the ComfyUI page.
    if !is_first_start {
        navigate_home(&app, &state);
    }
    set_status(&app, &state, ComfyStatus::Starting);

    // Something is already listening on the port and it isn't a process we
    // just killed above — most likely ComfyUI was started outside this app
    // (e.g. via run_nvidia_gpu.bat), or a previous launch of this app was
    // force-closed and orphaned its ComfyUI child. Spawning our own on top
    // would either fail to bind or race with the existing one, and either
    // way leaves a duplicate process running invisibly in the background.
    // Adopt the existing server instead of spawning a redundant copy.
    if is_comfy_responding(COMFY_URL).await {
        if state.comfy.lock().unwrap().generation == my_gen {
            state.comfy.lock().unwrap().pid = None;
            set_status(&app, &state, ComfyStatus::Ready { url: COMFY_URL.to_string() });
            navigate_to_comfy(&app);
        }
        return Ok(());
    }

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
        .arg("--disable-auto-launch")
        .arg("--disable-api-nodes")
        .env("PATH", append_to_path(&config.sox_dir()))
        // The launcher has no console, so Python otherwise inherits the
        // machine's legacy ANSI code page (for example cp932 on Japanese
        // Windows). Custom nodes commonly log Unicode symbols and Chinese
        // text; force UTF-8 so logging cannot crash ComfyUI during startup.
        .env("PYTHONUTF8", "1")
        .env("PYTHONIOENCODING", "utf-8")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);
    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        command.creation_flags(CREATE_NO_WINDOW);
    }

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
        // Phase 1: wait for ComfyUI to either start responding or exit early.
        // `child` must stay alive (not be dropped) for as long as we want
        // ComfyUI to keep running — with `kill_on_drop(true)`, dropping it
        // terminates the process, so it's held here for the process's whole
        // lifetime rather than being dropped the moment we reach Ready.
        loop {
            tokio::select! {
                biased;
                exit = child.wait() => {
                    if state2.comfy.lock().unwrap().generation != my_gen {
                        return; // superseded by a newer restart
                    }
                    let code = exit.map(|s| s.code().unwrap_or(-1)).unwrap_or(-1);
                    let log_tail = {
                        let mut comfy = state2.comfy.lock().unwrap();
                        comfy.pid = None;
                        comfy.log_tail()
                    };
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
                    if is_comfy_responding(COMFY_URL).await {
                        if state2.comfy.lock().unwrap().generation != my_gen {
                            return;
                        }
                        set_status(&app2, &state2, ComfyStatus::Ready { url: COMFY_URL.to_string() });
                        navigate_to_comfy(&app2);
                        break;
                    }
                }
            }
        }

        // Phase 2: ComfyUI is up and the window has navigated to it. Keep
        // holding `child` and watch for it exiting unexpectedly (crash, or
        // killed by taskkill from an explicit restart elsewhere).
        let exit = child.wait().await;
        if state2.comfy.lock().unwrap().generation == my_gen {
            let code = exit.map(|s| s.code().unwrap_or(-1)).unwrap_or(-1);
            let log_tail = {
                let mut comfy = state2.comfy.lock().unwrap();
                comfy.pid = None;
                comfy.log_tail()
            };
            set_status(&app2, &state2, ComfyStatus::Failed {
                message: format!("ComfyUI 进程已退出（退出码 {code}）"),
                log_tail,
            });
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
