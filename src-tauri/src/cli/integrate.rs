use anyhow::{Context, Result};
use clap::ValueEnum;
use serde_json::{json, Map, Value};
use std::fs;
use std::path::{Path, PathBuf};
use tauri_app_lib::config::{
    configured_default_ghost, global_claude_mcp_path, global_copilot_mcp_path,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum IntegrationTool {
    Claude,
    Copilot,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IntegrationScope {
    Project,
    Global,
}

pub fn run(
    tool: IntegrationTool,
    scope: IntegrationScope,
    ghost: Option<&str>,
    cwd: &Path,
) -> Result<PathBuf> {
    let ghost = ghost
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or(configured_default_ghost()?);
    let target_path = target_path(tool, scope, cwd)?;
    write_integration(tool, &target_path, &ghost)?;
    Ok(target_path)
}

pub fn target_path(tool: IntegrationTool, scope: IntegrationScope, cwd: &Path) -> Result<PathBuf> {
    match (tool, scope) {
        (IntegrationTool::Claude, IntegrationScope::Project) => Ok(cwd.join(".mcp.json")),
        (IntegrationTool::Claude, IntegrationScope::Global) => global_claude_mcp_path(),
        (IntegrationTool::Copilot, IntegrationScope::Project) => {
            Ok(cwd.join(".vscode").join("mcp.json"))
        }
        (IntegrationTool::Copilot, IntegrationScope::Global) => global_copilot_mcp_path(),
    }
}

pub fn write_integration(tool: IntegrationTool, target_path: &Path, ghost: &str) -> Result<()> {
    if let Some(parent) = target_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create {}", parent.display()))?;
    }

    let existing = if target_path.exists() {
        let raw = fs::read_to_string(target_path)
            .with_context(|| format!("Failed to read {}", target_path.display()))?;
        serde_json::from_str::<Value>(&raw)
            .with_context(|| format!("Failed to parse {}", target_path.display()))?
    } else {
        json!({})
    };

    let merged = merge_integration(tool, existing, ghost);
    fs::write(target_path, serde_json::to_string_pretty(&merged)?)
        .with_context(|| format!("Failed to write {}", target_path.display()))?;
    Ok(())
}

fn merge_integration(tool: IntegrationTool, mut config: Value, ghost: &str) -> Value {
    if !config.is_object() {
        config = json!({});
    }

    let object = config.as_object_mut().expect("config object");
    let servers_key = match tool {
        IntegrationTool::Claude => "mcpServers",
        IntegrationTool::Copilot => {
            if object.contains_key("mcpServers") {
                "mcpServers"
            } else {
                "servers"
            }
        }
    };

    let servers = object
        .entry(servers_key.to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    if !servers.is_object() {
        *servers = Value::Object(Map::new());
    }

    let entry = match (tool, servers_key) {
        (IntegrationTool::Claude, _) => json!({
            "command": "medium",
            "args": ["mcp", "--ghost", ghost],
            "transport": "stdio"
        }),
        (IntegrationTool::Copilot, "mcpServers") => json!({
            "type": "stdio",
            "command": "medium",
            "args": ["mcp", "--ghost", ghost],
            "env": {},
            "tools": ["*"]
        }),
        (IntegrationTool::Copilot, _) => json!({
            "type": "stdio",
            "command": "medium",
            "args": ["mcp", "--ghost", ghost]
        }),
    };

    servers
        .as_object_mut()
        .expect("servers object")
        .insert("medium".to_string(), entry);

    config
}

#[cfg(test)]
mod tests {
    use super::{merge_integration, IntegrationTool};
    use serde_json::json;

    #[test]
    fn merges_medium_into_claude_mcp_servers() {
        let value = merge_integration(
            IntegrationTool::Claude,
            json!({"mcpServers":{"other":{"command":"other"}}}),
            "vita",
        );

        assert!(value["mcpServers"]["other"].is_object());
        assert_eq!(value["mcpServers"]["medium"]["args"][2], "vita");
    }

    #[test]
    fn uses_workspace_servers_shape_for_copilot_when_missing() {
        let value = merge_integration(IntegrationTool::Copilot, json!({}), "vita");
        assert!(value["servers"]["medium"].is_object());
        assert_eq!(value["servers"]["medium"]["type"], "stdio");
    }

    #[test]
    fn preserves_existing_global_copilot_shape() {
        let value = merge_integration(
            IntegrationTool::Copilot,
            json!({"mcpServers":{"other":{"type":"stdio"}}}),
            "vita",
        );
        assert!(value["mcpServers"]["other"].is_object());
        assert_eq!(value["mcpServers"]["medium"]["tools"][0], "*");
    }
}
