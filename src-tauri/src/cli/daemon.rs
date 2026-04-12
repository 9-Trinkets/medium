use anyhow::{Context, Result};
use std::fs::{self, OpenOptions};
use std::process::Command as StdCommand;
use std::process::Stdio;
use tauri_app_lib::config::log_file_path;
use tauri_app_lib::ipc::{get_socket_paths, DEFAULT_DAEMON_INSTANCE};

pub fn ensure_running(perform_check: bool) -> Result<()> {
    if !perform_check {
        return Ok(());
    }

    let (cmd_socket, _) = get_socket_paths(DEFAULT_DAEMON_INSTANCE);

    if !cmd_socket.exists() {
        println!("Medium daemon not detected. Summoning the Medium...");

        let exe_path = std::env::current_exe()?;
        let log_path = log_file_path()?;

        if let Some(parent) = log_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let log_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .context("Could not open daemon log file")?;

        StdCommand::new(exe_path)
            .arg("daemon")
            .stdout(Stdio::from(log_file.try_clone()?))
            .stderr(Stdio::from(log_file))
            .spawn()
            .context("Failed to spawn Medium daemon")?;

        let mut attempts = 0;
        while !cmd_socket.exists() && attempts < 10 {
            std::thread::sleep(std::time::Duration::from_millis(200));
            attempts += 1;
        }

        if !cmd_socket.exists() {
            anyhow::bail!("Medium daemon failed to start within timeout.");
        }
    }
    Ok(())
}
