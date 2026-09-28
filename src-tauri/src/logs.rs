use chrono::{Days, NaiveDate, Utc};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::Mutex;

pub struct ProcessLog {
    directory: PathBuf,
    file: Mutex<(NaiveDate, File)>,
}

impl ProcessLog {
    pub fn new(directory: PathBuf) -> io::Result<Self> {
        fs::create_dir_all(&directory)?;
        let today = Utc::now().date_naive();
        let file = Self::open(&directory, today)?;
        let log = Self { directory, file: Mutex::new((today, file)) };
        log.cleanup(today)?;
        Ok(log)
    }

    fn open(directory: &std::path::Path, day: NaiveDate) -> io::Result<File> {
        OpenOptions::new().create(true).append(true)
            .open(directory.join(format!("comfyui-{day}.log")))
    }

    fn cleanup(&self, today: NaiveDate) -> io::Result<()> {
        let oldest = today - Days::new(6);
        for entry in fs::read_dir(&self.directory)? {
            let entry = entry?;
            let name = entry.file_name();
            let name = name.to_string_lossy();
            let Some(date) = name.strip_prefix("comfyui-").and_then(|s| s.strip_suffix(".log")) else { continue };
            let Ok(date) = NaiveDate::parse_from_str(date, "%Y-%m-%d") else { continue };
            if date < oldest && entry.file_type()?.is_file() {
                fs::remove_file(entry.path())?;
            }
        }
        Ok(())
    }

    pub fn write(&self, source: &str, text: &str) -> io::Result<()> {
        let now = Utc::now();
        let mut file = self.file.lock().unwrap();
        if file.0 != now.date_naive() {
            *file = (now.date_naive(), Self::open(&self.directory, now.date_naive())?);
            self.cleanup(now.date_naive())?;
        }
        writeln!(file.1, "[{}] [{}] {text}", now.to_rfc3339(), source)?;
        file.1.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retains_seven_days_and_appends_unicode() {
        let dir = std::env::temp_dir().join(format!("comfyui-log-test-{}-{}", std::process::id(), Utc::now().timestamp_nanos_opt().unwrap()));
        fs::create_dir_all(&dir).unwrap();
        let today = Utc::now().date_naive();
        let old = dir.join(format!("comfyui-{}.log", today - Days::new(7)));
        let recent = dir.join(format!("comfyui-{}.log", today - Days::new(6)));
        fs::write(&old, "old").unwrap();
        fs::write(&recent, "recent").unwrap();
        fs::write(dir.join("other.log"), "keep").unwrap();
        let log = ProcessLog::new(dir.clone()).unwrap();
        log.write("stdout", "音频修复").unwrap();
        log.write("launcher", "exit code -1").unwrap();
        assert!(!old.exists());
        assert!(recent.exists());
        assert!(dir.join("other.log").exists());
        let text = fs::read_to_string(dir.join(format!("comfyui-{today}.log"))).unwrap();
        assert!(text.contains("音频修复"));
        assert!(text.contains("exit code -1"));
        drop(log);
        let log = ProcessLog::new(dir.clone()).unwrap();
        log.write("launcher", "restarted").unwrap();
        let text = fs::read_to_string(dir.join(format!("comfyui-{today}.log"))).unwrap();
        assert!(text.contains("音频修复") && text.contains("restarted"));
        drop(log);
        fs::remove_dir_all(dir).unwrap();
    }
}
