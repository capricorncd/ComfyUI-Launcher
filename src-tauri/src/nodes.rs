use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::process::Command as TokioCommand;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;

use crate::config::Config;

const MAX_CONCURRENT_GIT: usize = 8;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeInfo {
    pub name: String,
    pub has_git: bool,
    pub remote_url: Option<String>,
    pub version: String,
    pub last_update: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionResult {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CloneResult {
    pub success: bool,
    pub message: String,
    pub dir_name: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCheck {
    pub name: String,
    /// Short hash of the remote's HEAD, if it could be determined.
    pub remote_version: Option<String>,
    pub up_to_date: bool,
}

async fn run_git(dir: &Path, args: &[&str]) -> Result<String, String> {
    let output = TokioCommand::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .await
        .map_err(|e| format!("无法执行 git: {e}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

async fn describe_node(dir: PathBuf, name: String) -> NodeInfo {
    if !dir.join(".git").exists() {
        return NodeInfo {
            name,
            has_git: false,
            remote_url: None,
            version: "N/A".to_string(),
            last_update: None,
        };
    }

    let remote_url = run_git(&dir, &["remote", "get-url", "origin"]).await.ok();
    let version = run_git(&dir, &["describe", "--tags", "--always"])
        .await
        .unwrap_or_else(|_| "unknown".to_string());
    let last_line = run_git(&dir, &["log", "-1", "--format=%cI|%h|%s"]).await.ok();
    let last_update = last_line.and_then(|l| l.split('|').next().map(|s| s.to_string()));

    NodeInfo {
        name,
        has_git: true,
        remote_url,
        version,
        last_update,
    }
}

pub async fn list_custom_nodes(config: &Config) -> Result<Vec<NodeInfo>, String> {
    let dir = config.custom_nodes_dir();
    let entries = std::fs::read_dir(&dir)
        .map_err(|e| format!("无法读取自定义节点目录 {}: {e}", dir.display()))?;

    let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_GIT));
    let mut tasks: JoinSet<NodeInfo> = JoinSet::new();

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with("__") {
            continue;
        }
        let permit = semaphore.clone();
        tasks.spawn(async move {
            let _permit = permit.acquire_owned().await.unwrap();
            describe_node(path, name).await
        });
    }

    let mut results = Vec::new();
    while let Some(res) = tasks.join_next().await {
        if let Ok(info) = res {
            results.push(info);
        }
    }
    results.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(results)
}

/// Lightweight remote HEAD lookup (`git ls-remote`) — just a ref listing,
/// no objects are fetched, so this is cheap enough to run for every node.
async fn remote_head_hash(url: &str) -> Option<String> {
    let output = TokioCommand::new("git")
        .args(["ls-remote", url, "HEAD"])
        .output()
        .await
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let hash = text.split_whitespace().next()?;
    if hash.len() < 7 {
        return None;
    }
    Some(hash.to_string())
}

async fn check_node_update(dir: PathBuf, name: String) -> UpdateCheck {
    let remote_url = run_git(&dir, &["remote", "get-url", "origin"]).await.ok();
    let local_hash = run_git(&dir, &["rev-parse", "HEAD"]).await.ok();

    let (remote_version, up_to_date) = match (remote_url, local_hash) {
        (Some(url), Some(local)) => match remote_head_hash(&url).await {
            Some(remote) => {
                let up_to_date = remote == local;
                (Some(remote[..7.min(remote.len())].to_string()), up_to_date)
            }
            None => (None, false),
        },
        _ => (None, false),
    };

    UpdateCheck {
        name,
        remote_version,
        up_to_date,
    }
}

/// Checks every git-tracked custom node's remote HEAD against its local
/// HEAD. Runs after the initial (network-free) `list_custom_nodes` so the
/// node list can render immediately while this fills in "up to date" state.
pub async fn check_updates(config: &Config) -> Result<Vec<UpdateCheck>, String> {
    let dir = config.custom_nodes_dir();
    let entries = std::fs::read_dir(&dir)
        .map_err(|e| format!("无法读取自定义节点目录 {}: {e}", dir.display()))?;

    let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_GIT));
    let mut tasks: JoinSet<UpdateCheck> = JoinSet::new();

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() || !path.join(".git").exists() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with("__") {
            continue;
        }
        let permit = semaphore.clone();
        tasks.spawn(async move {
            let _permit = permit.acquire_owned().await.unwrap();
            check_node_update(path, name).await
        });
    }

    let mut results = Vec::new();
    while let Some(res) = tasks.join_next().await {
        if let Ok(check) = res {
            results.push(check);
        }
    }
    Ok(results)
}

pub async fn pull_node(config: &Config, name: &str) -> Result<ActionResult, String> {
    let dir = config.custom_nodes_dir().join(name);
    if !dir.join(".git").exists() {
        return Err(format!("{name} 不是一个 git 仓库"));
    }
    match run_git(&dir, &["pull", "--ff-only"]).await {
        Ok(out) => Ok(ActionResult {
            success: true,
            message: if out.is_empty() {
                "已是最新".to_string()
            } else {
                out
            },
        }),
        Err(err) => Ok(ActionResult {
            success: false,
            message: err,
        }),
    }
}

fn derive_dir_name(url: &str) -> String {
    let trimmed = url.trim().trim_end_matches('/');
    let last = trimmed.rsplit('/').next().unwrap_or(trimmed);
    last.trim_end_matches(".git").to_string()
}

pub async fn clone_node(config: &Config, url: &str) -> Result<CloneResult, String> {
    let dir_name = derive_dir_name(url);
    if dir_name.is_empty() {
        return Err("无法从该地址解析出仓库名称".to_string());
    }
    let base = config.custom_nodes_dir();
    let target = base.join(&dir_name);
    if target.exists() {
        return Ok(CloneResult {
            success: false,
            message: format!("目录 {dir_name} 已存在，请改用更新按钮"),
            dir_name,
        });
    }

    let output = TokioCommand::new("git")
        .arg("-C")
        .arg(&base)
        .arg("clone")
        .arg(url)
        .arg(&dir_name)
        .output()
        .await
        .map_err(|e| format!("无法执行 git: {e}"))?;

    if output.status.success() {
        Ok(CloneResult {
            success: true,
            message: "克隆完成".to_string(),
            dir_name,
        })
    } else {
        Ok(CloneResult {
            success: false,
            message: String::from_utf8_lossy(&output.stderr).trim().to_string(),
            dir_name,
        })
    }
}
