# Configuration and integration

## Quick start

Use `medium init` to set up Medium for the first time. It ensures `~/.medium/config.toml` exists and writes a project-local Claude MCP entry in `.mcp.json`.

```bash
medium init
```

## Config locations

Medium uses these standard paths:

- **Config file:** `~/.medium/config.toml`
- **Daemon log:** `~/.medium/daemon.log`
- **IPC socket:** `/tmp/medium_ghost_default_cmd.sock`
- **Local ghost library:** `~/.medium/ghosts`
- **Claude MCP config:** `~/.claude/.mcp.json` (project-local) or global
- **Copilot workspace MCP:** `.vscode/mcp.json`
- **Copilot global MCP:** `~/.copilot/mcp-config.json`

## Integration options

Use `medium integrate` to add Medium to your tools:

```bash
# Local Claude project (current directory)
medium integrate claude

# Global Claude configuration
medium integrate claude --global

# Copilot workspace (current directory)
medium integrate copilot

# Copilot global configuration
medium integrate copilot --global
```

## Inspecting and modifying config

```bash
# Show where the config file is
medium config path

# View all configuration
medium config get

# Get a specific value
medium config get integration.default_ghost

# Set a value
medium config set integration.default_ghost vita
```

## Template files

Reusable configuration templates are provided in `templates/`:

- `medium.config.toml` — example Medium configuration
- `claude-project.mcp.json` — Claude project-local MCP entry
- `copilot-workspace.mcp.json` — VS Code workspace MCP configuration
- `copilot-global.mcp-config.json` — Copilot global MCP configuration

## Default ghost

The bundled default ghost is **`vita`**, imported from the [Arks Digital dino pack](https://arks.digital/) by @ArksDigital.
