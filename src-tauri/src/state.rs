use serde::Serialize;
use std::collections::VecDeque;
use std::sync::Mutex;
use tokio::sync::Mutex as AsyncMutex;
use url::Url;

use crate::config::Config;

const LOG_CAPACITY: usize = 500;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(tag = "state", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum ComfyStatus {
    Stopped,
    Starting,
    Ready { url: String },
    Failed { message: String, log_tail: Vec<String> },
}

pub struct ComfyProcess {
    pub pid: Option<u32>,
    pub status: ComfyStatus,
    pub log: VecDeque<String>,
    /// Bumped every time start_or_restart begins, so a stale readiness-poll
    /// loop from a previous run can notice it's been superseded and exit.
    pub generation: u64,
}

impl ComfyProcess {
    fn new() -> Self {
        Self {
            pid: None,
            status: ComfyStatus::Stopped,
            log: VecDeque::with_capacity(LOG_CAPACITY),
            generation: 0,
        }
    }

    pub fn push_log(&mut self, line: String) {
        if self.log.len() >= LOG_CAPACITY {
            self.log.pop_front();
        }
        self.log.push_back(line);
    }

    pub fn log_tail(&self) -> Vec<String> {
        self.log.iter().cloned().collect()
    }
}

pub struct AppState {
    pub config: Mutex<Config>,
    pub comfy: Mutex<ComfyProcess>,
    /// Serializes start/restart sequences so overlapping menu clicks can't race.
    pub start_lock: AsyncMutex<()>,
    /// URL the main window's `index.html` loading page was first served from
    /// (differs between `tauri dev` and a bundled build); reused to navigate
    /// back to our own UI on restart.
    pub home_url: Mutex<Option<Url>>,
}

impl AppState {
    pub fn new(config: Config) -> Self {
        Self {
            config: Mutex::new(config),
            comfy: Mutex::new(ComfyProcess::new()),
            start_lock: AsyncMutex::new(()),
            home_url: Mutex::new(None),
        }
    }
}
