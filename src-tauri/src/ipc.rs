use crate::logging;
use crate::protocol::{Event, RoutedCommand};
use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use std::path::PathBuf;
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::{broadcast, mpsc};
use tokio_util::codec::{Framed, LinesCodec};

pub const DEFAULT_DAEMON_INSTANCE: &str = "default";

pub fn get_socket_paths(instance_name: &str) -> (PathBuf, PathBuf) {
    let tmp_path = PathBuf::from("/tmp");
    let cmd = tmp_path.join(format!("medium_ghost_{}_cmd.sock", instance_name));
    let evt = tmp_path.join(format!("medium_ghost_{}_evt.sock", instance_name));
    (cmd, evt)
}

pub struct IpcServer {
    path: PathBuf,
}

impl IpcServer {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub async fn run(
        self,
        command_tx: mpsc::Sender<RoutedCommand>,
        event_rx: broadcast::Receiver<Event>,
    ) -> Result<()> {
        if self.path.exists() {
            tokio::fs::remove_file(&self.path).await?;
        }

        let listener = UnixListener::bind(&self.path)?;
        logging::info(format!("IPC server listening on {:?}", self.path));
        println!("IPC Server listening on {:?}", self.path);

        loop {
            let (stream, _) = listener.accept().await?;
            let command_tx = command_tx.clone();
            let event_rx_clone = event_rx.resubscribe();

            tokio::spawn(async move {
                if let Err(e) = handle_connection(stream, command_tx, event_rx_clone).await {
                    logging::error(format!("Error handling IPC connection: {}", e));
                    eprintln!("Error handling connection: {}", e);
                }
            });
        }
    }
}

async fn handle_connection(
    stream: UnixStream,
    command_tx: mpsc::Sender<RoutedCommand>,
    mut event_rx: broadcast::Receiver<Event>,
) -> Result<()> {
    let mut framed = Framed::new(stream, LinesCodec::new());

    loop {
        tokio::select! {
            result = framed.next() => {
                match result {
                    Some(Ok(line)) => {
                        let cmd: RoutedCommand = serde_json::from_str(&line)?;
                        command_tx.send(cmd).await?;
                    }
                    Some(Err(e)) => return Err(e.into()),
                    None => break, // Connection closed
                }
            }
            result = event_rx.recv() => {
                match result {
                    Ok(event) => {
                        let line = serde_json::to_string(&event)?;
                        framed.send(line).await?;
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => {
                        // Handle lagging if necessary
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::Command;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_ipc_communication() -> Result<()> {
        let dir = tempdir()?;
        let socket_path = dir.path().join("test.sock");

        let (cmd_tx, mut cmd_rx) = mpsc::channel(10);
        let (evt_tx, evt_rx) = broadcast::channel(10);

        let server_path = socket_path.clone();
        tokio::spawn(async move {
            let server = IpcServer::new(server_path);
            server.run(cmd_tx, evt_rx).await.unwrap();
        });

        // Give server a moment to start
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let stream = UnixStream::connect(&socket_path).await?;
        let mut framed = Framed::new(stream, LinesCodec::new());

        // Test sending a command
        let cmd = RoutedCommand {
            ghost: "pawn".to_string(),
            command: Command::Ping,
        };
        framed.send(serde_json::to_string(&cmd)?).await?;

        let received_cmd = cmd_rx.recv().await.unwrap();
        match received_cmd.command {
            Command::Ping => {}
            _ => panic!("Expected Ping command"),
        }
        assert_eq!(received_cmd.ghost, "pawn");

        // Test receiving an event
        let event = Event::Pong;
        evt_tx.send(event)?;

        let received_line = framed.next().await.unwrap()?;
        let received_event: Event = serde_json::from_str(&received_line)?;
        match received_event {
            Event::Pong => {}
            _ => panic!("Expected Pong event"),
        }

        Ok(())
    }
}
