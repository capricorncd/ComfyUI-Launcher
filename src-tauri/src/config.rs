use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub root_path: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            root_path: String::new(),
        }
    }
}

impl Config {
    pub fn python_exe(&self) -> PathBuf {
        Path::new(&self.root_path).join("python_embeded").join("python.exe")
    }

    pub fn main_py(&self) -> PathBuf {
        Path::new(&self.root_path).join("ComfyUI").join("main.py")
    }

    pub fn custom_nodes_dir(&self) -> PathBuf {
        Path::new(&self.root_path).join("ComfyUI").join("custom_nodes")
    }

    pub fn output_dir(&self) -> PathBuf {
        Path::new(&self.root_path).join("ComfyUI").join("output")
    }

    pub fn models_dir(&self) -> PathBuf {
        Path::new(&self.root_path).join("ComfyUI").join("models")
    }

    /// Returns an error message describing why this root path doesn't look
    /// like a valid ComfyUI portable install, if it doesn't.
    pub fn validate(&self) -> Result<(), String> {
        if self.root_path.trim().is_empty() {
            return Err("尚未设置 ComfyUI 安装目录，请在设置中指定".to_string());
        }
        if !self.python_exe().is_file() {
            return Err(format!(
                "未找到 {}，请确认路径下存在 python_embeded\\python.exe",
                self.python_exe().display()
            ));
        }
        if !self.main_py().is_file() {
            return Err(format!(
                "未找到 {}，请确认路径下存在 ComfyUI\\main.py",
                self.main_py().display()
            ));
        }
        Ok(())
    }
}

fn config_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("无法获取配置目录: {e}"))?;
    Ok(dir.join("config.json"))
}

pub fn load(app: &AppHandle) -> Config {
    let path = match config_path(app) {
        Ok(p) => p,
        Err(_) => return Config::default(),
    };
    match std::fs::read_to_string(&path) {
        Ok(text) => serde_json::from_str(&text).unwrap_or_default(),
        Err(_) => Config::default(),
    }
}

pub fn save(app: &AppHandle, config: &Config) -> Result<(), String> {
    let path = config_path(app)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("无法创建配置目录: {e}"))?;
    }
    let text = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    std::fs::write(&path, text).map_err(|e| format!("无法写入配置文件: {e}"))
}
