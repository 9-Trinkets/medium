# Core commands

## Daemon and initialization

```bash
medium daemon              # Start the daemon
medium init               # Initialize Medium (creates config, writes MCP entry)
medium status             # Check daemon status
medium logs --lines 100   # Show recent logs
medium logs --follow      # Stream logs in real time
medium logs --filter TEXT # Filter logs by text
medium doctor             # Run diagnostics
```

## Configuration

```bash
medium config path                                    # Show config file location
medium config get                                     # Show all config
medium config get integration.default_ghost           # Get a specific value
medium config set integration.default_ghost vita      # Set a value
```

## Integration

```bash
medium integrate claude                # Local Claude MCP entry
medium integrate claude --global        # Global Claude MCP entry
medium integrate copilot                # Copilot workspace MCP entry
medium integrate copilot --global       # Copilot global MCP entry
```

## Ghost management

```bash
medium ghosts list                          # List installed ghosts
medium ghosts validate ./path/to/ghost      # Validate a ghost directory
medium ghosts preview ./path/to/ghost       # Preview a ghost
medium ghosts import aseprite ./pack \      # Import from exported sheet
  --name imported-ghost \
  --sheet exported.png
```

See [Importing ghosts](importing-ghosts.md) for detailed import examples.

## MCP usage

```bash
medium mcp --ghost vita    # Start the MCP bridge for the vita ghost
```

The MCP bridge allows agents and Claude Code to control ghosts via the `control-medium` skill.
