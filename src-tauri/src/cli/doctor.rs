use anyhow::{Context, Result};
use serde_json::Value;
use std::fs;
use std::path::Path;
use tauri_app_lib::config::{
    configured_default_ghost, find_nearest_project_copilot_mcp_path, find_nearest_project_mcp_path,
    global_claude_mcp_path, global_copilot_mcp_path, load_global_config, log_file_path,
    resolve_config_path,
};
use tauri_app_lib::ipc::{get_socket_paths, DEFAULT_DAEMON_INSTANCE};
use tokio::net::UnixStream;

pub async fn run() -> Result<()> {
    println!("Medium Doctor\n");

    let resolved_config = resolve_config_path()?;
    let log_path = log_file_path()?;
    let cwd = std::env::current_dir().context("Could not resolve current working directory")?;
    let global_mcp_path = global_claude_mcp_path()?;
    let project_mcp_path = find_nearest_project_mcp_path(&cwd);
    let global_copilot_mcp_path = global_copilot_mcp_path()?;
    let project_copilot_mcp_path = find_nearest_project_copilot_mcp_path(&cwd);
    let (cmd_socket, _) = get_socket_paths(DEFAULT_DAEMON_INSTANCE);

    let mut issues_found = false;

    println!("Paths");
    println!(
        "{} Config: {:?} ({})",
        if resolved_config.path.exists() {
            "✅"
        } else {
            "⚠️"
        },
        resolved_config.path,
        resolved_config.source
    );
    println!("ℹ️  Logs:   {:?}", log_path);
    println!("ℹ️  Socket: {:?}", cmd_socket);

    println!("\nConfiguration");
    match load_global_config() {
        Ok(Some(config)) => {
            println!("✅ Config file parsed successfully.");
            let tts = config.tts.unwrap_or_default();
            let has_openai = tts.has_openai_api_key();
            let has_elevenlabs = tts.has_elevenlabs_api_key();
            print_key_status("OpenAI API key", has_openai);
            print_key_status("ElevenLabs API key", has_elevenlabs);
            match tts.provider_name() {
                Some(provider) => {
                    println!("ℹ️  TTS provider: {}", provider);
                    if provider == "openai" && !has_openai {
                        issues_found = true;
                    }
                    if provider == "elevenlabs" && !has_elevenlabs {
                        issues_found = true;
                    }
                }
                None => {
                    println!("⚠️  No default TTS provider configured.");
                    issues_found = true;
                }
            }
            println!("ℹ️  Default ghost: {}", configured_default_ghost()?);
        }
        Ok(None) => {
            println!("⚠️  Config file is missing.");
            issues_found = true;
        }
        Err(err) => {
            println!("❌ Config file could not be parsed: {}", err);
            issues_found = true;
        }
    }

    println!("\nMCP Integration");
    match inspect_mcp_config(&global_mcp_path)? {
        McpStatus::Configured => println!(
            "✅ Claude MCP config includes a medium server: {:?}",
            global_mcp_path
        ),
        McpStatus::Missing => println!("⚠️  Claude MCP config not found: {:?}", global_mcp_path),
        McpStatus::NoMediumServer => {
            println!(
                "⚠️  Claude MCP config exists but has no medium server: {:?}",
                global_mcp_path
            );
            issues_found = true;
        }
        McpStatus::Invalid(reason) => {
            println!(
                "❌ Claude MCP config is invalid at {:?}: {}",
                global_mcp_path, reason
            );
            issues_found = true;
        }
    }

    if let Some(project_mcp_path) = project_mcp_path {
        match inspect_mcp_config(&project_mcp_path)? {
            McpStatus::Configured => println!(
                "✅ Project MCP config includes a medium server: {:?}",
                project_mcp_path
            ),
            McpStatus::Missing => {}
            McpStatus::NoMediumServer => println!(
                "⚠️  Project MCP config exists but has no medium server: {:?}",
                project_mcp_path
            ),
            McpStatus::Invalid(reason) => println!(
                "❌ Project MCP config is invalid at {:?}: {}",
                project_mcp_path, reason
            ),
        }
    } else {
        println!(
            "ℹ️  No project-local .mcp.json found from {:?} upward.",
            cwd
        );
    }

    match inspect_mcp_config(&global_copilot_mcp_path)? {
        McpStatus::Configured => println!(
            "✅ Copilot global MCP config includes a medium server: {:?}",
            global_copilot_mcp_path
        ),
        McpStatus::Missing => println!(
            "ℹ️  Copilot global MCP config not found: {:?}",
            global_copilot_mcp_path
        ),
        McpStatus::NoMediumServer => println!(
            "⚠️  Copilot global MCP config exists but has no medium server: {:?}",
            global_copilot_mcp_path
        ),
        McpStatus::Invalid(reason) => {
            println!(
                "❌ Copilot global MCP config is invalid at {:?}: {}",
                global_copilot_mcp_path, reason
            );
            issues_found = true;
        }
    }

    if let Some(project_copilot_mcp_path) = project_copilot_mcp_path {
        match inspect_mcp_config(&project_copilot_mcp_path)? {
            McpStatus::Configured => println!(
                "✅ Copilot workspace MCP config includes a medium server: {:?}",
                project_copilot_mcp_path
            ),
            McpStatus::Missing => {}
            McpStatus::NoMediumServer => println!(
                "⚠️  Copilot workspace MCP config exists but has no medium server: {:?}",
                project_copilot_mcp_path
            ),
            McpStatus::Invalid(reason) => println!(
                "❌ Copilot workspace MCP config is invalid at {:?}: {}",
                project_copilot_mcp_path, reason
            ),
        }
    } else {
        println!(
            "ℹ️  No project-local .vscode/mcp.json found from {:?} upward.",
            cwd
        );
    }

    println!("\nDaemon");
    if !cmd_socket.exists() {
        println!("⚠️  Daemon socket is missing. The daemon appears to be stopped.");
    } else {
        match UnixStream::connect(&cmd_socket).await {
            Ok(_) => println!("✅ Daemon socket is present and responsive."),
            Err(err) => {
                println!("❌ Daemon socket exists but is unresponsive: {}", err);
                issues_found = true;
            }
        }
    }

    println!("\nSummary");
    if issues_found {
        println!("Doctor found issues. Review the warnings above before relying on this setup.");
    } else {
        println!("No blocking issues found.");
    }

    Ok(())
}

fn print_key_status(label: &str, configured: bool) {
    if configured {
        println!("✅ {} configured.", label);
    } else {
        println!("⚠️  {} missing or empty.", label);
    }
}

enum McpStatus {
    Configured,
    Missing,
    NoMediumServer,
    Invalid(String),
}

fn inspect_mcp_config(path: &Path) -> Result<McpStatus> {
    if !path.exists() {
        return Ok(McpStatus::Missing);
    }

    let content = fs::read_to_string(path).with_context(|| format!("Failed to read {:?}", path))?;
    let parsed = match serde_json::from_str::<Value>(&content) {
        Ok(value) => value,
        Err(err) => return Ok(McpStatus::Invalid(err.to_string())),
    };

    let has_medium = parsed
        .get("mcpServers")
        .or_else(|| parsed.get("servers"))
        .and_then(Value::as_object)
        .is_some_and(|servers| servers.contains_key("medium"));

    if has_medium {
        Ok(McpStatus::Configured)
    } else {
        Ok(McpStatus::NoMediumServer)
    }
}

#[cfg(test)]
mod tests {
    use super::{inspect_mcp_config, McpStatus};
    use anyhow::Result;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn reports_medium_server_in_valid_mcp_config() -> Result<()> {
        let tmp = tempdir()?;
        let path = tmp.path().join(".mcp.json");
        fs::write(&path, r#"{"mcpServers":{"medium":{"command":"medium"}}}"#)?;

        assert!(matches!(inspect_mcp_config(&path)?, McpStatus::Configured));
        Ok(())
    }

    #[test]
    fn reports_invalid_json() -> Result<()> {
        let tmp = tempdir()?;
        let path = tmp.path().join(".mcp.json");
        fs::write(&path, "{not-json")?;

        assert!(matches!(inspect_mcp_config(&path)?, McpStatus::Invalid(_)));
        Ok(())
    }
}
