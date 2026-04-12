use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use std::time::Duration;
use tauri_app_lib::config::{log_file_path, resolve_config_path};
use tauri_app_lib::ipc::{get_socket_paths, DEFAULT_DAEMON_INSTANCE};
use tauri_app_lib::protocol::{Command, Event, RoutedCommand};
use tokio::net::UnixStream;
use tokio_util::codec::{Framed, LinesCodec};

pub async fn run() -> Result<()> {
    let resolved_config = resolve_config_path()?;
    let log_path = log_file_path()?;
    let (cmd_socket, _) = get_socket_paths(DEFAULT_DAEMON_INSTANCE);

    println!("Medium Status\n");
    println!(
        "Config: {:?} ({})",
        resolved_config.path, resolved_config.source
    );
    println!("Logs:   {:?}", log_path);
    println!("Socket: {:?}", cmd_socket);

    if !cmd_socket.exists() {
        println!("Daemon: stopped");
        return Ok(());
    }

    let stream = match UnixStream::connect(&cmd_socket).await {
        Ok(stream) => stream,
        Err(_) => {
            println!("Daemon: socket present but unresponsive");
            return Ok(());
        }
    };

    let mut framed = Framed::new(stream, LinesCodec::new());
    let cmd = RoutedCommand {
        ghost: "default".to_string(),
        command: Command::Status,
    };

    framed.send(serde_json::to_string(&cmd)?).await?;

    match tokio::time::timeout(Duration::from_secs(2), async {
        while let Some(Ok(line)) = framed.next().await {
            if let Ok(Event::Status {
                active_ghost,
                known_ghosts,
            }) = serde_json::from_str::<Event>(&line)
            {
                return Some((active_ghost, known_ghosts));
            }
        }
        None
    })
    .await
    {
        Ok(Some((active_ghost, known_ghosts))) => {
            println!("Daemon: running");
            println!("Instance: {}", DEFAULT_DAEMON_INSTANCE);
            if known_ghosts.is_empty() {
                println!("Ghosts: none");
            } else {
                let formatted = format_ghosts(&active_ghost, known_ghosts);
                println!("Ghosts: {}", formatted);
            }
        }
        Ok(None) => {
            println!("Daemon: running but closed the connection before replying");
        }
        Err(_) => {
            println!("Daemon: running but timed out during status request");
        }
    }

    Ok(())
}

fn format_ghosts(active_ghost: &str, known_ghosts: Vec<String>) -> String {
    known_ghosts
        .into_iter()
        .map(|ghost| {
            if ghost == active_ghost {
                format!("{} *", ghost)
            } else {
                ghost
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::format_ghosts;

    #[test]
    fn marks_active_ghost_in_output() {
        let formatted = format_ghosts(
            "archer",
            vec![
                "pawn".to_string(),
                "archer".to_string(),
                "warrior".to_string(),
            ],
        );

        assert_eq!(formatted, "pawn, archer *, warrior");
    }
}
