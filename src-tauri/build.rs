fn main() {
    let build_time = build_time();
    println!("cargo:rustc-env=APP_BUILD_TIME={build_time}");
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "get_status", "start_or_restart", "get_config", "save_config",
            "list_custom_nodes", "pull_node", "clone_node", "check_node_updates",
            "open_folder", "launcher_pick_directory",
        ]),
    )).expect("failed to build launcher permissions");
}

fn build_time() -> String {
    #[cfg(windows)]
    let output = std::process::Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-Command",
            "(Get-Date).ToString('yyyy-MM-dd HH:mm:ss')",
        ])
        .output();

    #[cfg(not(windows))]
    let output = std::process::Command::new("date")
        .arg("+%Y-%m-%d %H:%M:%S")
        .output();

    output
        .ok()
        .filter(|result| result.status.success())
        .and_then(|result| String::from_utf8(result.stdout).ok())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "unknown".to_string())
}
