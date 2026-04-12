use anyhow::Result;
use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn info(message: impl AsRef<str>) {
    let _ = append("INFO", message.as_ref());
}

pub fn warn(message: impl AsRef<str>) {
    let _ = append("WARN", message.as_ref());
}

pub fn error(message: impl AsRef<str>) {
    let _ = append("ERROR", message.as_ref());
}

fn append(level: &str, message: &str) -> Result<()> {
    let path = active_log_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(file, "[{}] {:<5} {}", timestamp(), level, message)?;
    Ok(())
}

fn active_log_path() -> Result<PathBuf> {
    if let Ok(path) = env::var("MEDIUM_LOG_PATH") {
        let trimmed = path.trim();
        if !trimmed.is_empty() {
            return Ok(PathBuf::from(trimmed));
        }
    }

    #[cfg(test)]
    {
        return Ok(env::temp_dir().join("medium-test-daemon.log"));
    }

    #[cfg(not(test))]
    {
        crate::config::log_file_path()
    }
}

fn timestamp() -> String {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => format!(
            "{}.{}",
            duration.as_secs(),
            format!("{:03}", duration.subsec_millis())
        ),
        Err(_) => "0.000".to_string(),
    }
}
