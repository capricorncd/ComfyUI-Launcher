fn main() {
    let build_time = build_time();
    println!("cargo:rustc-env=APP_BUILD_TIME={build_time}");
    tauri_build::build()
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
