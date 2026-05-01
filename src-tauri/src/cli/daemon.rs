use anyhow::{Context, Result};
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::process::Command as StdCommand;
use std::process::Stdio;
use tauri_app_lib::config::log_file_path;
use tauri_app_lib::ipc::{get_socket_paths, DEFAULT_DAEMON_INSTANCE};
use tauri_app_lib::protocol::{Command, RoutedCommand};

pub fn ensure_running(perform_check: bool) -> Result<()> {
    if !perform_check {
        return Ok(());
    }

    let (cmd_socket, _) = get_socket_paths(DEFAULT_DAEMON_INSTANCE);
    if daemon_is_responsive(&cmd_socket) {
        return Ok(());
    }

    if cmd_socket.exists() {
        println!("Medium daemon socket is stale. Re-summoning the Medium...");
        let _ = fs::remove_file(&cmd_socket);
    } else {
        println!("Medium daemon not detected. Summoning the Medium...");
    }

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
    while attempts < 10 {
        if daemon_is_responsive(&cmd_socket) {
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
        attempts += 1;
    }

    anyhow::bail!("Medium daemon failed to start within timeout.");
}

fn daemon_is_responsive(cmd_socket: &Path) -> bool {
    if !cmd_socket.exists() {
        return false;
    }

    let mut stream = match UnixStream::connect(cmd_socket) {
        Ok(stream) => stream,
        Err(_) => return false,
    };

    let _ = stream.set_read_timeout(Some(std::time::Duration::from_millis(800)));
    let _ = stream.set_write_timeout(Some(std::time::Duration::from_millis(800)));

    let status = RoutedCommand {
        ghost: "default".to_string(),
        command: Command::Status,
    };

    let line = match serde_json::to_string(&status) {
        Ok(value) => value,
        Err(_) => return false,
    };

    if stream
        .write_all(format!("{line}\n").as_bytes())
        .and_then(|_| stream.flush())
        .is_err()
    {
        return false;
    }

    let mut reader = BufReader::new(stream);
    let mut response = String::new();
    reader
        .read_line(&mut response)
        .map(|count| count > 0)
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::daemon_is_responsive;
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::net::UnixListener;
    use std::thread;
    use tempfile::tempdir;

    #[test]
    fn reports_false_when_socket_is_missing() {
        let tmp = tempdir().unwrap();
        let socket_path = tmp.path().join("missing.sock");
        assert!(!daemon_is_responsive(&socket_path));
    }

    #[test]
    fn reports_true_for_responsive_daemon_socket() {
        let tmp = tempdir().unwrap();
        let socket_path = tmp.path().join("responsive.sock");
        let listener = UnixListener::bind(&socket_path).unwrap();

        let handle = thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut request = String::new();
                let _ = reader.read_line(&mut request);
                let _ = writeln!(
                    stream,
                    "{{\"type\":\"status\",\"active_ghost\":\"vita\",\"known_ghosts\":[\"vita\"]}}"
                );
            }
        });

        assert!(daemon_is_responsive(&socket_path));
        handle.join().unwrap();
    }
}
