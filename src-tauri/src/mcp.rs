use crate::ipc::{get_socket_paths, DEFAULT_DAEMON_INSTANCE};
use crate::protocol::{Command, RoutedCommand};
use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use rmcp::{
    model::{
        CallToolRequestParams, CallToolResult, Content, ErrorCode, Implementation,
        InitializeResult, ListToolsResult, PaginatedRequestParams, ServerCapabilities, ServerInfo,
        Tool,
    },
    service::{MaybeSendFuture, RequestContext, RoleServer},
    ErrorData as McpError, ServerHandler, ServiceExt,
};
use schemars::JsonSchema;
use serde::Deserialize;
use std::future::Future;
use std::fs::{self, OpenOptions};
use std::path::Path;
use std::process::{Command as StdCommand, Stdio};
use std::time::Duration;
use tokio::net::UnixStream;
use tokio_util::codec::{Framed, LinesCodec};

pub struct MediumMcpServer {
    pub default_ghost: String,
}

impl MediumMcpServer {
    pub fn new(default_ghost: String) -> Self {
        Self { default_ghost }
    }

    pub async fn run(self) -> Result<()> {
        send_command(
            &self.default_ghost,
            Command::SwitchGhost {
                name: self.default_ghost.clone(),
            },
        )
        .await?;
        let transport = (tokio::io::stdin(), tokio::io::stdout());
        let server = self.serve(transport).await?;
        server.waiting().await?;
        Ok(())
    }

}

#[derive(Deserialize, JsonSchema)]
struct SummonArgs {
    /// The ghost persona to summon (e.g. 'vita' or an imported ghost name)
    name: String,
}

#[derive(Deserialize, JsonSchema)]
struct DismissArgs {
    /// The ghost persona to dismiss
    name: String,
}

#[derive(Deserialize, JsonSchema)]
struct SpeakArgs {
    /// The text to speak
    text: String,
    /// Optional ghost name (uses default if omitted)
    #[serde(default)]
    ghost: Option<String>,
    /// Whether voice output is enabled; false keeps the speech bubble without TTS
    #[serde(default)]
    voice: Option<bool>,
}

#[derive(Deserialize, JsonSchema)]
struct PlayAnimationArgs {
    /// Animation name to play
    name: String,
    /// Whether to loop the animation
    #[serde(default)]
    loop_anim: bool,
    /// Optional ghost name (uses default if omitted)
    #[serde(default)]
    ghost: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
struct SetFacingArgs {
    /// Direction to face: 'left' or 'right'
    direction: String,
    /// Optional ghost name (uses default if omitted)
    #[serde(default)]
    ghost: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
struct ListGhostsArgs {}

impl ServerHandler for MediumMcpServer {
    fn get_info(&self) -> ServerInfo {
        let mut capabilities = ServerCapabilities::default();
        capabilities.tools = Some(Default::default());
        InitializeResult::new(capabilities).with_server_info(Implementation::new("medium", "0.1.0"))
    }

    fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<ListToolsResult, McpError>> + MaybeSendFuture + '_ {
        async move {
            Ok(ListToolsResult::with_all_items(vec![
                Tool::new(
                    "summon",
                    "Summon a ghost persona into existence",
                    rmcp::handler::server::tool::schema_for_type::<SummonArgs>(),
                ),
                Tool::new(
                    "dismiss",
                    "Dismiss a ghost persona",
                    rmcp::handler::server::tool::schema_for_type::<DismissArgs>(),
                ),
                Tool::new(
                    "speak",
                    "Make the ghost speak",
                    rmcp::handler::server::tool::schema_for_type::<SpeakArgs>(),
                ),
                Tool::new(
                    "play_animation",
                    "Trigger an animation on the ghost",
                    rmcp::handler::server::tool::schema_for_type::<PlayAnimationArgs>(),
                ),
                Tool::new(
                    "set_facing",
                    "Set which direction the ghost faces",
                    rmcp::handler::server::tool::schema_for_type::<SetFacingArgs>(),
                ),
                Tool::new(
                    "list_ghosts",
                    "List available ghosts (built-in and custom)",
                    rmcp::handler::server::tool::schema_for_type::<ListGhostsArgs>(),
                ),
            ]))
        }
    }

    fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> impl Future<Output = Result<CallToolResult, McpError>> + MaybeSendFuture + '_ {
        let default_ghost = self.default_ghost.clone();
        async move {
            let arguments = serde_json::Value::Object(request.arguments.unwrap_or_default());
            match request.name.as_ref() {
                "summon" => {
                    let args: SummonArgs = serde_json::from_value(arguments).map_err(|e| {
                        McpError::invalid_params(format!("Invalid arguments: {}", e), None)
                    })?;
                    send_command(
                        &args.name,
                        Command::SwitchGhost {
                            name: args.name.clone(),
                        },
                    )
                    .await
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                    Ok(CallToolResult::success(vec![Content::text("Summoning...")]))
                }
                "dismiss" => {
                    let args: DismissArgs = serde_json::from_value(arguments).map_err(|e| {
                        McpError::invalid_params(format!("Invalid arguments: {}", e), None)
                    })?;
                    send_command(&args.name, Command::Close)
                        .await
                        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                    Ok(CallToolResult::success(vec![Content::text(
                        "Dismissing...",
                    )]))
                }
                "speak" => {
                    let args: SpeakArgs = serde_json::from_value(arguments).map_err(|e| {
                        McpError::invalid_params(format!("Invalid arguments: {}", e), None)
                    })?;
                    let ghost = args.ghost.as_deref().unwrap_or(&default_ghost);
                    send_command(
                        ghost,
                        Command::Speak {
                            text: args.text,
                            voice: args.voice,
                        },
                    )
                    .await
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                    Ok(CallToolResult::success(vec![Content::text("Speaking...")]))
                }
                "play_animation" => {
                    let args: PlayAnimationArgs =
                        serde_json::from_value(arguments).map_err(|e| {
                            McpError::invalid_params(format!("Invalid arguments: {}", e), None)
                        })?;
                    let ghost = args.ghost.as_deref().unwrap_or(&default_ghost);
                    send_command(
                        ghost,
                        Command::PlayAnimation {
                            name: args.name,
                            loop_anim: args.loop_anim,
                        },
                    )
                    .await
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                    Ok(CallToolResult::success(vec![Content::text("Animating...")]))
                }
                "set_facing" => {
                    let args: SetFacingArgs = serde_json::from_value(arguments).map_err(|e| {
                        McpError::invalid_params(format!("Invalid arguments: {}", e), None)
                    })?;
                    let ghost = args.ghost.as_deref().unwrap_or(&default_ghost);
                    send_command(
                        ghost,
                        Command::SetFacing {
                            direction: args.direction,
                        },
                    )
                    .await
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                    Ok(CallToolResult::success(vec![Content::text("Turning...")]))
                }
                "list_ghosts" => {
                    let _args: ListGhostsArgs = serde_json::from_value(arguments).map_err(|e| {
                        McpError::invalid_params(format!("Invalid arguments: {}", e), None)
                    })?;
                    match crate::get_ghost_list() {
                        Ok(ghost_list) => {
                            let json = serde_json::to_string(&ghost_list)
                                .unwrap_or_else(|_| "{}".to_string());
                            Ok(CallToolResult::success(vec![Content::text(json)]))
                        }
                        Err(e) => Err(McpError::internal_error(e.to_string(), None)),
                    }
                }
                _ => Err(McpError::new(
                    ErrorCode(32601), // METHOD_NOT_FOUND
                    "Method not found",
                    None,
                )),
            }
        }
    }
}

async fn send_command(ghost_name: &str, cmd: Command) -> Result<String> {
    let (cmd_path, _) = get_socket_paths(DEFAULT_DAEMON_INSTANCE);
    match send_command_once(&cmd_path, ghost_name, cmd.clone()).await {
        Ok(()) => {}
        Err(first_error) => {
            ensure_daemon_running().await?;
            send_command_once(&cmd_path, ghost_name, cmd)
                .await
                .with_context(|| format!("Failed after daemon recovery attempt: {}", first_error))?;
        }
    }

    Ok("Success".to_string())
}

async fn send_command_once(cmd_path: &Path, ghost_name: &str, cmd: Command) -> Result<()> {
    let stream = UnixStream::connect(cmd_path)
        .await
        .with_context(|| format!("Could not connect to Medium daemon at {:?}", cmd_path))?;

    let mut framed = Framed::new(stream, LinesCodec::new());
    let line = serde_json::to_string(&RoutedCommand {
        ghost: ghost_name.to_string(),
        command: cmd,
    })?;
    framed.send(line).await?;
    Ok(())
}

async fn daemon_socket_is_responsive(cmd_path: &Path) -> bool {
    if !cmd_path.exists() {
        return false;
    }

    let stream = match UnixStream::connect(cmd_path).await {
        Ok(stream) => stream,
        Err(_) => return false,
    };

    let mut framed = Framed::new(stream, LinesCodec::new());
    let status = match serde_json::to_string(&RoutedCommand {
        ghost: "default".to_string(),
        command: Command::Status,
    }) {
        Ok(value) => value,
        Err(_) => return false,
    };

    if framed.send(status).await.is_err() {
        return false;
    }

    matches!(
        tokio::time::timeout(Duration::from_millis(800), framed.next()).await,
        Ok(Some(Ok(_)))
    )
}

async fn ensure_daemon_running() -> Result<()> {
    let (cmd_path, _) = get_socket_paths(DEFAULT_DAEMON_INSTANCE);
    if daemon_socket_is_responsive(&cmd_path).await {
        return Ok(());
    }

    if cmd_path.exists() {
        let _ = fs::remove_file(&cmd_path);
    }

    let exe_path = std::env::current_exe()?;
    let log_path = crate::config::log_file_path()?;
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

    for _ in 0..10 {
        if daemon_socket_is_responsive(&cmd_path).await {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }

    anyhow::bail!("Medium daemon failed to become responsive within timeout.")
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;
    use futures_util::SinkExt;
    use rmcp::model::{
        ClientCapabilities, JsonRpcNotification, JsonRpcRequest, NumberOrString, Request,
    };
    use rmcp::service::RxJsonRpcMessage;
    use serde_json::json;
    use std::sync::{Mutex, OnceLock};
    use tokio::net::UnixListener;

    fn test_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    #[tokio::test]
    async fn test_summon_tool_to_ipc() -> Result<()> {
        let _guard = test_lock().lock().unwrap();
        let (cmd_path, _) = get_socket_paths(DEFAULT_DAEMON_INSTANCE);
        // Remove existing socket if any from previous runs
        if cmd_path.exists() {
            let _ = std::fs::remove_file(&cmd_path);
        }
        let listener = UnixListener::bind(&cmd_path)?;

        let server = MediumMcpServer::new("vita".to_string());

        let (mut tx, rx) = futures::channel::mpsc::channel(10);
        let (client_tx, mut client_rx) = futures::channel::mpsc::channel(10);

        // Pass tuple (Sink, Stream) as IntoTransport
        let transport = (client_tx, rx);

        tokio::spawn(async move {
            let _ = server.serve(transport).await.unwrap().waiting().await;
        });

        // 1. Send Initialize
        let init_params = rmcp::model::InitializeRequestParams::new(
            ClientCapabilities::default(),
            Implementation::new("test", "0.1.0"),
        );
        let init_req = RxJsonRpcMessage::<RoleServer>::Request(JsonRpcRequest::new(
            NumberOrString::Number(1),
            rmcp::model::ClientRequest::InitializeRequest(Request::new(init_params)),
        ));
        tx.send(init_req).await.unwrap();

        // Wait for init response
        let _ = tokio::time::timeout(tokio::time::Duration::from_secs(1), client_rx.next()).await?;

        // 2. Send Initialized notification
        let initialized_notif = RxJsonRpcMessage::<RoleServer>::Notification(JsonRpcNotification {
            jsonrpc: Default::default(),
            notification: rmcp::model::ClientNotification::InitializedNotification(
                Default::default(),
            ),
        });
        tx.send(initialized_notif).await.unwrap();

        // 3. Send Tool Call
        let mut call_params = rmcp::model::CallToolRequestParams::new("summon");
        let mut args_map = serde_json::Map::new();
        args_map.insert("name".to_string(), json!("warrior"));
        call_params.arguments = Some(args_map);

        let request = RxJsonRpcMessage::<RoleServer>::Request(JsonRpcRequest::new(
            NumberOrString::Number(2),
            rmcp::model::ClientRequest::CallToolRequest(Request::new(call_params)),
        ));
        tx.send(request).await.unwrap();

        // Verify IPC command
        let (stream, _) =
            tokio::time::timeout(tokio::time::Duration::from_secs(2), listener.accept()).await??;
        let mut framed = Framed::new(stream, LinesCodec::new());
        let line = framed.next().await.context("No data on IPC stream")??;

        let cmd: RoutedCommand = serde_json::from_str(&line)?;
        assert_eq!(cmd.ghost, "warrior");
        match cmd.command {
            Command::SwitchGhost { name } => assert_eq!(name, "warrior"),
            _ => panic!("Expected SwitchGhost command"),
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_speak_tool_forwards_voice_flag_to_ipc() -> Result<()> {
        let _guard = test_lock().lock().unwrap();
        let (cmd_path, _) = get_socket_paths(DEFAULT_DAEMON_INSTANCE);
        if cmd_path.exists() {
            let _ = std::fs::remove_file(&cmd_path);
        }
        let listener = UnixListener::bind(&cmd_path)?;

        let server = MediumMcpServer::new("vita".to_string());

        let (mut tx, rx) = futures::channel::mpsc::channel(10);
        let (client_tx, mut client_rx) = futures::channel::mpsc::channel(10);
        let transport = (client_tx, rx);

        tokio::spawn(async move {
            let _ = server.serve(transport).await.unwrap().waiting().await;
        });

        let init_params = rmcp::model::InitializeRequestParams::new(
            ClientCapabilities::default(),
            Implementation::new("test", "0.1.0"),
        );
        let init_req = RxJsonRpcMessage::<RoleServer>::Request(JsonRpcRequest::new(
            NumberOrString::Number(1),
            rmcp::model::ClientRequest::InitializeRequest(Request::new(init_params)),
        ));
        tx.send(init_req).await.unwrap();
        let _ = tokio::time::timeout(tokio::time::Duration::from_secs(1), client_rx.next()).await?;

        let initialized_notif = RxJsonRpcMessage::<RoleServer>::Notification(JsonRpcNotification {
            jsonrpc: Default::default(),
            notification: rmcp::model::ClientNotification::InitializedNotification(
                Default::default(),
            ),
        });
        tx.send(initialized_notif).await.unwrap();

        let mut call_params = rmcp::model::CallToolRequestParams::new("speak");
        let mut args_map = serde_json::Map::new();
        args_map.insert("text".to_string(), json!("Quiet bubble only"));
        args_map.insert("ghost".to_string(), json!("archer"));
        args_map.insert("voice".to_string(), json!(false));
        call_params.arguments = Some(args_map);

        let request = RxJsonRpcMessage::<RoleServer>::Request(JsonRpcRequest::new(
            NumberOrString::Number(2),
            rmcp::model::ClientRequest::CallToolRequest(Request::new(call_params)),
        ));
        tx.send(request).await.unwrap();

        let (stream, _) =
            tokio::time::timeout(tokio::time::Duration::from_secs(2), listener.accept()).await??;
        let mut framed = Framed::new(stream, LinesCodec::new());
        let line = framed.next().await.context("No data on IPC stream")??;

        let cmd: RoutedCommand = serde_json::from_str(&line)?;
        assert_eq!(cmd.ghost, "archer");
        match cmd.command {
            Command::Speak {
                text,
                voice,
            } => {
                assert_eq!(text, "Quiet bubble only");
                assert_eq!(voice, Some(false));
            }
            _ => panic!("Expected Speak command"),
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_daemon_socket_is_responsive_false_when_socket_missing() -> Result<()> {
        let _guard = test_lock().lock().unwrap();
        let (cmd_path, _) = get_socket_paths(DEFAULT_DAEMON_INSTANCE);
        if cmd_path.exists() {
            let _ = std::fs::remove_file(&cmd_path);
        }

        assert!(!daemon_socket_is_responsive(&cmd_path).await);
        Ok(())
    }

    #[tokio::test]
    async fn test_daemon_socket_is_responsive_true_for_status_reply() -> Result<()> {
        let _guard = test_lock().lock().unwrap();
        let (cmd_path, _) = get_socket_paths(DEFAULT_DAEMON_INSTANCE);
        if cmd_path.exists() {
            let _ = std::fs::remove_file(&cmd_path);
        }
        let listener = UnixListener::bind(&cmd_path)?;

        tokio::spawn(async move {
            if let Ok((stream, _)) = listener.accept().await {
                let mut framed = Framed::new(stream, LinesCodec::new());
                if let Some(Ok(_line)) = framed.next().await {
                    let _ = framed
                        .send(
                            serde_json::json!({
                                "type": "status",
                                "active_ghost": "vita",
                                "known_ghosts": ["vita"]
                            })
                            .to_string(),
                        )
                        .await;
                }
            }
        });

        assert!(daemon_socket_is_responsive(&cmd_path).await);
        Ok(())
    }
}
