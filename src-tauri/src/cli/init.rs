use anyhow::{Context, Result};
use serde_json::{json, Map, Value};
use std::fs;
use tauri_app_lib::config::{global_claude_mcp_path, medium_dir};

pub fn run() -> Result<()> {
    let medium_dir = medium_dir()?;
    if !medium_dir.exists() {
        fs::create_dir_all(&medium_dir)?;
    }

    let claude_mcp_path = global_claude_mcp_path()?;
    if let Some(parent) = claude_mcp_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mcp_config = if claude_mcp_path.exists() {
        let existing = fs::read_to_string(&claude_mcp_path)
            .with_context(|| format!("Failed to read {:?}", claude_mcp_path))?;
        serde_json::from_str::<Value>(&existing)
            .with_context(|| format!("Failed to parse {:?}", claude_mcp_path))?
    } else {
        json!({})
    };

    let merged = merge_medium_server(mcp_config);
    fs::write(&claude_mcp_path, serde_json::to_string_pretty(&merged)?)?;
    println!("✅ Medium MCP configured in {:?}", claude_mcp_path);
    println!(
        "\nConfiguration complete. Run 'medium daemon' in a terminal to start the avatar daemon."
    );

    Ok(())
}

fn merge_medium_server(mut config: Value) -> Value {
    if !config.is_object() {
        config = json!({});
    }

    let object = config.as_object_mut().expect("config object");
    let mcp_servers = object
        .entry("mcpServers")
        .or_insert_with(|| Value::Object(Map::new()));

    if !mcp_servers.is_object() {
        *mcp_servers = Value::Object(Map::new());
    }

    mcp_servers
        .as_object_mut()
        .expect("mcpServers object")
        .insert(
            "medium".to_string(),
            json!({
                "command": "medium",
                "args": ["mcp", "--ghost", "vita"],
                "transport": "stdio"
            }),
        );

    config
}

#[cfg(test)]
mod tests {
    use super::merge_medium_server;
    use serde_json::json;

    #[test]
    fn preserves_existing_servers_when_adding_medium() {
        let input = json!({
            "mcpServers": {
                "other": {
                    "command": "other"
                }
            }
        });

        let merged = merge_medium_server(input);
        let servers = merged["mcpServers"].as_object().unwrap();
        assert!(servers.contains_key("other"));
        assert!(servers.contains_key("medium"));
    }
}
